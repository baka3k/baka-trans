from __future__ import annotations

import torch

from hy_mt_poc.decoding import decode_generated_suffix, generated_suffix


class FakeTokenizer:
    def batch_decode(self, values, *, skip_special_tokens):
        assert skip_special_tokens is True
        assert values.tolist() == [[21, 22]]
        return ["  bản dịch  "]


def test_generated_suffix_drops_all_prompt_tokens() -> None:
    values = torch.tensor([[10, 11, 12, 21, 22]])
    assert generated_suffix(values, 3).tolist() == [[21, 22]]
    assert decode_generated_suffix(FakeTokenizer(), values, 3) == "bản dịch"
