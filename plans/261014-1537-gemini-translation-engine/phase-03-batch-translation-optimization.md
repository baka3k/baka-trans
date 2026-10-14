# Phase 03 — Batch translation worker + docs + regression

Status: planned
Parent: [plan.md](plan.md)
Blocked by: phases 01-02
Test scope: Rust unit tests (`src-tauri/src/ai/local_worker.rs` inline tests) + docs; full regression gates. Expected size: **M**.

## Goal

Optimize the translation process by sending Whisper sentences to Gemini in sliding-window
batches (context retention across chunks) while preserving strict per-item ordering,
transcript snapshot semantics, TTS enqueue, and cancellation behavior. Other engines keep
today's per-sentence latency exactly.

## Tasks

1. **Batch-capable worker** (`src-tauri/src/ai/local_worker.rs`)
   - Extend `TranslationClient` usage: engines declare
     `batch_window() -> Option<(usize, Duration)>` — `Some((5, 500 ms))` for Gemini,
     `None` for Hy-MT2 and OpenAI-compatible (constants in code with a one-line
     comment tying 500 ms to the 1-3 s end-to-end promise).
   - In `spawn_translation_worker` (`local_worker.rs:454-557`): when
     `batch_window()` is `Some`, after the first `recv()` drain up to window-1 more
     jobs with `try_recv`, then wait the remaining max-wait for additional jobs; send
     one `translate_batch` call; then for each job in **arrival order** emit the Final
     snapshot (revision 2, `latency_ms` measured per job from its own `started`) and
     `try_send` its TTS request — reusing the existing snapshot/TTS/backlog logic
     unchanged; on batch error emit Error snapshots for every job in the window (same
     `app-error`/snapshot semantics as today, no retry, no engine fallback).
   - `None` path: keep the current single-flight loop byte-for-byte.
   - Cancellation/generation checks (`is_worker_active`) run before the batch call,
     after it, and between per-item emissions; while waiting out the max-wait window,
     `select!` against the cancellation token so a stopped session never fires a paid
     request for a cancelled window.
2. **Docs**
   - `README.md` local-route engine list + `ARCHITECTURE.md` engine chain line: add
     Gemini (cloud) and the batch behavior note.
   - `docs/SECURITY.md` if it enumerates keychain entries — Google key reuse note
     (no new entry).
3. **Regression run** — all gates from plan.md; manual smoke optional: engine test with
   a real key, short session with 6+ sentences producing ≥1 full window + 1 partial.

## Acceptance criteria → test mapping

| Rule | Test |
|---|---|
| Window of 5 available jobs → exactly one `translate_batch` call, 5 Final snapshots in arrival order | typical batch test with mock client |
| Slow speech: single job flushed at max-wait, not delayed by an empty queue drain | timeout-boundary test (fake clock or short constant) |
| Batch error → every window job gets an Error snapshot; no retry, no fallback | invalid-input test |
| Length mismatch → missing indexes get `local_gemini_batch_length_mismatch` errors, extras truncated | boundary test (short + long in one table) |
| `None` window engines keep single-flight behavior | regression test: existing worker tests unchanged and green |
| Cancellation mid-window stops without late emissions | cancellation test |
| TTS backlog handling preserved per item (full → `local_tts_backlog_full`, kept text) | reuse/extend existing TTS backlog test with batch input |

Budget: ~7 test functions, within the ceiling.

## Out of scope

- Making Hy-MT2 or OpenAI-compatible batch-capable; user-facing batching settings;
  overlay `?key=` migration.
