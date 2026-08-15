"""Decode only model output created after the prompt tokens."""

from __future__ import annotations

from typing import Any


def generated_suffix(generated_ids: Any, input_token_count: int) -> Any:
    if input_token_count < 0:
        raise ValueError("input token count cannot be negative")
    if len(generated_ids.shape) != 2:
        raise ValueError("generated token tensor must have two dimensions")
    if generated_ids.shape[1] < input_token_count:
        raise ValueError("generated token tensor is shorter than the prompt")
    return generated_ids[:, input_token_count:]


def decode_generated_suffix(
    tokenizer: Any,
    generated_ids: Any,
    input_token_count: int,
) -> str:
    suffix = generated_suffix(generated_ids, input_token_count)
    return tokenizer.batch_decode(suffix, skip_special_tokens=True)[0].strip()
