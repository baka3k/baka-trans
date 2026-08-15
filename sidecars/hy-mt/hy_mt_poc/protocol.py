"""Bounded, versioned NDJSON protocol for the managed HY-MT sidecar."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, TextIO

from .constants import MAX_INPUT_CHARS, MAX_PROTOCOL_LINE_BYTES, MAX_REQUEST_BYTES, MAX_NEW_TOKENS, PROTOCOL_VERSION


@dataclass(frozen=True)
class ProtocolError(Exception):
    code: str
    message: str
    retryable: bool = False

    def payload(self, request_id: str | None = None) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "type": "error",
            "code": self.code,
            "message": self.message,
            "retryable": self.retryable,
        }
        if request_id is not None:
            payload["id"] = request_id
        return payload


def emit(stream: TextIO, payload: dict[str, Any]) -> None:
    """Write one protocol frame. Callers must reserve stdout for this function."""
    stream.write(json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n")
    stream.flush()


def parse_line(raw: bytes) -> dict[str, Any]:
    if len(raw) > MAX_PROTOCOL_LINE_BYTES:
        raise ProtocolError("line_too_large", "Protocol line exceeds the supported size.")
    try:
        decoded = raw.decode("utf-8")
        value = json.loads(decoded)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ProtocolError("invalid_json", "Protocol message must be UTF-8 JSON.") from exc
    if not isinstance(value, dict):
        raise ProtocolError("invalid_request", "Protocol message must be a JSON object.")
    return value


def _request_id(message: dict[str, Any], *, required: bool) -> str | None:
    request_id = message.get("id")
    if request_id is None and not required:
        return None
    if not isinstance(request_id, str) or not request_id or len(request_id) > 128:
        raise ProtocolError("invalid_id", "Request id must be a non-empty string up to 128 characters.")
    return request_id


def validate_translate(message: dict[str, Any]) -> dict[str, Any]:
    if message.get("type") != "translate":
        raise ProtocolError("unsupported_message", "Unsupported protocol message type.")
    if message.get("protocolVersion") != PROTOCOL_VERSION:
        raise ProtocolError("protocol_mismatch", "Unsupported protocol version.")
    request_id = _request_id(message, required=True)
    source = message.get("sourceLanguage")
    target = message.get("targetLanguage")
    text = message.get("text")
    max_new_tokens = message.get("maxNewTokens", MAX_NEW_TOKENS)
    if not isinstance(source, str) or not source:
        raise ProtocolError("unsupported_language", "Source language must be a non-empty language code.")
    if target != "vi":
        raise ProtocolError("unsupported_language", "Only Vietnamese target language is currently supported.")
    if not isinstance(text, str) or not text.strip():
        raise ProtocolError("invalid_text", "Translation text must be non-empty.")
    if len(text.strip()) > MAX_INPUT_CHARS:
        raise ProtocolError("request_too_large", "Translation text exceeds the supported request size.")
    if len(text.encode("utf-8")) > MAX_REQUEST_BYTES:
        raise ProtocolError("request_too_large", "Translation text exceeds the supported request size.")
    if not isinstance(max_new_tokens, int) or isinstance(max_new_tokens, bool) or not 1 <= max_new_tokens <= MAX_NEW_TOKENS:
        raise ProtocolError("invalid_token_limit", f"maxNewTokens must be between 1 and {MAX_NEW_TOKENS}.")
    return {"id": request_id, "text": text, "maxNewTokens": max_new_tokens}


def validate_cancel(message: dict[str, Any]) -> str:
    if message.get("type") != "cancel":
        raise ProtocolError("unsupported_message", "Unsupported protocol message type.")
    if message.get("protocolVersion") != PROTOCOL_VERSION:
        raise ProtocolError("protocol_mismatch", "Unsupported protocol version.")
    return _request_id(message, required=True) or ""
