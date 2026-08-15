from __future__ import annotations

import pytest

from hy_mt_poc.protocol import ProtocolError, parse_line, validate_cancel, validate_translate


def translate(**overrides: object) -> dict[str, object]:
    payload: dict[str, object] = {
        "type": "translate",
        "protocolVersion": 1,
        "id": "request-1",
        "sourceLanguage": "ja",
        "targetLanguage": "vi",
        "text": "確認してください。",
        "maxNewTokens": 32,
    }
    payload.update(overrides)
    return payload


def test_translate_request_is_bounded_and_normalized() -> None:
    assert validate_translate(translate()) == {"id": "request-1", "text": "確認してください。", "maxNewTokens": 32}


@pytest.mark.parametrize(
    ("payload", "code"),
    [
        (translate(protocolVersion=2), "protocol_mismatch"),
        (translate(sourceLanguage="en"), "unsupported_language"),
        (translate(text=""), "invalid_text"),
        (translate(maxNewTokens=0), "invalid_token_limit"),
        (translate(id=""), "invalid_id"),
    ],
)
def test_translate_request_rejects_untrusted_fields(payload: dict[str, object], code: str) -> None:
    with pytest.raises(ProtocolError, match=code) as raised:
        validate_translate(payload)
    assert raised.value.code == code


def test_malformed_and_oversized_lines_are_rejected() -> None:
    with pytest.raises(ProtocolError) as malformed:
        parse_line(b"not json\n")
    assert malformed.value.code == "invalid_json"
    with pytest.raises(ProtocolError) as oversized:
        parse_line(b"x" * (64 * 1024 + 1))
    assert oversized.value.code == "line_too_large"


def test_cancel_requires_protocol_version_and_id() -> None:
    assert validate_cancel({"type": "cancel", "protocolVersion": 1, "id": "request-1"}) == "request-1"
    with pytest.raises(ProtocolError) as raised:
        validate_cancel({"type": "cancel", "protocolVersion": 1})
    assert raised.value.code == "invalid_id"
