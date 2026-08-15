from __future__ import annotations

import pytest

from hy_mt_poc.prompting import chat_messages, official_prompt, tokenize_chat


class RecordingTokenizer:
    def __init__(self) -> None:
        self.call = None

    def apply_chat_template(self, messages, **kwargs):
        self.call = (messages, kwargs)
        return "tokens"


def test_official_prompt_matches_model_card_shape() -> None:
    assert official_prompt("  おはよう。 ") == (
        "Translate the following text into Vietnamese. Note that you should only output "
        "the translated result without any additional explanation:\n\nおはよう。"
    )
    assert chat_messages("おはよう。") == [
        {
                "role": "user",
                "content": (
                    "Translate the following text into Vietnamese. Note that you should only output "
                    "the translated result without any additional explanation:\n\nおはよう。"
                ),
        }
    ]


def test_chat_template_has_no_system_or_generation_prompt() -> None:
    tokenizer = RecordingTokenizer()
    assert tokenize_chat(tokenizer, "おはよう。") == "tokens"
    messages, kwargs = tokenizer.call
    assert [message["role"] for message in messages] == ["user"]
    assert kwargs == {
        "tokenize": True,
        "add_generation_prompt": True,
        "return_tensors": "pt",
    }


@pytest.mark.parametrize("value", ["", "   ", "\n\t", "abc\x00def"])
def test_empty_or_malformed_text_is_rejected(value: str) -> None:
    with pytest.raises(ValueError):
        official_prompt(value)


def test_non_string_text_is_rejected() -> None:
    with pytest.raises(TypeError):
        official_prompt(None)  # type: ignore[arg-type]
