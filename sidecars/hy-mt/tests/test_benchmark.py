from __future__ import annotations

from pathlib import Path

from hy_mt_poc.benchmark import load_corpus, percentile, summarize


def test_percentiles_and_summary_are_stable() -> None:
    rows = [
        {"status": "ok", "latencyMs": 10, "outputTokens": 2, "candidate": "a"},
        {"status": "ok", "latencyMs": 30, "outputTokens": 3, "candidate": ""},
        {"status": "error", "error": "boom"},
    ]
    assert percentile([10, 20, 30], 0.95) == 30
    assert summarize(rows) == {
        "attempted": 3,
        "completed": 2,
        "failures": 1,
        "emptyOutputs": 1,
        "latencyP50Ms": 10,
        "latencyP95Ms": 30,
        "latencyMeanMs": 20.0,
        "totalOutputTokens": 5,
    }


def test_fixed_corpus_has_accepted_coverage_and_boundaries() -> None:
    corpus = load_corpus(Path(__file__).parents[1] / "corpus" / "ja-vi.jsonl")
    accepted = [item for item in corpus if item["accepted"]]
    rejected = [item for item in corpus if not item["accepted"]]
    categories = {item["category"] for item in accepted}
    assert len(accepted) >= 30
    assert len(rejected) >= 4
    assert {
        "names",
        "numbers",
        "technical-meeting",
        "short",
        "long",
        "punctuation",
        "mixed-english",
    } <= categories
