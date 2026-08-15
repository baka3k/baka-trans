from __future__ import annotations

from hy_mt_poc.prompting import tokenize_chat


class BatchEncodingLike:
    input_ids = "input-ids"


class Transformers5Tokenizer:
    def apply_chat_template(self, *args, **kwargs):
        del args, kwargs
        return BatchEncodingLike()


def test_tokenize_chat_normalizes_transformers5_batch_encoding() -> None:
    assert tokenize_chat(Transformers5Tokenizer(), "おはよう。") == "input-ids"
