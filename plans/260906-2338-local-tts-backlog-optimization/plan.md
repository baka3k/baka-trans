---
title: "Local TTS Speech Backlog Optimization (Coalesce, Pipelined Playback, Cached Synthesizer)"
status: implemented baseline (2026-09-07; macOS soak + Windows ignored-test run pending)
priority: P2
effort: 12h
tags: [backend, refactor, tech-debt, audio]
blockedBy: []
blocks: []
created: 2026-09-06
mode: hi-plan --full
---

# Local TTS Speech Backlog Optimization

## Overview

Eliminate the recurring `local_tts_backlog_full` error — "Local speech is falling
behind. The translated text was kept, but this sentence will not be spoken." —
from the local Whisper → translate → TTS pipeline by removing the structural
throughput deficit in the TTS stage and by degrading gracefully instead of
dropping speech when backlog still builds.

Root cause (measured in code, `src-tauri/src/ai/local_worker.rs:559-652`): the
TTS worker serializes synthesis with playback. Its per-sentence cycle is
`synthesis_time + playback_time`, while a continuously speaking user produces
sentences at `1 per playback_time`. Every sentence therefore adds
`synthesis_time` of permanent backlog; the sleep never over-waits to recover
it, and the bounded queue (`TTS_QUEUE_CAPACITY = 4`) overflows after roughly
`4 × P / S` sentences of continuous speech (≈ 1-2 min with macOS `say`, far
sooner with VieNeu neural TTS). When `tts_tx.try_send` returns `Full`
(`local_worker.rs:504-519`), the sentence is dropped with the error above.

```text
Today (serialized):
  translate → [S synthesis][P playback sleep] → next sentence
  deficit per sentence = S  (never recovered)  → queue(4) overflows → speech dropped

Target (pipelined):
  translate → [S][S][S] ...        synthesis runs ahead (bounded by MAX_BUFFERS_AHEAD)
             cpal plays [P][P][P]  playback drains in realtime on its own thread
  deficit per sentence = 0 when S < P; overflow degrades by COALESCING, not dropping
```

## Scope (user-selected optimizations 1-5)

1. Decouple the playback sleep from the TTS worker so synthesis overlaps
   playback (phase 02).
2. Coalesce queued requests when the speech queue is full instead of dropping
   content (phase 01).
3. Raise the TTS queue capacity from 4 to 12 (phase 01).
4. Reduce per-call synthesis overhead: cache the Windows `SpeechSynthesizer`
   (including an orphaned-synthesis guard) (phase 03). The earlier idea of
   reusing one fixed macOS temp file was REJECTED in red-team review —
   per-call UUID temp files stay (see `reports/red-team.md` finding 10).
5. When dropping is still unavoidable (capacity stall past the coalesce
   bound, or queue age past `MAX_QUEUE_AGE_MS`), drop the OLDEST queued
   speech, keeping order and the freshest content, with an explicit error
   (phase 01).

## Cross-Plan Coordination

