# Phase 13: HY-MT Pipeline Integration and Regression

## Context

HY-MT is installable and selectable, but the ordered local worker still constructs Ollama directly. This phase connects the engine dispatcher to live Whisper → translation → transcript/TTS flow while preserving every queue, revision, routing, and cancellation guarantee.

## Requirements

- Prewarm the selected translation engine before capture starts; a failed HY load must not begin an unusable session.
- Replace the direct `OllamaClient` field/call with the engine dispatcher only; keep segmentation, queue capacity, transcript revisions, TTS queue, playback, levels, and cloud branches unchanged.
- Exactly one translation result/error snapshot exists per utterance ID.
- Stop/pause/generation invalidation cancels HY generation and prevents late transcript/TTS mutation.
- Engine failure does not silently run the utterance through the other engine.
- Text remains final if subsequent TTS fails, matching current semantics.
- Real-model smoke/performance tests are opt-in and never make normal CI download 4 GB.

## Architecture

```text
Whisper source snapshot
  -> LocalTranslationEngineClient.translate
     -> Ollama, or
     -> HyMtManager one-flight request
  -> same final/error snapshot
  -> same bounded TTS queue/playback
```

## Related Files

- `src-tauri/src/ai/local_whisper_ollama.rs` (rename to `local_whisper.rs` only after tests)
- `src-tauri/src/ai.rs`, `src-tauri/src/session.rs`
- `src-tauri/src/local_translation.rs`, `src-tauri/src/hy_mt.rs`
- Existing local worker/session/transcript tests and new opt-in HY smoke fixtures.
- Frontend activity/error copy only where engine-neutral wording is required.

## Implementation Steps

1. Construct/prewarm the selected engine in session startup after config/language validation and before audio capture.
2. Pass an engine client/dispatcher into `TranslationWorker` and replace only the translation call/result mapping.
3. Propagate source/target language names, max output tokens, cancellation generation, and per-request deadline to HY-MT.
4. Wire stop, pause, worker drain, session-generation changes, and app shutdown to cancel; terminate/restart the sidecar after the cancellation grace period if necessary.
5. Preserve pending/final/error snapshot revisions and skip TTS on translation errors/cancelled/stale generations.
6. Keep queue capacity at four and add metrics/errors for engine latency, request timeout, queue drop, restart, device, and model revision without logging transcript text.
7. Add fake-engine tests for order, errors, timeout, cancellation, stale response, restart, queue pressure, and TTS behavior.
8. Add ignored real-model tests for JA→VI probe, repeated warm inference, offline inference, cancellation, and a sustained Whisper → HY-MT → TTS session.
9. Run cloud provider, summaries, overlays, export, audio routing, and existing Ollama regression suites unchanged.

## Todo

- [ ] Session prewarm gates capture correctly.
- [ ] One result/error snapshot per utterance is enforced.
- [ ] Stop/pause/cancel produces no late transcript or audio.
- [ ] Queue and TTS semantics remain unchanged.
- [ ] Ollama and cloud regressions pass.
- [ ] Opt-in real-model offline/sustained tests pass on M5.

## Risks

- Prewarm on the UI command path can appear frozen. Emit starting/loading progress and keep blocking work off Tokio/React.
- Killing a sidecar can make the next session cold. Report restart/prewarm state and never accept a request before verified Ready.
- A sidecar result can arrive after generation invalidation. Check both request ID and session generation before any store/event/TTS mutation.
- Broad module renaming can obscure the functional change. Rename separately after behavioral tests are green.

## Success Criteria

- Local Whisper can run either Ollama or HY-MT through the same ordered worker with identical transcript/TTS semantics.
- A 30-minute M5 session records zero translation-queue drops, crashes, duplicate snapshots, late mutations, or late playback.
- Stopping during generation reaches Idle within the bounded drain policy and leaves no active HY request/child leak.
- Existing Ollama, Google, OpenAI, TTS, audio-routing, transcript, export, summary, and overlay tests pass.
