# Phase 02: Local Whisper and Ollama Runtime

## Context

The current session orchestrator starts capture, translated-audio playback, and one cloud provider task. The local provider must reuse capture and lifecycle behavior while replacing only the translation stage and omitting cloud audio output.

## Requirements

- Capture PCM16 mono at 16 kHz through the existing `audio.rs` path.
- Segment utterances without blocking or changing the input meter/original monitor behavior.
- Run Whisper off the async executor, translate Japanese text through native Ollama, and keep results ordered.
- Honor pause, resume, manual boundary, stop, and shutdown.
- Bound memory and surface actionable per-utterance errors.

## Architecture

Use one task graph per session:

```text
existing capture channel
  -> bounded speech segmenter
  -> bounded Whisper worker (spawn_blocking)
  -> ordered Ollama worker
  -> transcript store upsert + event emit
```

Assign an immutable session generation token plus a stable utterance ID and revision. Every store mutation checks the active generation so a late response from a stopped session is ignored.

## Related Files

- `src-tauri/src/audio.rs` (reuse; change only if a small public helper/constant is required)
- new `src-tauri/src/ai/local_whisper_ollama.rs`
- `src-tauri/src/ai.rs`
- `src-tauri/src/session.rs`
- `src-tauri/src/models.rs`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`

## Implementation Steps

1. Add the Whisper binding and confirm CPU builds for supported targets before enabling optional acceleration.
2. Implement a pure/testable PCM segmenter with pre-roll, minimum speech, trailing silence, maximum duration, and explicit flush.
3. Add a model context loader with clear errors for missing, unreadable, incompatible, or failed model initialization.
4. Execute Whisper inference in `spawn_blocking`, force Japanese transcription, normalize whitespace, and reject empty/no-speech output without calling Ollama.
5. Create a transcript item immediately after a valid source transcription and mark it pending.
6. Translate with one ordered native Ollama worker and upsert the same item with Vietnamese text, latency, final status, and incremented revision.
7. Route `RealtimeControl` manual boundaries to segment flush. Define pause as no new inference/translation intake while preserving the current lifecycle semantics.
8. Make `session.rs` provider-aware: 16 kHz capture, no cloud credential, no translated playback, and no translated output validation for local text-only mode.
9. Add bounded-channel overload behavior and cancellation; emit one actionable `app-error` and item error rather than silently dropping established speech.
10. Add unit/integration tests for segmentation edges, ordering, cancellation, manual flush, empty speech, Whisper failure, slow Ollama, Ollama failure, and stopped-session late responses.

## Todo

- [ ] Existing audio runtime is reused at 16 kHz.
- [ ] Whisper never blocks Tokio/Tauri threads.
- [ ] Queues and utterance buffers have explicit bounds.
- [ ] Same utterance ID flows from source to Vietnamese result.
- [ ] Local stop leaves no worker or late event.

## Risks

- Energy-only segmentation may cut Japanese phrases at pauses. Expose conservative defaults and cover manual/maximum boundaries; do not couple segmentation to UI rendering.
- Whisper model initialization can be slow. Report `starting` until ready and load only once per session.
- Sequential translation can lag under sustained speech. Cap utterance size and queue depth, report backlog state, and avoid unbounded parallel calls that reorder results.

## Success Criteria

- A deterministic Japanese PCM fixture becomes source text and then Vietnamese text on the same stored transcript item.
- Audio-level events continue while Whisper and Ollama are busy.
- Existing Google/OpenAI session tests pass without changed output behavior.
- Pause/stop/manual boundary behavior has automated coverage for the local provider.
