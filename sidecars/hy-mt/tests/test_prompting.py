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
        "Translate the following segment into Vietnamese, "
        "without additional explanation.\n\nおはよう。"
    )
    assert chat_messages("おはよう。") == [
        {
                "role": "user",
                "content": (
                    "Translate the following segment into Vietnamese, "
                    "without additional explanation.\n\nおはよう。"
                ),
        }
    ]


def test_official_prompt_english_source_uses_xx_template() -> None:
    assert official_prompt("Good morning.", source_language_code="en") == (
        "Translate the following segment into Vietnamese, "
        "without additional explanation.\n\nGood morning."
    )


def test_official_prompt_zh_source_uses_zh_template() -> None:
    assert official_prompt("你好。", source_language_code="zh", target_language_code="vi") == (
        "将以下文本翻译为越南语，注意只需要输出翻译后的结果，"
        "不要额外解释：\n\n你好。"
    )


def test_official_prompt_zh_target_uses_zh_template() -> None:
    assert official_prompt("Xin chào.", source_language_code="vi", target_language_code="zh") == (
        "将以下文本翻译为中文，注意只需要输出翻译后的结果，"
        "不要额外解释：\n\nXin chào."
    )


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