- `plans/260716-2033-local-llm-audio-translation` (status: `in_progress`) owns
  the local spoken pipeline, including the bounded-TTS-queue invariant from its
  phase 06/07 ("Serialize translated utterances into the TTS queue to preserve
  speech order. Bound the queue and never grow memory indefinitely."). This
  plan preserves both invariants — order is preserved by tail-merge coalescing
  and FIFO pops; memory stays bounded by `SPEECH_QUEUE_CAPACITY` and
  `MAX_COALESCED_CHARS` — and records a dated coordination note in that plan's
  context scan.
- No `blockedBy`: the pipeline this plan modifies (TranslationJob → TtsRequest
  workers, added in commit `01e0673`) is complete on `main`; `git status` is
  clean.
- Touches no cloud (`GoogleLiveTranslate`, `OpenaiRealtime`) code, no Hy-MT2
  sidecar code, and no shared capture path. `src-tauri/src/audio.rs` playback
  internals are read-only dependencies of this plan (not modified).

## Scope Challenge Decisions

1. **Keep `tokio::sync::mpsc` for the speech queue, or replace with a shared
   deque?** Replace with `Arc<Mutex<VecDeque<TtsRequest>>>` + `Notify`
   (`SpeechQueue`). Coalescing and drop-oldest require the producer to inspect
   and mutate queue contents, which mpsc cannot do. Channel-close semantics are
   replaced by an explicit `closed` flag + `close()`; the consumer
   (`pop().await`) exits on `closed && empty`, and the existing
   `drain_worker` cancel-then-abort ladder keeps working unchanged because
   worker exit is still driven by `is_worker_active` + loop end. No `Notify`
   exists in the codebase yet; this is a deliberate, isolated first use.
2. **Does decoupling playback introduce a new failure — the cpal playback
   channel (capacity 24, `audio.rs:359`) filling up?** Yes, if the worker
   prefetches unboundedly. Mitigated with a lookahead cap: before pushing a
   synthesized buffer, the worker waits while `playback_inflight >=
   MAX_BUFFERS_AHEAD (3)`. A pacer task finishing playback decrements the
   counter; the worker re-checks on a 20 ms poll (matches the existing poll
   idiom in `tts.rs:686-702`). This also caps stale-buffer memory (≤3 buffers
   ahead ≈ tens of seconds of PCM at most).
3. **Coalesce direction — head-merge pushed to back, or merge into the queue
   tail?** Merge into the tail (the most recent queued request absorbs the new
   sentence). Head-merge would reorder speech (`[s2,s3,s4,s1+s5]`); tail-merge
   keeps strict FIFO speech order (`[s1,s2,s3,s4+s5]`) and matches how
   consecutive sentences are naturally spoken. `utterance_id` of a merged
   request keeps the tail request's id (diagnostics only).
4. **When is `local_tts_backlog_full` still emitted?** On any true
   drop-oldest: (a) queue full AND the tail merge would exceed
   `MAX_COALESCED_CHARS` (stalled consumer), or (b) the queue front is older
   than `MAX_QUEUE_AGE_MS` = 10 s. The age bound exists because coalescing
   alone would mask a stalled consumer as a silent 30-40 s speech lag with
   zero errors — for live translation, stale speech is a failure even when
   lossless (red-team finding). Coalescing itself emits no error — content
   is preserved, and the sticky frontend error banner
   (`MainApp.tsx:2398-2402`, cleared only by user action) must not fire for
   a lossless outcome.

## Current-State Evidence

| Piece | Location |
| --- | --- |
| Error emission on `tts_tx.try_send` Full | `src-tauri/src/ai/local_worker.rs:504-519` |
| TTS queue `mpsc::channel(TTS_QUEUE_CAPACITY=4)` | `local_worker.rs:30, 165` |
| TTS worker: synthesize → try_send → `sleep(playback_ms)` → level reset | `local_worker.rs:573-645` (sleep at `:620`) |
| `PipelineActivity` single atomics (`SPEECH_SYNTHESIZING`/`SPEECH_PLAYING`), priority logic | `local_worker.rs:81-91, 665-676` |
| Status writes are generation-gated | `src-tauri/src/session.rs:518-535` (`set_pipeline_status_if_active`) |
| Playback: cpal callback drains `sync_channel(24)` via `try_recv`, underrun = silence; stop kills audio instantly (`PlaybackRuntime::Drop`) | `src-tauri/src/audio.rs:353-396, 599-709, 74-81` |
| Windows: `SpeechSynthesizer::new()` + full `AllVoices()` enumeration per sentence | `src-tauri/src/tts.rs:448-498` |
| macOS: `say` writes WAV to `temp_dir/uuid.wav` then reads it back (`say -o -` stdout verified NOT working on this machine) | `tts.rs:664-708` |
| Process-global cache precedent `OnceLock<Mutex<Option<T>>>` | `src-tauri/src/llm.rs:20`, `src-tauri/src/security.rs:8-9` |
| Windows `SpeechSynthesizer` is `Send + Sync` (windows 0.61 registry) | windows-0.61.3 `Media/SpeechSynthesis/mod.rs:376-377` |
| Frontend: `translated-audio-level` → meter, `local-pipeline-stage` → stage label, `app-error` → sticky banner | `src/app/MainApp.tsx:647, 656, 698, 2398-2402` |
| CI gates: `npm test`, `npm run build`, `cargo test`, `cargo fmt --check` (macos+windows) | `.github/workflows/desktop.yml`, `scripts/release-macos.sh:173-184` |

## Architecture

One new module + two focused rewrites of existing functions. No frontend
changes, no config schema change (`LocalTranslationConfig` untouched), no new
events.

### Phase 01 — `SpeechQueue` (items 2, 3, 5)

```text
src-tauri/src/ai/speech_queue.rs   (new)
  SPEECH_QUEUE_CAPACITY: usize = 12
  MAX_COALESCED_CHARS: usize  = 600   // ≈ 30 s of speech; bounds merged text

  pub(crate) struct SpeechQueue { inner: Mutex<State>, notify: Notify }
  struct State { queue: VecDeque<TtsRequest>, closed: bool }

  enum PushOutcome { Queued, Coalesced, DroppedOldest { dropped_id: String } }

  impl SpeechQueue {
      fn new(capacity) -> Self
      fn push(&self, req: TtsRequest) -> PushOutcome   // sync, no await under lock
        loop:
          if closed                -> DroppedOldest { req.id }   // shutdown, no error noise
          if queue.len() < capacity-> push_back; return Queued
          tail = queue.back_mut()
          if tail.chars + req.chars <= MAX_COALESCED_CHARS
                                   -> tail.text += " " + req.text; return Coalesced
          else                     -> dropped = queue.pop_front(); continue loop (retry push)
      async fn pop(&self) -> Option<TtsRequest>
        loop: lock { if let Some(r) = queue.pop_front() -> return Some(r)
                     if closed -> return None }
              notify.notified().await
      fn close(&self)                  // closed = true; notify_waiters
  }
```

Producer (`SentenceTranslator`, `local_worker.rs:504-529`): replace the
`try_send` match with `speech_queue.push(...)`; emit the existing
`local_tts_backlog_full` app-error only on a live `DroppedOldest` (message
updated to cover both shed reasons — capacity stall and the 10 s staleness
bound — and to name the skipped utterance id, as the playback path already
does at `local_worker.rs:604-610`). `Coalesced` is silent. The
`local_tts_worker_closed` diagnostic (`local_worker.rs:518-528`) has no
shared-queue equivalent and is removed; a dead TTS task still surfaces via
`drain_worker`'s join error (`local_worker.rs:1206`).

Lifecycle (`run_local_translation`, `local_worker.rs:159-311`): replace
`let (tts_tx, tts_rx) = mpsc::channel(TTS_QUEUE_CAPACITY)` with
`Arc<SpeechQueue>`; the in-loop stop paths keep dropping
`utterance_tx`/`translation_tx` unchanged, and `speech_queue.close()` runs
between `drain_worker(&mut translator)` and `drain_worker(&mut tts_worker)`
on all three return paths — mirroring mpsc ownership (`tts_tx` lives inside
`SentenceTranslator`, so the channel only closes when the translator task
exits). This preserves both real stop behaviors (red-team corrected):
Stop sets `cancellation` first and discards queued speech as today; the
pause/natural-end path drains the translator first, so late-pushed
sentences stay in the open queue and are spoken during the TTS drain, as
today. `TTS_QUEUE_CAPACITY` const is removed.

### Phase 02 — Pipelined playback (item 1)

```text
PipelineActivity (local_worker.rs:81-91) becomes:
  translation_stage : AtomicU8      // unchanged
  synthesizing      : AtomicBool    // TTS worker inside tts::synthesize
  playback_inflight : AtomicUsize   // live pacer tasks
  MAX_BUFFERS_AHEAD: usize = 3

pipeline_activity_state():
  playback_inflight > 0 -> (Speaking, "speaking")
  synthesizing          -> (Speaking, "synthesizing")
  else translation_stage as today

TTS worker loop (spawn_tts_worker, local_worker.rs:573-645):
  activity.synthesizing = true; settle
  audio = tts::synthesize(...)
  activity.synthesizing = false; settle
  // lookahead cap — replaces the serialized sleep
  while playback_inflight.load() >= MAX_BUFFERS_AHEAD && is_worker_active(..):
      tokio::time::sleep(20ms).await
  playback_tx.try_send(audio)                      // unchanged, incl. Full/Disconnected errors
  playback_inflight += 1
  activity/speech_stage -> speaking; settle
  tokio::spawn(async move {                        // pacer: owns playback-wait + level reset
      tokio::time::sleep(playback_ms).await
      if !is_generation_active(generation, &active_generation) { playback_inflight -= 1; return; }
      emit translated-audio-level reset
      playback_inflight -= 1
      settle_pipeline_activity(...)
  })
```

Safety notes (researched): `set_pipeline_status_if_active` already ignores
stale generations (`session.rs:518-535`), so pacer status writes after stop are
inert; the pacer's level-reset emit is additionally generation-gated (a new
session must not see its predecessor's meter reset). A pacer sleeping through
a stop simply fails nothing — the `SyncSender` clone dies with the runtime and
the cpal thread is killed by `PlaybackRuntime::Drop` (`audio.rs:74-81`), so
late audio cannot play (existing guarantee). The 20 ms lookahead poll only
runs while the worker is active, so drain still terminates.

### Phase 03 — Synthesis overhead (item 4)

Windows (`tts.rs` platform module): process-global
`static CACHED_SYNTHESIZER: OnceLock<Mutex<Option<CachedSynthesizer>>>`
(pattern precedent `llm.rs:20`). `CachedSynthesizer { synthesizer,
configured_voice_id }`. Per sentence: lock (std `Mutex`, no await held);
if `configured_voice_id != config.voice_id`, re-apply `SetVoice` (voice list
enumeration only happens on change — today it runs on every sentence at
`tts.rs:456-498`). Poisoning → recreate instance. `list_voices()` keeps using
its own instance (settings action, not per-sentence).

macOS (`tts.rs:664-708`): `say -o -` to stdout verified NOT supported (creates
a literal `-.wav`; `/dev/stdout` fails), so the temp file stays — but it
becomes a single reused path `temp_dir/baka-trans-tts.wav` instead of a fresh
UUID per sentence (kills temp-dir churn/orphans; sequential worker makes the
shared path safe). The dominant macOS cost (`say` process spawn) is
unavoidable and is covered by phase 02's pipelining instead.

VieNeu: no code change (server-side synthesis; warm-up is operational). Its
high S is exactly what phases 01+02 absorb.

### Phase 04 — Tests, docs, coordination log

Unit tests (pure-logic, in-module, per repo convention), docs updates for the
changed error semantics, dated coordination notes in
`plans/260716-2033-local-llm-audio-translation/plan.md`, and a
`docs/logs/` implementation log.

## Phases

| Phase | Document | Outcome |
| --- | --- | --- |
| 01 | [phase-01-speech-queue.md](phase-01-speech-queue.md) | `SpeechQueue` (capacity 12, tail-merge coalescing, drop-oldest fallback, explicit close) replaces the 4-slot mpsc |
| 02 | [phase-02-pipelined-playback.md](phase-02-pipelined-playback.md) | Synthesis overlaps playback; pacer tasks + inflight counter + lookahead cap; activity model reworked |
| 03 | [phase-03-synthesis-overhead.md](phase-03-synthesis-overhead.md) | Cached Windows synthesizer (no per-sentence COM init/voice enumeration); reused macOS temp path |
| 04 | [phase-04-regression-tests-and-docs.md](phase-04-regression-tests-and-docs.md) | Full test matrix, docs/coordination updates, release gates |

## File Impact Map

| Area | Files |
| --- | --- |
| Speech queue (new) | `src-tauri/src/ai/speech_queue.rs` |
| Pipeline rewrite | `src-tauri/src/ai/local_worker.rs` |
| Synthesis caching | `src-tauri/src/tts.rs` |
| Module registration | `src-tauri/src/ai.rs` (`mod speech_queue;`) |
| Docs / coordination | `docs/RELIABILITY.md` (if it references the old error), `docs/logs/2026-09-06-local-tts-backlog-optimization.md`, `plans/260716-2033-local-llm-audio-translation/plan.md` |
| Tests | `speech_queue.rs` tests, `local_worker.rs` tests (activity model), `tts.rs` tests |

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Decoupling floods the 24-slot cpal channel → new "output overloaded" errors | `MAX_BUFFERS_AHEAD = 3` lookahead cap gates every push; capacity 24 ≫ 3; increments/spawns only on the successful-send branch (no phantom slots) |
| Stop-race error spam: playback is dropped before `RealtimeControl::Stop` arrives and pipelining would free-run failed sends | Full/Disconnected error emits gated on `is_worker_active` + queue-not-closed (red-team finding) |
| Pacer tasks outlive the session and emit stale events/meter resets | Every pacer action after sleep is generation-gated — load-bearing for `local-pipeline-stage`, which `emit_local_pipeline_stage` emits unconditionally; the session-status path is gated separately in `session.rs` |
| Two overlapping playbacks flicker status/level | `playback_inflight` counter (not a bool) drives the speaking state; per-pacer level resets are cosmetic and bounded by the lookahead cap (≤3) |
| Coalescing masks a stalled consumer as a silent 30-40 s speech lag | `MAX_QUEUE_AGE_MS = 10 s` sheds stale fronts through the same drop-oldest + error path; speech never lags silently beyond the bound |
| Coalesced text grows unbounded during a stalled consumer | `MAX_COALESCED_CHARS = 600` (separator counted); beyond it the queue drops oldest and emits the (updated) backlog error |
| Speech order breaks under coalescing | Tail-merge only; pops are strict FIFO; order is `…, tail, tail+new` |
| Cached Windows synthesizer plays a stale voice after settings change | Voice re-applied whenever `configured_voice_id != config.voice_id`; per-session config objects carry the voice id; voice-uninstalled-mid-cache silently keeps the old voice (accepted, documented) |
| Orphaned in-flight synthesis reuses the cached WinRT instance concurrently | `in_flight` flag on the cache entry; concurrent acquisitions fall back to a throwaway instance (red-team finding) |
| Stop semantics regress (queued speech plays after stop / or drain hangs) | Stop path keeps cancellation-first discard (as today); pause path closes the queue only after the translator drains; `close()` signals with permit-storing `notify_one` (lost-wakeup-proof, red-team finding); `drain_worker` ladder unchanged; playback killed by `PlaybackRuntime::Drop` |
| Concurrent-phase reverts conflict | Phases 01 and 02 both rewrite `spawn_tts_worker`: revert in reverse order (02 then 01) only |

## Out of Scope

- Streaming/partial TTS synthesis, mid-utterance cancellation (explicitly
  deferred by the umbrella plan).
- Dropping or shortening text at the translation stage; retry/backoff around
  the translation client.
- VieNeu server warm-up or inference speed; macOS `say` replacement.
- Cloud pipelines (`google_live.rs`, `openai_realtime.rs`) and their playback
  pacing.
- Frontend changes (the existing sticky banner and meter behavior are
  preserved; coalescing simply triggers the banner far less often).
- `tts_rate` UI/config changes (user-facing mitigation already exists).

## Success Criteria

- A 3-minute continuous-speech macOS session (System voice) with synthesis at
  realistic speeds completes with **zero** `local_tts_backlog_full` errors
  and **zero** `local_tts_playback_error` (Full) errors — the decidable
  criterion (there is no per-utterance "spoken" log today; optionally add a
  debug log on pacer completion carrying the utterance id and coalesce count
  to make spoken-count auditing possible).
- With an artificially slowed synthesizer (debug delay) forcing overflow:
  speech never lags silently beyond `MAX_QUEUE_AGE_MS`; coalescing keeps
  speech complete and ordered while backlog is fresh; past the bounds the
  drop-oldest error names the skipped utterance.
- Stopping or pausing mid-playback still cuts audio immediately; no stale
  `translated-audio-level` reset or pipeline-stage event arrives after a new
  session starts (generation guard); the stop-race emits no
  disconnected-output error spam.
- Windows CI still passes with the cached synthesizer; switching voices in
  settings takes effect on the next sentence without an app restart; the
  ignored cross-thread WinRT smoke test is run once on a Windows machine.
- `npm test`, `npm run build`, `cargo test`, `cargo fmt --check` pass on
  macOS and Windows CI; `cargo clippy --all-targets -- -D warnings` passes
  locally.

## Review and Validation

- Adversarial review: [reports/red-team.md](reports/red-team.md) — verdict
  **GO after fixes**; 15 deduplicated findings, 13 accepted and folded into
  the phase docs (notably: `close()` lost-wakeup race, truthful stop/drain
  semantics, silent-lag age bound, stop-race error gating, macOS fixed-temp
  race → rejected the fixed path), 2 rejected with rationale.

## Implementation Handoff

Suggested command:

```text
/hi-craft plans/260906-2338-local-tts-backlog-optimization/plan.md --full
```
