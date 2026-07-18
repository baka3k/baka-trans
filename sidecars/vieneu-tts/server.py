"""Managed, loopback-only VieNeu-TTS runtime for baka-trans."""

from __future__ import annotations

import argparse
import hashlib
import hmac
import io
import json
import logging
import os
import sys
import threading
import time
import wave
from dataclasses import asdict, dataclass
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

import numpy as np


LOGGER = logging.getLogger("baka-trans-vieneu")
MAX_REQUEST_BYTES = 64 * 1024
MAX_TEXT_CHARS = 4_000
SAMPLE_RATE_HZ = 48_000
STYLES = {"tu_nhien", "tin_tuc", "doc_truyen"}
MODEL_VERSION = "v3-turbo-int8-2026-07"
BACKBONE_REPO = "pnnbao-ump/VieNeu-TTS-v3-Turbo"
BACKBONE_REVISION = "75ff82a72f54d55ed389e1eeb12041d3c4bac7d4"
CODEC_REPO = "OpenMOSS-Team/MOSS-Audio-Tokenizer-Nano-ONNX"
CODEC_REVISION = "ceff0d0749bfb3fa2d61149794ec6feef0d1e1ae"
INSTALL_MANIFEST = "install-manifest.json"


@dataclass(frozen=True)
class Artifact:
    group: str
    path: str
    size: int
    sha256: str


ARTIFACTS = (
    Artifact("backbone", "onnx_int8/config.json", 2_152, "a9f8d9c4b4736448ab355d1a98cfe48f5e39aecf2916c37b0806c228612e9a2d"),
    Artifact("backbone", "onnx_int8/tokenizer.json", 22_320, "6cc6bcbe380b8c37bd9f2514e37c5dfa3e00e122c6e3125dae5c4afe48e39158"),
    Artifact("backbone", "onnx_int8/vieneu_acoustic_cached.onnx", 7_207_223, "0be6575ffe1c4c2009edb9c9b218c235f09665f630d1840e63c74bef30d462c1"),
    Artifact("backbone", "onnx_int8/vieneu_backbone_shared.data", 103_891_968, "68c0bd5e75f9cf2d557040201f5465dc03a61206813845f2de1ebe6542652b92"),
    Artifact("backbone", "onnx_int8/vieneu_decode_step.onnx", 1_062_040, "7907f8e067de22ee88f0912ffc8ccaf7cf90025e1d41351d2a5bb7cec44fc859"),
    Artifact("backbone", "onnx_int8/vieneu_prefill.onnx", 1_090_823, "9d04bd8023c5a003dd60939848bba7e85c5d8448480e607a9ae7aa3ecd6d7494"),
    Artifact("backbone", "onnx_int8/vieneu_v3_heads.npz", 52_219_622, "c2eadeb5b0b85c3009270352adea8c05a72f31c5a9f189ead9184333fb1becb8"),
    Artifact("codec", "codec_browser_onnx_meta.json", 17_036, "3e291c883bb7d11ff2fe8e964e3e495519760358859f35c951254c7741592731"),
    Artifact("codec", "moss_audio_tokenizer_decode_full.onnx", 681_902, "0fbbafe3fd4afa2a019af5c5ced204af6e2d1db044fa40f021525d2aee95b4ac"),
    Artifact("codec", "moss_audio_tokenizer_decode_shared.data", 44_198_912, "e69d52e0f4e84ca27850557ee54face46632d3a5a16c89bd246c7c408466dcad"),
    Artifact("codec", "moss_audio_tokenizer_decode_step.onnx", 351_400, "9527c86a29e1837edec1f74db57d5eeaadb3a715af3382703566460afed25855"),
    Artifact("codec", "moss_audio_tokenizer_encode.data", 44_507_136, "aa751265b2bab2887eac224484546b194875aa7494b607115439b3dc6b228a2c"),
    Artifact("codec", "moss_audio_tokenizer_encode.onnx", 815_775, "eadea4a645abdcf98714c7aead122ee2ce7da6e080f9f80b977cd1ca8e19473a"),
)
TOTAL_MODEL_BYTES = sum(item.size for item in ARTIFACTS)


