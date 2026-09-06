# Phase 04 — Regression Matrix, Docs, and Coordination Log

Parent: [plan.md](plan.md)

## Objective

Prove the three behavior changes together, update every document that
describes the old drop-newest error semantics, and record the coordination
note in the umbrella plan.

## Regression matrix

Rust (all cross-platform unless noted):

- `speech_queue.rs`: capacity/coalesce/drop-oldest/close/wakeup tests from
  phase 01 — green.
- `local_worker.rs`: activity-state matrix (phase 02) — green; existing
  segmenter/upsert/drain tests untouched and green.
- `tts.rs`: `voice_needs_reload` unit tests (phase 03) — green; platform
  smoke tests remain `#[ignore]`.
- New: drain-contract tests matching the REAL stop semantics (red-team
  correction — Stop sets cancellation before draining and discards queued
  speech; pause/natural-end drains and speaks):
  - stop path: `cancellation` set before `close()` → consumer exits at the
    active-check without draining (queue contents abandoned, as today);
  - pause path: no cancellation → `close()` after the producer exits →
    consumer drains all queued items then returns `None`.
- New (phase-02 guard): force `playback_tx` failures during the stop race
  and assert the Full/Disconnected app-errors are suppressed once the queue
  is closed / worker inactive, and `playback_inflight` returns to baseline.

Gates (match `scripts/release-macos.sh:173-184` and
`.github/workflows/desktop.yml`):

```text
npm test
npm run build
cargo fmt --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --all-targets -- -D warnings   # local, both cfg targets via CI
```

Manual soak (macOS, System voice; repeat once with VieNeu if installed):

| Scenario | Expected |
| --- | --- |
| ≥3 min continuous speech | zero `local_tts_backlog_full` errors; zero `local_tts_playback_error` Full errors (the decidable criterion — no per-utterance "spoken" log exists; optionally add a debug log on pacer completion carrying the utterance id + coalesce count) |
| Artificial stall (temporarily add a 3 s sleep in the TTS worker behind `#[cfg(test)]`/local patch, or throttle by setting `tts_rate` high and speaking fast) | sentences coalesce while backlog is fresh; past `MAX_QUEUE_AGE_MS` (10 s) or `MAX_COALESCED_CHARS` the drop-oldest error fires naming a skipped utterance id — speech never lags silently beyond the bound |
| Stop mid-playback | audio cuts instantly; no level-reset or stage event after the session ends |
| Stop → immediate new session | no stale events from the old generation reach the new session's UI; no orphaned-`say` temp-file race (per-call UUID names kept) |
| Pause | capture/translation stop; the in-flight sentence plus up to `MAX_BUFFERS_AHEAD` (3) already-synthesized sentences play out (amended contract, documented in phase 02) |
| Windows (CI + manual follow-up) | cached synthesizer compiles and synthesizes; voice switch applies next sentence |

## Docs

- `docs/RELIABILITY.md`: add/refresh the local-pipeline backlog behavior —
  coalesce-on-full, drop-oldest as last resort, and the pipelined playback
  model (search the doc for the old error text first; update if present).
- `README.md`: if the local-mode error table or troubleshooting mentions the
  falling-behind message, update it to the new drop-oldest wording.
- New `docs/logs/2026-09-06-local-tts-backlog-optimization.md`: what changed,
  why (deficit math: `S + P` vs `max(S, P)`), observable behavior differences
  (banner now fires only on true drops; stage indicator alternates
  synthesizing/speaking), and rollback notes (phases are independently
  revertable).
- Error message copy: the only user-visible string change is the
  `local_tts_backlog_full` message from phase 01. No event names, error
  codes, or frontend contracts change.

## Cross-plan coordination

- `plans/260716-2033-local-llm-audio-translation/plan.md`: append a dated
  context-scan bullet (matching the file's existing `2026-07-18 ... update:`
  convention) stating that speech-queue coalescing/drop-oldest, pipelined
  playback, and synthesizer caching are tracked in
  `plans/260906-2338-local-tts-backlog-optimization`, and that the
  bounded-queue + speech-order invariants from its phase 06/07 are preserved
  (bounded by `SPEECH_QUEUE_CAPACITY` + `MAX_COALESCED_CHARS`; order by FIFO
  + tail-merge).
- This plan's `reports/` directory: store the red-team verdict.

## Acceptance

- All gates green on macOS locally and on both CI runners.
- Docs updated where the old behavior is described; coordination bullet
  present in the umbrella plan.
- No change to: `LocalTranslationConfig` schema, Tauri command surface,
  frontend files, event names, or cloud pipelines (verify with
  `git diff --stat` — only `src-tauri/src/ai/*`, `src-tauri/src/tts.rs`,
  docs, and plans may appear).
