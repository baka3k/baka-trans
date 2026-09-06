# Local TTS Speech Backlog Optimization — 2026-09-06

## Context

Continuous local speech sessions reliably hit the sticky
`local_tts_backlog_full` banner after roughly 1-2 minutes ("Local speech is
falling behind…"). Root cause: the TTS worker serialized synthesis with
playback — each sentence cost `synthesis_time + playback_time` while a
speaking user produces one sentence per `playback_time`, so every sentence
added permanent backlog until the 4-slot mpsc overflowed and the newest
sentence was dropped (`src-tauri/src/ai/local_worker.rs`, TTS worker cycle).
Plan: `plans/260906-2338-local-tts-backlog-optimization/plan.md` (red-team
verdict: GO after fixes).

## Change

- **Speech queue with coalescing** (`src-tauri/src/ai/speech_queue.rs`, new):
  the 4-slot mpsc became a shared bounded `SpeechQueue` (capacity 12). On
  overflow the new sentence merges into the queued tail (single-space
  separator, `MAX_COALESCED_CHARS` = 600 budget counting the separator) so
  content is preserved silently; only a true shed — coalesce bound exceeded or
  queue front older than `MAX_QUEUE_AGE_MS` = 10 s — drops the OLDEST request
  and emits the reworded `local_tts_backlog_full` naming the skipped utterance
  id. Strict FIFO pops plus tail-merge keep speech order. Explicit `close()`
  replaces channel-close semantics; a drop guard closes the queue on
  early-return error paths so the TTS worker can never park forever. The
  `local_tts_worker_closed` diagnostic is gone (no shared-queue equivalent; a
  dead TTS task still surfaces via the drain join error).
- **Pipelined playback** (`src-tauri/src/ai/local_worker.rs`): the serialized
  playback sleep moved into per-buffer pacer tasks, so synthesis overlaps
  playback and per-sentence throughput becomes `max(S, P)` instead of `S + P`.
  A lookahead cap (`MAX_BUFFERS_AHEAD` = 3, 20 ms poll) keeps the shared
  24-slot cpal channel far from full. `PipelineActivity` now tracks
  `synthesizing` + `playback_inflight` instead of one speech flag, so the
  stage indicator truthfully alternates synthesizing/speaking. Pacer emits are
  generation-gated (the `local-pipeline-stage` event is emitted
  unconditionally, so the gate is load-bearing); Full/Disconnected playback
  errors are suppressed during the stop race (playback teardown lands before
  `RealtimeControl::Stop`), and the inflight counter only increments on
  successful sends.
- **Windows synthesizer cache** (`src-tauri/src/tts.rs`): a process-global
  `OnceLock<Mutex<Option<CachedSynthesizer>>>` keeps one configured
  `SpeechSynthesizer`; per-sentence WinRT activation and the full `AllVoices()`
  scan are gone (voice re-applied only when the requested id changes; rate and
  volume re-applied every call). An `in_flight` guard makes concurrent
  acquisitions fall back to a throwaway instance rather than share the cached
  one with an orphaned (uncancellable) WinRT synthesis. macOS stays on
  per-call UUID temp WAVs — the fixed-path idea was rejected in red-team
  review because an orphaned `say` can outlive the worker and race the file.
  VieNeu is unchanged (server-side synthesis).

## Impact

Impact level: medium-high. The common failure (banner + dropped sentence after
~1-2 min of continuous speech) is structurally removed, and residual overload
degrades by merging speech instead of deleting it. Observable differences:
the banner now fires only on true drops and names the skipped utterance; the
stage indicator alternates synthesizing/speaking; after pause, up to 3 already
synthesized sentences finish playing (amended umbrella contract, accepted);
a configured voice that is uninstalled mid-cache keeps speaking until settings
re-list voices (accepted; today's behavior failed loudly per sentence).
Automated evidence: 162 Rust tests green (11 new speech-queue tests, activity
matrix + lookahead + counter-drift tests), `cargo fmt --check`, clippy
`-D warnings`, `npm test`, production build. Open follow-ups: manual macOS
soak (≥3 min continuous speech, zero backlog errors), and running the new
`#[ignore]`d Windows cache smoke test once on a Windows machine
(`cargo test -- --ignored`) to exercise cross-thread WinRT agility.

## Decision

Chose a producer-inspectable shared queue (`Mutex<VecDeque>` + `Notify`,
permit-storing `notify_one` + `notify_waiters` on close) over channel
workarounds because coalescing and drop-oldest require mutating queue
contents, and chose pipelining over any macOS `say` replacement (process spawn
is unavoidable; pipelining hides it). Phases remain revertible in reverse
order (02 then 01) — both rewrite `spawn_tts_worker`.

## References

- plan: `plans/260906-2338-local-tts-backlog-optimization/plan.md`
- commit: `feat(local): coalesce TTS backlog and pipeline speech playback` (2026-09-07)
- red-team review: `plans/260906-2338-local-tts-backlog-optimization/reports/red-team.md`
- speech queue: `src-tauri/src/ai/speech_queue.rs`
- pipelined TTS worker: `src-tauri/src/ai/local_worker.rs`
- Windows synthesizer cache: `src-tauri/src/tts.rs`
- umbrella coordination note: `plans/260716-2033-local-llm-audio-translation/plan.md`