def emit_event(event: str, **payload: Any) -> None:
    print(json.dumps({"event": event, **payload}, ensure_ascii=False), flush=True)


def artifact_path(model_dir: Path, artifact: Artifact) -> Path:
    root = model_dir / artifact.group
    candidate = root / Path(artifact.path)
    resolved_root = root.resolve()
    resolved_parent = candidate.parent.resolve()
    if resolved_root != resolved_parent and resolved_root not in resolved_parent.parents:
        raise RuntimeError("Model artifact escaped its managed directory")
    return candidate


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def validate_model(model_dir: Path, *, emit_progress: bool) -> dict[str, Any]:
    manifest_path = model_dir / INSTALL_MANIFEST
    if not manifest_path.is_file() or manifest_path.is_symlink():
        raise RuntimeError("VieNeu model installation is incomplete")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if (
        manifest.get("modelVersion") != MODEL_VERSION
        or manifest.get("backboneRevision") != BACKBONE_REVISION
        or manifest.get("codecRevision") != CODEC_REVISION
    ):
        raise RuntimeError("VieNeu model manifest version is not supported")

    verified = 0
    for artifact in ARTIFACTS:
        path = artifact_path(model_dir, artifact)
        if not path.is_file() or path.is_symlink():
            raise RuntimeError(f"Missing managed model artifact: {artifact.path}")
        if path.stat().st_size != artifact.size:
            raise RuntimeError(f"Managed model artifact has the wrong size: {artifact.path}")
        if sha256_file(path) != artifact.sha256:
            raise RuntimeError(f"Managed model artifact failed integrity verification: {artifact.path}")
        verified += artifact.size
        if emit_progress:
            emit_event(
                "progress",
                phase="verifying",
                downloadedBytes=TOTAL_MODEL_BYTES,
                verifiedBytes=verified,
                totalBytes=TOTAL_MODEL_BYTES,
                percent=min(100, round(verified * 100 / TOTAL_MODEL_BYTES)),
                message="Verifying VieNeu model files…",
            )
    return manifest


class DownloadProgress:
    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._downloaded = 0
        self._last_percent = -1
        self._last_emit = 0.0

    def add(self, value: int) -> None:
        if value <= 0:
            return
        with self._lock:
            self._downloaded = min(TOTAL_MODEL_BYTES, self._downloaded + value)
            percent = min(100, round(self._downloaded * 100 / TOTAL_MODEL_BYTES))
            now = time.monotonic()
            if percent == self._last_percent and now - self._last_emit < 0.5:
                return
            self._last_percent = percent
            self._last_emit = now
            emit_event(
                "progress",
                phase="downloading",
                downloadedBytes=self._downloaded,
                verifiedBytes=0,
                totalBytes=TOTAL_MODEL_BYTES,
                percent=percent,
                message="Downloading VieNeu-TTS…",
            )


DOWNLOAD_PROGRESS: DownloadProgress | None = None


def progress_tqdm_class():
    from tqdm.auto import tqdm

    class ManagedTqdm(tqdm):
        def __init__(self, *args: Any, **kwargs: Any) -> None:
            initial = int(kwargs.get("initial") or 0)
            super().__init__(*args, **kwargs)
            if DOWNLOAD_PROGRESS is not None:
                DOWNLOAD_PROGRESS.add(initial)

        def update(self, value: int = 1) -> bool | None:
            result = super().update(value)
            if DOWNLOAD_PROGRESS is not None:
                DOWNLOAD_PROGRESS.add(int(value))
            return result

    return ManagedTqdm


