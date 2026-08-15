"""Corpus and sustained-loop benchmark orchestration."""

from __future__ import annotations

import json
import statistics
import sys
import time
from pathlib import Path
from typing import Any

from .evidence import memory_snapshot, write_csv, write_json
from .runner import HyMtRunner


def load_corpus(path: Path) -> list[dict[str, Any]]:
    cases = []
    with path.open("r", encoding="utf-8") as stream:
        for line_number, line in enumerate(stream, start=1):
            if not line.strip():
                continue
            item = json.loads(line)
            item["lineNumber"] = line_number
            cases.append(item)
    if len(cases) < 30:
        raise RuntimeError("benchmark corpus must contain at least 30 cases")
    return cases


def percentile(values: list[float], percent: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    index = min(len(ordered) - 1, max(0, round((len(ordered) - 1) * percent)))
    return round(ordered[index], 3)


def summarize(rows: list[dict[str, Any]]) -> dict[str, Any]:
    completed = [row for row in rows if row.get("status") == "ok"]
    latencies = [float(row["latencyMs"]) for row in completed]
    output_tokens = [int(row["outputTokens"]) for row in completed]
    return {
        "attempted": len(rows),
        "completed": len(completed),
        "failures": len(rows) - len(completed),
        "emptyOutputs": sum(not row.get("candidate", "").strip() for row in completed),
        "latencyP50Ms": percentile(latencies, 0.50),
        "latencyP95Ms": percentile(latencies, 0.95),
        "latencyMeanMs": round(statistics.fmean(latencies), 3) if latencies else None,
        "totalOutputTokens": sum(output_tokens),
    }


def run_corpus(
    runner: HyMtRunner,
    corpus_path: Path,
    output_dir: Path,
    modes: list[str],
) -> dict[str, Any]:
    cases = load_corpus(corpus_path)
    rows: list[dict[str, Any]] = []
    boundary_rows: list[dict[str, Any]] = []
    for case in cases:
        if not case.get("accepted", True):
            try:
                runner.render_prompt(case.get("source"))
            except (TypeError, ValueError) as error:
                boundary_rows.append(
                    {"id": case["id"], "status": "rejected", "error": str(error)}
                )
            else:
                boundary_rows.append(
                    {"id": case["id"], "status": "unexpectedly-accepted", "error": ""}
                )
            continue
        for mode in modes:
            base = {
                "id": case["id"],
                "category": case["category"],
                "mode": mode,
                "source": case["source"],
                "referenceVi": case["reference_vi"],
            }
            try:
                result = runner.translate(case["source"], generation_mode=mode)
            except Exception as error:  # preserve an experimental failure as evidence
                rows.append({**base, "status": "error", "error": repr(error)})
            else:
                rows.append(
                    {
                        **base,
                        "status": "ok",
                        "candidate": result.text,
                        "inputTokens": result.input_tokens,
                        "outputTokens": result.output_tokens,
                        "latencyMs": result.latency_ms,
                        "tokensPerSecond": result.tokens_per_second,
                        "rssBytes": result.memory["rssBytes"],
                        "mpsDriverAllocatedBytes": result.memory.get("mpsDriverAllocatedBytes"),
                    }
                )
    payload = {
        "runner": runner.metadata(),
        "corpus": str(corpus_path.resolve()),
        "modes": modes,
        "summary": {mode: summarize([row for row in rows if row["mode"] == mode]) for mode in modes},
        "boundaryCases": boundary_rows,
        "rows": rows,
    }
    write_json(output_dir / "benchmark.json", payload)
    write_csv(output_dir / "benchmark.csv", rows)
    write_csv(output_dir / "boundaries.csv", boundary_rows)
    return payload


def run_soak(
    runner: HyMtRunner,
    corpus_path: Path,
    output_dir: Path,
    duration_seconds: float,
    interval_seconds: float,
) -> dict[str, Any]:
    cases = [case for case in load_corpus(corpus_path) if case.get("accepted", True)]
    fixtures = cases[: min(6, len(cases))]
    started = time.monotonic()
    deadline = started + duration_seconds
    rows: list[dict[str, Any]] = []
    peak_rss = 0
    peak_system_percent = 0.0
    sequence = 0
    while time.monotonic() < deadline:
        cycle_started = time.monotonic()
        case = fixtures[sequence % len(fixtures)]
        row: dict[str, Any] = {"sequence": sequence, "id": case["id"]}
        try:
            result = runner.translate(case["source"], generation_mode="greedy")
        except Exception as error:  # preserve soak failures and continue
            row.update(status="error", error=repr(error))
        else:
            memory = result.memory
            peak_rss = max(peak_rss, int(memory["rssBytes"]))
            peak_system_percent = max(peak_system_percent, float(memory["systemUsedPercent"]))
            row.update(
                status="ok",
                latencyMs=result.latency_ms,
                outputTokens=result.output_tokens,
                candidate=result.text,
                rssBytes=memory["rssBytes"],
                systemUsedPercent=memory["systemUsedPercent"],
                mpsDriverAllocatedBytes=memory.get("mpsDriverAllocatedBytes"),
            )
        rows.append(row)
        sequence += 1
        if sequence == 1 or sequence % max(1, round(60 / interval_seconds)) == 0:
            elapsed = time.monotonic() - started
            print(
                f"soak progress: {elapsed:.1f}s/{duration_seconds:.1f}s, iterations={sequence}",
                file=sys.stderr,
                flush=True,
            )
        remaining = interval_seconds - (time.monotonic() - cycle_started)
        if remaining > 0 and time.monotonic() + remaining < deadline:
            time.sleep(remaining)

    ok_rows = [row for row in rows if row["status"] == "ok"]
    latencies = [float(row["latencyMs"]) for row in ok_rows]
    payload = {
        "runner": runner.metadata(),
        "durationTargetSeconds": duration_seconds,
        "durationActualSeconds": round(time.monotonic() - started, 3),
        "intervalSeconds": interval_seconds,
        "iterations": len(rows),
        "completed": len(ok_rows),
        "failures": len(rows) - len(ok_rows),
        "slowerThanFixtureInterval": sum(value > interval_seconds * 1_000 for value in latencies),
        "latencyP50Ms": percentile(latencies, 0.50),
        "latencyP95Ms": percentile(latencies, 0.95),
        "peakRssBytes": peak_rss,
        "peakSystemUsedPercent": peak_system_percent,
        "memoryAtEnd": memory_snapshot(),
        "rows": rows,
    }
    write_json(output_dir / "soak.json", payload)
    write_csv(output_dir / "soak.csv", rows)
    return payload
