"""Loopback-only VieNeu-TTS bridge used by the baka-trans desktop app."""

from __future__ import annotations

import argparse
import io
import json
import logging
import threading
import wave
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any

import numpy as np
from vieneu import Vieneu


LOGGER = logging.getLogger("baka-trans-vieneu")
MAX_REQUEST_BYTES = 64 * 1024
MAX_TEXT_CHARS = 4_000
SAMPLE_RATE_HZ = 48_000
STYLES = {"tu_nhien", "tin_tuc", "doc_truyen"}


class VieNeuRuntime:
    def __init__(self, *, backend: str, precision: str, threads: int) -> None:
        LOGGER.info("Loading VieNeu-TTS v3 Turbo (%s/%s)", backend, precision)
        self._tts = Vieneu(
            mode="v3turbo",
            backend=backend,
            precision=precision,
            threads=threads,
        )
        self._lock = threading.Lock()
        self.voices = [
            {"id": str(voice_id), "name": str(label), "language": "vi-VN"}
            for label, voice_id in self._tts.list_preset_voices()
        ]
        LOGGER.info("VieNeu-TTS ready with %d preset voices", len(self.voices))

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
            raise ValueError(f"Unknown VieNeu voice: {voice}")
        if style not in STYLES:
            raise ValueError(f"Unknown VieNeu reading style: {style}")

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

    def __init__(self, address: tuple[str, int], runtime: VieNeuRuntime) -> None:
        super().__init__(address, BridgeHandler)
        self.runtime = runtime


class BridgeHandler(BaseHTTPRequestHandler):
    server: BridgeServer
    protocol_version = "HTTP/1.1"

    def do_GET(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if self.path == "/health":
            self._send_json(
                HTTPStatus.OK,
                {
                    "ok": True,
                    "engine": "VieNeu-TTS-v3-Turbo",
                    "sampleRateHz": SAMPLE_RATE_HZ,
                    "voiceCount": len(self.server.runtime.voices),
                },
            )
            return
        if self.path == "/voices":
            self._send_json(HTTPStatus.OK, self.server.runtime.voices)
            return
        self._send_error(HTTPStatus.NOT_FOUND, "Endpoint not found")

    def do_POST(self) -> None:  # noqa: N802 - BaseHTTPRequestHandler API
        if self.path != "/synthesize":
            self._send_error(HTTPStatus.NOT_FOUND, "Endpoint not found")
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
            self._send_error(HTTPStatus.BAD_REQUEST, str(error))
            return
        except Exception as error:  # model errors need to reach the desktop client
            LOGGER.exception("VieNeu synthesis failed")
            self._send_error(HTTPStatus.INTERNAL_SERVER_ERROR, str(error))
            return

        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "audio/wav")
        self.send_header("Content-Length", str(len(audio)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(audio)

    def log_message(self, message: str, *args: Any) -> None:
        LOGGER.info("%s - %s", self.client_address[0], message % args)

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

    def _send_error(self, status: HTTPStatus, message: str) -> None:
        self._send_json(status, {"error": message})


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="VieNeu-TTS bridge for baka-trans")
    parser.add_argument("--host", default="127.0.0.1", choices=("127.0.0.1",))
    parser.add_argument("--port", type=int, default=23334)
    parser.add_argument("--backend", default="onnx", choices=("onnx", "auto", "pytorch"))
    parser.add_argument("--precision", default="int8", choices=("int8", "fp32"))
    parser.add_argument("--threads", type=int, default=0)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    runtime = VieNeuRuntime(
        backend=args.backend,
        precision=args.precision,
        threads=max(0, args.threads),
    )
    server = BridgeServer((args.host, args.port), runtime)
    LOGGER.info("Listening on http://%s:%d", args.host, args.port)
    try:
        server.serve_forever(poll_interval=0.25)
    except KeyboardInterrupt:
        LOGGER.info("Stopping")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
