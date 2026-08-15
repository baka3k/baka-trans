"""Long-lived, offline-only NDJSON HY-MT server."""

from __future__ import annotations

import argparse
import os
import queue
import socket
import sys
import threading
from pathlib import Path
from typing import Any, BinaryIO, TextIO

from .constants import DEFAULT_TRANSLATE_TIMEOUT_SECONDS, MODEL_ID, MODEL_REVISION, PROTOCOL_VERSION, RUNTIME_IDENTITY, RUNTIME_VERSION, TRUST_REMOTE_CODE
from .lifecycle import LifecycleError, active_path, install, status, validate_model
from .protocol import ProtocolError, emit, parse_line, validate_cancel, validate_translate


def hardened_environment() -> None:
    os.environ.update({
        "HF_HUB_OFFLINE": "1",
        "TRANSFORMERS_OFFLINE": "1",
        "HF_HUB_DISABLE_TELEMETRY": "1",
        "PYTORCH_ENABLE_MPS_FALLBACK": "0",
        "TOKENIZERS_PARALLELISM": "false",
    })
    for name in ("HF_TOKEN", "HUGGING_FACE_HUB_TOKEN", "HUGGINGFACE_HUB_TOKEN"):
        if os.environ.get(name):
            raise LifecycleError("Hub credentials are not accepted in serve mode.")

    def blocked(*args: Any, **kwargs: Any) -> None:
        del args, kwargs
        raise RuntimeError("Network access is disabled in HY-MT serve mode.")

    # `local_files_only` is the primary loading control. These process-local
    # guards make an accidental socket path fail closed as well; OS egress
    # policy remains an additional release/launcher control.
    socket.create_connection = blocked
    socket.socket.connect = blocked
    socket.socket.connect_ex = blocked
    socket.socket.sendto = blocked


class ServeLoop:
    def __init__(self, runner: HyMtRunner, output: TextIO) -> None:
        self.runner = runner
        self.output = output
        self.messages: queue.Queue[dict[str, Any] | None] = queue.Queue()
        self.active_id: str | None = None
        self.cancel_event = threading.Event()
        self._output_lock = threading.Lock()

    def _emit(self, payload: dict[str, Any]) -> None:
        with self._output_lock:
            emit(self.output, payload)

    def ready(self) -> None:
        metadata = self.runner.metadata()
        self._emit({
            "type": "ready",
            "protocolVersion": PROTOCOL_VERSION,
            "runtimeVersion": RUNTIME_VERSION,
            "modelId": MODEL_ID,
            "revision": MODEL_REVISION,
            "trustRemoteCode": TRUST_REMOTE_CODE,
            "device": metadata["actualDevice"],
            "dtype": metadata["actualDtype"],
            "pid": os.getpid(),
            "loadMs": metadata["modelLoadMs"],
        })

    def receive(self, stream: BinaryIO) -> None:
        for raw in stream:
            try:
                message = parse_line(raw)
                if message.get("type") == "cancel":
                    request_id = validate_cancel(message)
                    if request_id == self.active_id:
                        # The reader is deliberately independent of the one
                        # generation worker so a cancel reaches Transformers
                        # while generate() is still running.
                        self.cancel_event.set()
                        continue
                self.messages.put(message)
            except ProtocolError as exc:
                self._emit(exc.payload())
        self.messages.put(None)

    def _handle(self, message: dict[str, Any]) -> None:
        message_type = message.get("type")
        if message_type == "cancel":
            request_id = validate_cancel(message)
            if request_id == self.active_id:
                self.cancel_event.set()
            else:
                self._emit({"type": "cancelled", "id": request_id, "active": False})
            return
        request = validate_translate(message)
        request_id = request["id"]
        if self.active_id is not None:
            raise ProtocolError("busy", "A translation request is already active.", retryable=True)
        self.active_id = request_id
        self.cancel_event.clear()
        try:
            result = self.runner.translate(
                request["text"],
                generation_mode="greedy",
                max_new_tokens=request["maxNewTokens"],
                timeout_seconds=DEFAULT_TRANSLATE_TIMEOUT_SECONDS,
                cancellation=self.cancel_event,
            )
            if result.cancelled:
                self._emit({"type": "cancelled", "id": request_id, "active": True})
            elif not result.text.strip():
                raise ProtocolError("invalid_output", "Model returned an empty translation.", retryable=True)
            else:
                self._emit({"type": "result", "id": request_id, "text": result.text, "inputTokens": result.input_tokens, "outputTokens": result.output_tokens, "latencyMs": result.latency_ms})
        finally:
            self.active_id = None
            self.cancel_event.clear()

    def run(self) -> None:
        while True:
            message = self.messages.get()
            if message is None:
                return
            try:
                self._handle(message)
            except ProtocolError as exc:
                request_id = message.get("id") if isinstance(message.get("id"), str) else None
                self._emit(exc.payload(request_id))
            except Exception:
                request_id = message.get("id") if isinstance(message.get("id"), str) else None
                self._emit(ProtocolError("inference_failed", "Translation could not be completed.", retryable=True).payload(request_id))
                self.active_id = None
                self.cancel_event.clear()


def run_serve(model_root: Path, device: str, stdin: BinaryIO, stdout: TextIO) -> int:
    hardened_environment()
    # Imported only after hardening: importing the runner pins offline mode in
    # the environment before torch/transformers and the Hub client load, and
    # the install command must never import it for exactly that reason.
    from .runner import HyMtRunner

    model_dir = active_path(model_root)
    validate_model(model_dir)
    runner = HyMtRunner(model_dir, requested_device=device)
    loop = ServeLoop(runner, stdout)
    reader = threading.Thread(target=loop.receive, args=(stdin,), name="hy-mt-stdin", daemon=True)
    reader.start()
    loop.ready()
    loop.run()
    return 0


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Managed HY-MT translation sidecar")
    commands = result.add_subparsers(dest="command", required=True)
    for name in ("install", "check"):
        command = commands.add_parser(name)
        command.add_argument("--model-root", type=Path, required=True)
    serve = commands.add_parser("serve")
    serve.add_argument("--model-root", type=Path, required=True)
    serve.add_argument("--device", choices=("mps", "cpu"), default="mps")
    return result


def main() -> None:
    args = parser().parse_args()
    try:
        if args.command == "install":
            # Install is the only online lifecycle step. The Hub client freezes
            # HF_HUB_OFFLINE at import time, so drop offline flags (including
            # any inherited from the environment) before it is imported.
            for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE"):
                os.environ.pop(name, None)
            def progress(payload: dict[str, Any]) -> None:
                emit(sys.stdout, payload)
            emit(sys.stdout, {"type": "status", "state": "downloading"})
            install(args.model_root, progress)
            return
        if args.command == "check":
            emit(sys.stdout, {"type": "status", **status(args.model_root)})
            return
        raise SystemExit(run_serve(args.model_root, args.device, sys.stdin.buffer, sys.stdout))
    except (LifecycleError, OSError):
        # Lifecycle errors are deliberately fixed text; never expose a path,
        # token, exception trace, or user meeting text to the protocol.
        emit(sys.stdout, {"type": "error", "code": "lifecycle_failed", "message": "Model lifecycle operation failed.", "retryable": False})
        raise SystemExit(2)


if __name__ == "__main__":
    main()
