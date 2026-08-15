"""Prompt construction and chat-template application."""

from __future__ import annotations

from typing import Any

from .constants import (
    LANGUAGE_NAMES,
    LANGUAGE_NAMES_ZH,
    MAX_INPUT_CHARS,
    PROMPT_TEMPLATE,
    PROMPT_TEMPLATE_ZH,
    TARGET_LANGUAGE_NAME,
)

_ZH_CODES = {"zh", "zh-Hans", "zh-Hant"}


def language_name(code: str) -> str:
    return LANGUAGE_NAMES.get(code, code)


def language_name_zh(code: str) -> str:
    return LANGUAGE_NAMES_ZH.get(code, code)


def _is_zh_involved(source_code: str, target_code: str) -> bool:
    return source_code in _ZH_CODES or target_code in _ZH_CODES


def validate_source_text(source_text: str) -> str:
    if not isinstance(source_text, str):
        raise TypeError("source text must be a string")
    normalized = source_text.strip()
    if not normalized:
        raise ValueError("source text must not be empty")
    if "\x00" in normalized:
        raise ValueError("source text must not contain NUL characters")
    if len(normalized) > MAX_INPUT_CHARS:
        raise ValueError(f"source text exceeds {MAX_INPUT_CHARS} characters")
    return normalized


def official_prompt(
    source_text: str,
    *,
    source_language_code: str = "ja",
    target_language_code: str = "vi",
    target_language: str = TARGET_LANGUAGE_NAME,
) -> str:
    normalized = validate_source_text(source_text)
    if _is_zh_involved(source_language_code, target_language_code):
        return PROMPT_TEMPLATE_ZH.format(
            target_language=language_name_zh(target_language_code),
            source_text=normalized,
        )
    resolved_target = language_name(target_language_code) if target_language_code != "vi" else target_language
    return PROMPT_TEMPLATE.format(
        target_language=resolved_target,
        source_text=normalized,
    )


def chat_messages(
    source_text: str,
    *,
    source_language_code: str = "ja",
    target_language_code: str = "vi",
    target_language: str = TARGET_LANGUAGE_NAME,
) -> list[dict[str, str]]:
    return [{"role": "user", "content": official_prompt(
        source_text,
        source_language_code=source_language_code,
        target_language_code=target_language_code,
        target_language=target_language,
    )}]


def tokenize_chat(
    tokenizer: Any,
    source_text: str,
    *,
    source_language_code: str = "ja",
    target_language_code: str = "vi",
    target_language: str = TARGET_LANGUAGE_NAME,
) -> Any:
    tokenized = tokenizer.apply_chat_template(
        chat_messages(
            source_text,
            source_language_code=source_language_code,
            target_language_code=target_language_code,
            target_language=target_language,
        ),
        tokenize=True,
        add_generation_prompt=True,
        return_tensors="pt",
    )
    # Transformers 5 may return a BatchEncoding whereas the 4.x API returned
    # the input-id tensor directly. The runner deliberately needs only input
    # ids, so normalize both supported return shapes here.
    return getattr(tokenized, "input_ids", tokenized)
