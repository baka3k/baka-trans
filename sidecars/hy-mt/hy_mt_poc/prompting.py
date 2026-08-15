"""Prompt construction and chat-template application."""

from __future__ import annotations

from typing import Any

from .constants import LANGUAGE_NAMES, MAX_INPUT_CHARS, PROMPT_TEMPLATE, TARGET_LANGUAGE_NAME


def language_name(code: str) -> str:
    return LANGUAGE_NAMES.get(code, code)


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
    target_language: str = TARGET_LANGUAGE_NAME,
) -> str:
    normalized = validate_source_text(source_text)
    return PROMPT_TEMPLATE.format(
        source_language=language_name(source_language_code),
        target_language=target_language,
        source_text=normalized,
    )


def chat_messages(
    source_text: str,
    *,
    source_language_code: str = "ja",
    target_language: str = TARGET_LANGUAGE_NAME,
) -> list[dict[str, str]]:
    return [{"role": "user", "content": official_prompt(
        source_text,
        source_language_code=source_language_code,
        target_language=target_language,
    )}]


def tokenize_chat(
    tokenizer: Any,
    source_text: str,
    *,
    source_language_code: str = "ja",
    target_language: str = TARGET_LANGUAGE_NAME,
) -> Any:
    tokenized = tokenizer.apply_chat_template(
        chat_messages(
            source_text,
            source_language_code=source_language_code,
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
