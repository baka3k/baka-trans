# Phase 02 — Pipelined Playback (Decouple the Sleep from Synthesis)

Parent: [plan.md](plan.md)

Covers user-selected optimization **1** — the structural fix for the backlog
deficit. Depends on nothing from phase 01 at runtime, but lands after it.
Revert note (red-team finding): phases 01 and 02 both rewrite
`spawn_tts_worker`, so once both land they can only be reverted in reverse
order (02 then 01), not independently.

## Objective

In `spawn_tts_worker` (`src-tauri/src/ai/local_worker.rs:573-645`), the
worker currently sleeps one full playback duration after pushing each buffer
(`:620`), serializing synthesis behind playback. Move the playback wait into
a detached per-buffer **pacer task** so the worker immediately synthesizes
the next sentence, bounded by a small lookahead cap. Per-sentence TTS
throughput becomes `max(S, P)` instead of `S + P`, and during the speaker's
pauses the worker prefetch-drains any residual backlog to zero.

## Rework `PipelineActivity` (`local_worker.rs:81-91`)

Today a single `speech_stage: AtomicU8` (`SPEECH_SYNTHESIZING`/`SPEECH_PLAYING`)
is owned by the worker's sequential cycle. With overlapping synthesis and
playback, one flag cannot represent both. Replace with:

```rust
#[derive(Default)]
struct PipelineActivity {
    translation_stage: AtomicU8,     // unchanged: INACTIVE / TRANSCRIBING / TRANSLATING
    synthesizing: AtomicBool,        // TTS worker is inside tts::synthesize
    playback_inflight: AtomicUsize,  // live pacer tasks (buffers playing or queued ahead)
}

const MAX_BUFFERS_AHEAD: usize = 3;
```

`pipeline_activity_state` (`local_worker.rs:665-676`) becomes:

```text
if playback_inflight > 0   -> (SessionStatus::Speaking, "speaking")
if synthesizing            -> (SessionStatus::Speaking, "synthesizing")
else                       -> translation_stage mapping (unchanged)
```

Speech still outranks translation in the status display, and "synthesizing"
is still surfaced when no audio is currently playing. Update the existing test
`speech_activity_has_priority_over_overlapping_translation`
(`local_worker.rs:1382-1407`) to the new fields, and add a case:
`synthesizing && playback_inflight > 0 → ("speaking")`.
Remove `SPEECH_SYNTHESIZING` / `SPEECH_PLAYING` consts and the
`speech_stage` field.

## TTS worker loop rewrite (`spawn_tts_worker`)

New cycle per request:

1. `activity.synthesizing.store(true)`; `settle_pipeline_activity`.
2. `tts::synthesize(...)` — unchanged, including the
   `local_tts_cancelled` break (`:632-636`) and the error branch (`:638-640`).
3. `activity.synthesizing.store(false)`; settle.
4. **Lookahead cap** — replaces the serialized sleep:
   ```rust
   while playback_inflight.load(Ordering::SeqCst) >= MAX_BUFFERS_AHEAD
       && is_worker_active(generation, &active_generation, &cancellation)
   {
       tokio::time::sleep(Duration::from_millis(20)).await;
   }
   if !is_worker_active(...) { break; }
   ```
   The 20 ms poll matches the existing poll idiom (`tts.rs:686-702`,
   `tts.rs:148`). Without this cap the worker would race ahead and fill the
   shared 24-slot cpal channel, turning the old "falling behind" error into
   the playback overload error at `local_worker.rs:595-610`.
5. `playback_tx.try_send(...)` — error branch CHANGES (red-team finding:
   `stop_session` drops `PlaybackRuntime` at `session.rs:288-291` BEFORE the
   `RealtimeControl::Stop` arrives, and generation is not bumped on stop, so
   `is_worker_active` stays true during the stop race; today the serialized
   sleep throttles failures to 1-2, but after pipelining a 12-deep queue
   would spam the sticky banner with "output disconnected/disconnected"
   errors on nearly every mid-speech stop):
   - Keep the existing `Full`/`Disconnected` error messages, but emit either
     only when `is_worker_active(generation, &active_generation, &cancellation)`
     AND the speech queue is not closed — suppress during the stop race.
   - On the error path, do NOT touch `playback_inflight` and do NOT spawn a
     pacer (a phantom increment here would permanently occupy lookahead
     slots and silently stall all speech after 3 failed sends).
6. Success branch only: `playback_inflight.fetch_add(1)`; settle (state now
   reports "speaking" via the counter).
7. Spawn the **pacer** and continue the loop immediately:

```rust
let pacer_activity = activity.clone();
let pacer_app = app.clone();
let pacer_generation = generation;
let pacer_active = active_generation.clone();
tokio::spawn(async move {
    tokio::time::sleep(Duration::from_millis(playback_ms)).await;
    pacer_activity.playback_inflight.fetch_sub(1, Ordering::SeqCst);
    // Generation gate: a predecessor's pacer must never touch the meter,
    // stage event, or status of a newer session.
    if !is_generation_active(pacer_generation, &pacer_active) {
        return;
    }
    let _ = pacer_app.emit("translated-audio-level", TranslatedAudioLevelEvent {
        sample_count: 0, rms: 0.0, peak: 0.0,
    });
    let _ = settle_pipeline_activity(&pacer_app, pacer_generation, &pacer_activity);
});
```