def install_model(model_dir: Path) -> None:
    global DOWNLOAD_PROGRESS
    from huggingface_hub import snapshot_download

    model_dir.mkdir(parents=True, exist_ok=True)
    if model_dir.is_symlink():
        raise RuntimeError("Managed model directory cannot be a symbolic link")
    DOWNLOAD_PROGRESS = DownloadProgress()
    emit_event(
        "progress",
        phase="downloading",
        downloadedBytes=0,
        verifiedBytes=0,
        totalBytes=TOTAL_MODEL_BYTES,
        percent=0,
        message="Preparing VieNeu-TTS download…",
    )
    snapshot_download(
        BACKBONE_REPO,
        revision=BACKBONE_REVISION,
        local_dir=model_dir / "backbone",
        allow_patterns=[item.path for item in ARTIFACTS if item.group == "backbone"],
        max_workers=4,
        tqdm_class=progress_tqdm_class(),
    )
    snapshot_download(
        CODEC_REPO,
        revision=CODEC_REVISION,
        local_dir=model_dir / "codec",
        allow_patterns=[item.path for item in ARTIFACTS if item.group == "codec"],
        max_workers=4,
        tqdm_class=progress_tqdm_class(),
    )

    manifest = {
        "modelVersion": MODEL_VERSION,
        "backboneRepo": BACKBONE_REPO,
        "backboneRevision": BACKBONE_REVISION,
        "codecRepo": CODEC_REPO,
        "codecRevision": CODEC_REVISION,
        "totalBytes": TOTAL_MODEL_BYTES,
        "artifacts": [asdict(item) for item in ARTIFACTS],
    }
    temporary = model_dir / f"{INSTALL_MANIFEST}.tmp"
    temporary.write_text(json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, model_dir / INSTALL_MANIFEST)
    validate_model(model_dir, emit_progress=True)
    emit_event(
        "complete",
        phase="installed",
        downloadedBytes=TOTAL_MODEL_BYTES,
        verifiedBytes=TOTAL_MODEL_BYTES,
        totalBytes=TOTAL_MODEL_BYTES,
        percent=100,
        message="VieNeu-TTS is installed.",
    )


def managed_vieneu(model_dir: Path, threads: int):
    os.environ["HF_HUB_OFFLINE"] = "1"
    os.environ["TRANSFORMERS_OFFLINE"] = "1"
    os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
    from vieneu._v3_turbo_engine.onnx_runtime_lite import OnnxV3LiteEngine
    from vieneu.base import BaseVieneuTTS
    from vieneu.v3turbo import V3TurboVieNeuTTS

    runtime = V3TurboVieNeuTTS.__new__(V3TurboVieNeuTTS)
    BaseVieneuTTS.__init__(runtime)
    runtime.sample_rate = SAMPLE_RATE_HZ
    runtime.engine = OnnxV3LiteEngine(
        checkpoint_path=str(model_dir / "backbone"),
        onnx_dir=str(model_dir / "backbone" / "onnx_int8"),
        codec_dir=str(model_dir / "codec"),
        threads=max(1, threads),
    )
    runtime.backend = "onnx"
    runtime.default_style = "tu_nhien"
    runtime._preset_voices = {}
    runtime._default_voice = None
    runtime._load_v3_voices()
    runtime.max_batch_size = 1
    runtime._batch_engine = None
    return runtime


