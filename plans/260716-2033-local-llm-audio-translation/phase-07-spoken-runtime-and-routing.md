# Phase 07: Spoken Local Runtime and Audio Routing

## Context

The local worker already performs segmentation, Whisper inference, ordered Gemma translation, and transcript snapshots. The session currently skips translated playback and output validation for local mode. This phase connects translated text to TTS and the existing playback sink.

## Requirements

- Require a selected translated output for local spoken mode.
- Create local playback at 24 kHz and pass its sender into the local runtime.
- Preserve transcript ordering and add ordered speech output.
- Keep capture and inference responsive during TTS.
- Emit existing speaking/audio-level state where possible.
- Cancel queued or in-flight speech on stop, mode change, shutdown, or stale generation.

## Architecture

Add a bounded TTS request queue after successful Gemma translation:

```text
translated snapshot final
  -> TTS request { utteranceId, text }
  -> one ordered synthesis worker
  -> PCM chunks
  -> current PlaybackRuntime sender
```

The text result and audio result have separate failure states. TTS failure must not turn a valid final translation into an error or duplicate its transcript card.

## Related Files

- `src-tauri/src/ai/local_whisper_ollama.rs`
- `src-tauri/src/ai.rs`
- `src-tauri/src/session.rs`
- `src-tauri/src/audio.rs`
- `src-tauri/src/models.rs`
- `src/app/MainApp.tsx` and local workspace component
- `src/transcript.ts` only if speech metadata requires a non-text field
- runtime and UI tests

## Implementation Steps

1. Remove local text-only bypasses from backend output validation and playback creation.
2. Start the existing playback runtime at 24 kHz for local mode and reuse output device/channel conflict rules.
3. Pass playback and TTS dependencies into the local runtime without changing Google/OpenAI signatures.
4. Add a bounded ordered TTS queue with explicit backlog behavior.
5. Synthesize off the async event loop and stream bounded PCM chunks to playback.
6. Emit `speaking` and `translated-audio-level` through existing contracts, or add one local-stage event if needed without changing cloud semantics.
7. Keep translation final on TTS failure and expose Retry speech for that utterance only.
8. Make pause stop new intake while allowing the current sentence to finish.
9. Make stop/mode change/shutdown immediately cancel synthesis, clear queued requests, and drop playback.
10. Update local readiness, audio rows, output selectors, channel selectors, routing warnings, and empty states to reflect spoken output.
11. Add concurrency tests for order, queue saturation, cancellation, stale generations, and playback failure.

## Todo

- [ ] Local Start requires output, voice, Whisper, and Gemma readiness.
- [ ] Spoken order matches transcript order.
- [ ] Capture meters continue while TTS is busy.
- [ ] Stop produces no late speech.
- [ ] TTS failure preserves translated text.
- [ ] Local output device/channel controls behave like cloud controls.

## Risks

- Blocking sends to CPAL can stall the translation worker. Keep playback delivery in its own bounded worker.
- Audio can outlive the transcript session if cancellation is incomplete. Check generation before synthesis, before enqueue, and during chunk delivery.
- Reusing output-monitor conflict rules can expose previously hidden invalid profiles. Show an actionable warning and preserve saved values.

## Success Criteria

- A Japanese fixture produces one Japanese/Vietnamese transcript pair and one ordered Vietnamese speech output.
- Selected non-default headset and left/right routing are honored.
- Slow synthesis does not stop `audio-level` events or freeze the UI.
- Stop during synthesis and stop during playback both prevent further audible output.
- Google and OpenAI runtime tests pass without changed payloads or event behavior.