Decrement **before** the generation check (bookkeeping must always happen;
only the outward-facing emits are gated).

## Safety analysis (verified against researched behavior; red-team corrected)

- **Status after stop:** `set_pipeline_status_if_active`
  (`session.rs:518-535`) gates only the `session-status` write — it does
  NOT gate the `local-pipeline-stage` event, which `emit_local_pipeline_stage`
  (`local_worker.rs:701-704`) emits unconditionally. The pacer's generation
  gate is therefore LOAD-BEARING for the stage event, not defense in depth
  (an earlier draft claimed otherwise; red-team finding — keep the gate and
  treat it as required).
- **Late audio:** a pacer only sleeps and emits an event; it never touches
  audio. Stopping the session drops `PlaybackRuntime`, whose `Drop`
  (`audio.rs:74-81`) stops the cpal thread immediately — the existing
  "stop cuts audio" guarantee is untouched by this phase.
- **Drain termination:** the worker's only new wait is the lookahead poll,
  which exits when `is_worker_active` turns false; `drain_worker`
  (`local_worker.rs:1182-1226`) is unchanged. Detached pacers are NOT
  awaited by drain (they are pure timers ≤ playback duration of ≤3 buffers).
- **Pacer task leak:** bounded by the lookahead cap — at most
  `MAX_BUFFERS_AHEAD` live pacers at any moment plus the one being spawned.
- **Meter semantics:** `translated-audio-level` is emitted per buffer at
  push (unchanged, `:612-613`) and reset per pacer end (moved from the
  worker's post-sleep code at `:621-629`). With overlap, one buffer's reset
  can land while a later buffer is still playing, dipping the meter early —
  cosmetic (verified: `translatedLevel` drives only the meter percent and a
  transcript activity hint, `MainApp.tsx:504-506`, `transcript.ts:353-357`;
  it feeds no pause/UX logic) and bounded by the cap. Do not add refcounting
  for the meter (YAGNI).
- **Cap precision:** the pacer measures from push time, not actual playback
  start, so `MAX_BUFFERS_AHEAD` bounds pacer timers rather than exact
  queued-audio depth; the hard bound remains the cpal channel's own capacity
  24 (`audio.rs:359`), and only the TTS worker sends to it (verified —
  `PlaybackRuntime::sender()` clones are used for translation playback only,
  `session.rs:435`).
- **`SPEECH_PLAYING` display duration:** today "speaking" shows for the full
  `S + P` cycle; after this phase it shows during actual playback windows
  (pacer lifetime) — a more truthful UI, and "synthesizing" fills the gaps.
- **Pause behavior (accepted contract change, documented):** pause stops
  capture/translation but does NOT flush the lookahead, so up to
  `MAX_BUFFERS_AHEAD` (3) already-synthesized sentences play out after
  pause, versus today's ≤1. These are sentences the speaker just finished
  uttering, so content is never from the future; the umbrella contract
  ("current spoken sentence finishes") is amended to "current + up to
  2 already-synthesized sentences finish". The phase-04 matrix asserts this
  amended wording.

## Tests (`local_worker.rs` `mod tests`)

- Update `speech_activity_has_priority_over_overlapping_translation` to the
  new `PipelineActivity` fields (synthesizing bool + inflight counter) with
  the matrix: inflight>0 → speaking; synthesizing-only → synthesizing;
  both → speaking; neither → translation mapping.
- `#[tokio::test]` for the lookahead wait: with a fake
  `playback_inflight` seeded to `MAX_BUFFERS_AHEAD` and a flag flipped by a
  spawned task after ~30 ms, assert the wait loop exits and respects
  cancellation (set `cancellation` mid-wait → exits immediately). Extract
  the wait into a small helper
  `async fn wait_for_playback_slot(activity: &PipelineActivity, ...)` so it
  is directly testable without real synthesis.
- `#[tokio::test]` for the counter-drift guard (red-team finding): drive the
  worker loop against a closed playback sender so every `try_send` fails,
  feed it 5 requests, and assert `playback_inflight` is back to 0 and no
  pacer tasks remain — a phantom increment would wedge all future speech.

Integration (manual, macOS): run a real session; confirm via the app UI that
speech no longer stops after ~1-2 min of continuous talking and that the
stage indicator alternates synthesizing/speaking instead of a single long
"speaking".

## Acceptance

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings` pass.
- The deficit math holds in a manual soak: ≥3 min continuous speech with the
  System voice produces zero backlog errors (with phases 01+02 both landed,
  either alone already removes the common-case error; this phase removes the
  root deficit).
- Stop during playback still cuts audio instantly; no meter reset or stage
  event from a stopped session leaks into a newly started one.