class VieNeuRuntime:
    def __init__(self, *, model_dir: Path, threads: int) -> None:
        validate_model(model_dir, emit_progress=False)
        LOGGER.info("Loading managed VieNeu-TTS v3 Turbo (onnx/int8)")
        self._tts = managed_vieneu(model_dir, threads)
        self._lock = threading.Lock()
        self.voices = [
            {"id": str(voice_id), "name": str(label), "language": "vi-VN"}
            for label, voice_id in self._tts.list_preset_voices()
        ]
        if not self.voices:
            raise RuntimeError("Managed VieNeu-TTS has no preset voices")
        LOGGER.info("Managed VieNeu-TTS ready with %d preset voices", len(self.voices))

    def synthesize(
        self,
        *,
        text: str,
        voice: str,
        style: str,
        rate: float,
        volume: float,
    ) -> bytes:
        known_voices = {item["id"] for item in self.voices}
        if voice not in known_voices:
            raise ValueError("The selected VieNeu voice is not available")
        if style not in STYLES:
            raise ValueError("The selected VieNeu reading style is not supported")

        with self._lock:
            samples = self._tts.infer(text, voice=voice, style=style)
        audio = np.asarray(samples, dtype=np.float32).reshape(-1)
        if audio.size == 0:
            raise RuntimeError("VieNeu-TTS returned no audio")
        if abs(rate - 1.0) > 0.001:
            positions = np.arange(0.0, float(audio.size), rate, dtype=np.float64)
            audio = np.interp(
                positions,
                np.arange(audio.size, dtype=np.float64),
                audio,
            ).astype(np.float32)
        audio = np.clip(audio * volume, -1.0, 1.0)
        pcm16 = np.rint(audio * 32767.0).astype("<i2", copy=False)

        output = io.BytesIO()
        with wave.open(output, "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(SAMPLE_RATE_HZ)
            wav.writeframes(pcm16.tobytes())
        return output.getvalue()


class BridgeServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(
        self,
        address: tuple[str, int],
        runtime: VieNeuRuntime,
        token: str,
        nonce: str,
    ) -> None:
        super().__init__(address, BridgeHandler)
        self.runtime = runtime
        self.token = token
        self.nonce = nonce


class BridgeHandler(BaseHTTPRequestHandler):
    server: BridgeServer
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if not self._authorized():
            self._send_error(HTTPStatus.UNAUTHORIZED, "unauthorized", "Authentication failed.")
            return
        if self.path == "/health":
            self._send_json(
                HTTPStatus.OK,
                {
                    "ok": True,
                    "engine": "VieNeu-TTS-v3-Turbo",
                    "sampleRateHz": SAMPLE_RATE_HZ,
                    "voiceCount": len(self.server.runtime.voices),
                    "nonce": self.server.nonce,
                },
            )
            return
        if self.path == "/voices":
            self._send_json(HTTPStatus.OK, self.server.runtime.voices)
            return
        self._send_error(HTTPStatus.NOT_FOUND, "not_found", "Endpoint not found.")

    def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if not self._authorized():
            self._send_error(HTTPStatus.UNAUTHORIZED, "unauthorized", "Authentication failed.")
            return
        if self.path != "/synthesize":
            self._send_error(HTTPStatus.NOT_FOUND, "not_found", "Endpoint not found.")
            return
        try:
            payload = self._read_json()
            text = str(payload.get("text", "")).strip()
            voice = str(payload.get("voice", "")).strip()
            style = str(payload.get("style", "tu_nhien")).strip()
            rate = float(payload.get("rate", 1.0))
            volume = float(payload.get("volume", 1.0))
            if not text:
                raise ValueError("Text is required")
            if len(text) > MAX_TEXT_CHARS:
                raise ValueError(f"Text exceeds {MAX_TEXT_CHARS} characters")
            if not 0.5 <= rate <= 2.0:
                raise ValueError("Rate must be between 0.5 and 2.0")
            if not 0.0 <= volume <= 1.0:
                raise ValueError("Volume must be between 0 and 1")
            audio = self.server.runtime.synthesize(
                text=text,
                voice=voice,
                style=style,
                rate=rate,
                volume=volume,
            )
        except (ValueError, json.JSONDecodeError) as error:
            self._send_error(HTTPStatus.BAD_REQUEST, "invalid_request", str(error))
            return
        except Exception as error:
            LOGGER.error("VieNeu synthesis failed: %s", type(error).__name__)
            self._send_error(
                HTTPStatus.INTERNAL_SERVER_ERROR,
                "synthesis_failed",
                "VieNeu-TTS could not synthesize this sentence.",
            )
            return

        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "audio/wav")
        self.send_header("Content-Length", str(len(audio)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(audio)

    def log_message(self, message: str, *args: Any) -> None:
        LOGGER.debug("request: %s", message % args)

    def _authorized(self) -> bool:
        supplied = self.headers.get("Authorization", "")
        expected = f"Bearer {self.server.token}"
        return hmac.compare_digest(supplied.encode("utf-8"), expected.encode("utf-8"))

    def _read_json(self) -> dict[str, Any]:
        raw_length = self.headers.get("Content-Length", "")
        try:
            length = int(raw_length)
        except ValueError as error:
            raise ValueError("Invalid Content-Length") from error
        if length <= 0 or length > MAX_REQUEST_BYTES:
            raise ValueError("Invalid request size")
        content_type = self.headers.get("Content-Type", "").split(";", 1)[0].strip()
        if content_type != "application/json":
            raise ValueError("Content-Type must be application/json")
        payload = json.loads(self.rfile.read(length).decode("utf-8"))
        if not isinstance(payload, dict):
            raise ValueError("JSON body must be an object")
        return payload

    def _send_json(self, status: HTTPStatus, payload: Any) -> None:
        encoded = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(encoded)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(encoded)

    def _send_error(self, status: HTTPStatus, code: str, message: str) -> None:
        self._send_json(status, {"error": {"code": code, "message": message}})


def read_bootstrap() -> tuple[str, str]:
    line = sys.stdin.buffer.readline(MAX_REQUEST_BYTES)
    if not line:
        raise RuntimeError("Missing parent bootstrap")
    payload = json.loads(line.decode("utf-8"))
    token = str(payload.get("token", ""))
    nonce = str(payload.get("nonce", ""))
    if len(token) < 43 or len(nonce) < 32:
        raise RuntimeError("Invalid parent bootstrap")
    return token, nonce


def start_parent_watchdog() -> None:
    def watch() -> None:
        try:
            while sys.stdin.buffer.read(4096):
                pass
        finally:
            os._exit(0)

    threading.Thread(target=watch, name="parent-watchdog", daemon=True).start()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Managed VieNeu-TTS runtime for baka-trans")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--install-model", action="store_true")
    mode.add_argument("--check-model", action="store_true")
    parser.add_argument("--model-dir", type=Path, required=True)
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--threads", type=int, default=3)
    return parser.parse_args()


def serve(args: argparse.Namespace) -> None:
    token, nonce = read_bootstrap()
    start_parent_watchdog()
    runtime = VieNeuRuntime(model_dir=args.model_dir, threads=max(1, min(args.threads, 8)))
    server = BridgeServer(("127.0.0.1", args.port), runtime, token, nonce)
    port = int(server.server_address[1])
    emit_event("ready", port=port, nonce=nonce, pid=os.getpid())
    try:
        server.serve_forever(poll_interval=0.25)
    finally:
        server.server_close()


def main() -> None:
    args = parse_args()
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
        stream=sys.stderr,
    )
    try:
        if args.install_model:
            install_model(args.model_dir)
        elif args.check_model:
            manifest = validate_model(args.model_dir, emit_progress=True)
            emit_event(
                "complete",
                phase="installed",
                downloadedBytes=TOTAL_MODEL_BYTES,
                verifiedBytes=TOTAL_MODEL_BYTES,
                totalBytes=TOTAL_MODEL_BYTES,
                percent=100,
                message="VieNeu-TTS model verification passed.",
                modelVersion=manifest["modelVersion"],
            )
        else:
            serve(args)
    except KeyboardInterrupt:
        emit_event("cancelled", phase="paused", message="VieNeu-TTS setup was paused.")
        raise SystemExit(130)
    except Exception as error:
        LOGGER.error("Managed VieNeu operation failed: %s", type(error).__name__)
        emit_event(
            "error",
            phase="error",
            code="managed_vieneu_failed",
            message=str(error),
        )
        raise SystemExit(1)


if __name__ == "__main__":
    main()
