# Red Team Review — Local TTS Speech Backlog Optimization

Reviewed: 2026-09-06 · 3 hostile reviewers (Security Adversary, Assumption
Destroyer, Failure Mode Analyst) over plan.md + phase-01..04, cross-checked
against `local_worker.rs`, `tts.rs`, `audio.rs`, `session.rs`.

Verdict: **GO after fixes.** No Critical findings. 15 deduplicated findings:
13 accepted (folded into the phase docs), 2 rejected with rationale.

## Accepted findings (applied)

| # | Sev | Finding | Fix applied |
| --- | --- | --- | --- |
| 1 | HIGH | `close()` via `notify_waiters` alone loses wakeups (`Notified` registers on first poll) → TTS worker hangs in `pop()`, every stop degrades to the abort ladder | `close()` signals with permit-storing `notify_one()` + `notify_waiters()`; regression test `pop_wakes_on_close_without_registered_waiter` (phase-01) |
| 2 | HIGH | Phase-01 stop/drain premise was factually wrong: today Stop sets `cancellation` BEFORE draining, so queued speech is discarded, not spoken | Section rewritten: two real paths (cancel-on-Stop, drain-on-pause); parity test sets cancellation before close (phase-01, phase-04) |
| 3 | HIGH | Calling `close()` in the loop's stop paths closes the queue earlier than mpsc ever closes → silently drops speech spoken today on the pause path; also deletes the `local_tts_worker_closed` diagnostic unmentioned | `close()` moved to between translator-drain and tts-drain (mirrors mpsc ownership: `tts_tx` lives in `SentenceTranslator`); diagnostic removal documented + compensated by `drain_worker` join error (phase-01, plan.md) |
| 4 | HIGH | Post-phase-02, the stop race (playback dropped before Stop arrives) free-runs failed `try_send`s → sticky-banner error spam on every mid-speech stop | Full/Disconnected emits gated on `is_worker_active` + queue-not-closed; counter test for the Full path (phase-02, phase-04) |
| 5 | HIGH | Coalescing alone converts a loud failure into a silent 30-40 s speech lag (≈20+ sentences absorbed) — worse than the banner for live translation | `MAX_QUEUE_AGE_MS = 10 s` age bound sheds stale fronts through the same drop-oldest + error path; `TtsRequest.enqueued_at` added (phase-01, plan.md) |
| 6 | MED | `playback_inflight` drift if increment/spawn run on the failed-`try_send` branch → 3 failures permanently stall all speech | Increment+spawn specified on success branch only; drift regression test (phase-02) |
| 7 | MED | "Stale pacer settles are inert without the gate" is half false — `emit_local_pipeline_stage` emits unconditionally; the gate is load-bearing for the stage event | Safety analysis rewritten: gate is required, not defense in depth (phase-02) |
| 8 | MED | "Spoken exactly once, verified via transcript vs logs" is undecidable — no per-utterance spoken log exists | Success criterion redefined: zero backlog errors + zero playback Full errors; optional pacer-completion debug log (plan.md, phase-04) |
| 9 | MED | No poisoning story for the SpeechQueue Mutex | `lock().unwrap_or_else(\|p\| p.into_inner())` specified (phase-01) |
| 10 | MED | Fixed macOS temp file races an orphaned `say` (spawn_blocking survives worker abort): orphan's final `remove_file` can delete the new session's WAV mid-read | REJECTED the fixed-path idea; per-call UUID names kept; phase-03 macOS scope reduced to "no change" (phase-03, plan.md scope item 4) |
| 11 | MED | Cached WinRT instance can be used concurrently with an orphaned in-flight synthesis (uncancellable `SynthesizeTextToStreamAsync` after worker abort) | `in_flight` flag on the cache entry; concurrent acquisition falls back to a throwaway instance (phase-03) |
| 12 | LOW | Coalesce char budget ignored the separator (off-by-one); `debug_assert!(capacity >= 1)` missing (capacity-0 panic poisons the mutex) | `+1` counted; `debug_assert!` in `new()`; budget test pins the separator (phase-01) |
| 13 | LOW | "Independently revertable" false — phases 01/02 both rewrite `spawn_tts_worker`; effort 8 h optimistic | Reverse-order revert documented; effort raised to 12 h (plan.md) |
| 14 | LOW | Windows voice uninstalled mid-cache silently keeps the old voice (today: loud `local_tts_voice_missing`) | Documented as accepted behavior change in phase-03 + phase-04 log item |
| 15 | LOW | Cross-thread WinRT agility only exercised by running the ignored test on Windows | Phase-04 Windows manual follow-up must run `cargo test -- --ignored` once, not just compile |

## Rejected findings

| # | Sev | Finding | Rationale |
| --- | --- | --- | --- |
| R1 | LOW | Session-end/startup `remove_file` for the temp WAV (symlink/predictable-path hardening) | temp_dir is per-user (0700 on macOS) — a same-uid attacker already owns the session; moot anyway after finding 10 rejected the fixed path |
| R2 | LOW | Pacer measures from push time, not playback start — reword the "≤3 buffers ≈ tens of seconds" claim | Accepted as a wording fix in phase-02's safety analysis (cap bounds pacer timers; cpal's own capacity 24 is the hard bound) rather than a design change |

## Non-issues verified by reviewers (no action)

- Burst deadlock: `push` is sync and never awaits — a stalled consumer cannot
  block the translation worker; backpressure lands on the existing
  `translation_tx` bound.
- Memory: worst case ≈ 2.2 MB PCM lookahead + ≤12 text requests — bounded.
- Translated-text exposure: utterance ids in error copy are UUIDs; no text in
  logs/events; `TtsRequest` deliberately gets no `impl Debug`.
- Meter is display-only (`MainApp.tsx:504-506`, `transcript.ts:353-357`) — no
  hidden UX dependency on `translated-audio-level`.
- `set_pipeline_status_if_active` generation/status gate confirmed at
  `session.rs:525`.
