# Phase 07 - Manual Utterance Boundary Fallback

Status: planned
Depends on: phase 03, phase 04, phase 06

## Goal

Add a user-controlled fallback button that forces the current captured speech segment to be translated when the remote speaker talks continuously and automatic sentence or turn detection does not produce a translation quickly enough.

This is not the primary realtime mode. It is a meeting-pressure fallback for the failure case where the app is receiving audio, but the API is waiting too long for a natural pause before producing translated text/audio.

## Scope Challenge

Question 1: Should this replace automatic turn detection?
Decision: No. Keep automatic realtime translation as the normal path. The button only forces a boundary when automatic behavior stalls.

Question 2: Should the UI expose many VAD tuning knobs?
Decision: Not initially. Add one direct command first, then add advanced turn-detection settings only if manual commit alone is insufficient in live tests.

Question 3: Should pressing the button stop capture?
Decision: No. Capture continues. The button should mark a boundary for the backend/API while the meeting keeps flowing.

## API Notes

- Realtime WebSocket audio is streamed with `input_audio_buffer.append`.
- `input_audio_buffer.commit` commits the current buffer into a conversation item and clears the buffer.
- With server VAD enabled, the server normally commits automatically.
- With VAD disabled, the client must commit manually and may need to trigger response generation explicitly.
- The implementation must verify the exact translation-session event names because the current code uses translation-specific `session.input_audio_buffer.append` and transcript/audio events rather than the generic conversation event names.

## Architecture

```text
React "Translate now" button / shortcut
  -> Tauri command: force_translate_boundary
  -> AppState boundary_tx
  -> realtime translation task select! branch
  -> WebSocket client event:
       input_audio_buffer.commit or translation-session equivalent
       optional response.create if required by turn-detection mode
  -> backend emits boundary status event
  -> UI shows committed / ignored / error feedback
```

Key rule: the UI command must not write directly to audio queues. It should signal the realtime task that owns the WebSocket writer.

## Data Model

Add backend/frontend DTOs:

- `ManualBoundaryMode`
  - `auto_vad_with_manual_commit`
  - `manual_turn_detection`
- `ManualBoundaryRequest`
  - `reason`: `user_button` or `keyboard_shortcut`
  - `requested_at_ms`
- `ManualBoundaryStatus`
  - `idle`
  - `pending`
  - `committed`
  - `ignored_empty_buffer`
  - `rate_limited`
  - `error`
- `ManualBoundaryEvent`
  - `status`
  - `message`
  - `committed_at_ms`

Extend `SessionConfig` later only if runtime testing shows the user needs a persistent mode toggle. The first implementation can keep automatic VAD and add manual commit as an active-session command.

## Implementation Tasks

1. Add a realtime control channel.
   - Add a `tokio::sync::mpsc::Sender<RealtimeControl>` or similar field to `AppState`.
   - Store it when `start_pipeline` spawns `run_realtime_translation`.
   - Clear it in `finish_pipeline`.
   - Keep the channel separate from captured audio samples.

2. Add backend command plumbing.
   - Add `force_translate_boundary(app, state)` to `commands.rs`.
   - Add `AppState::force_translate_boundary`.
   - Return clear errors when no active session exists or status is not `listening`, `translating`, or `speaking`.
   - Debounce rapid presses, for example ignore requests within 800-1200 ms while one commit is pending.

3. Update the realtime loop.
   - Add a `tokio::select!` branch for realtime control messages.
   - On manual boundary request, send the appropriate commit event to the WebSocket writer.
   - Track whether enough audio has been appended since the last commit to avoid predictable empty-buffer errors.
   - If the active mode disables automatic responses, send the required response trigger after commit.
   - Parse commit/empty-buffer/error server events and emit a UI status event.
   - Do not block audio capture or playback while waiting for commit acknowledgement.

4. Add UI control.
   - Add a prominent compact button in the Session panel, near Start/Pause/Stop.
   - Suggested label: `Translate now`.
   - Suggested icon: use an available lucide action icon.
   - Enable only during active session statuses.
   - Add a keyboard shortcut that is unlikely to collide with OS/app controls, for example `Cmd+Enter`.
   - Show short status feedback: `Boundary sent`, `No buffered speech`, `Still translating`, or error text.
   - Do not add instructional paragraphs inside the app; keep feedback concise.

5. Add metrics and diagnostics.
   - Count manual boundary requests, successful commits, ignored empty commits, API errors, and average time from button press to translated audio/text.
   - Include the counters in logs or a debug-only status structure, not in normal meeting exports.

6. Add a fallback mode only after validation.
   - If manual commit with server VAD is unreliable, add a setting that disables automatic turn detection for the session.
   - In that mode, use the button as the primary commit trigger and send `response.create` if the API requires it.
   - Keep this setting off by default because it makes the user responsible for segmentation.

## Verification

- `npm test`
- `npm run build`
- `cargo fmt --check`
- `cargo check`
- `cargo test`
- Unit tests:
  - `force_translate_boundary` returns an error while idle.
  - active session command reaches the realtime control channel.
  - rapid repeated presses are debounced or rate-limited.
  - empty-buffer server error maps to `ignored_empty_buffer`.
  - transcript reducer does not duplicate text when server VAD and manual commit happen close together.
- Realtime smoke tests:
  - stream a long continuous audio fixture with no silence and press `Translate now`.
  - verify first translated text/audio appears sooner than waiting for automatic turn detection.
  - press the button during silence and verify a nonfatal UI message.
  - press the button repeatedly and verify the session remains healthy.
- Manual meeting test:
  - Teams audio routed through BlackHole.
  - remote speaker or fixture speaks for at least 30-60 seconds without a long pause.
  - user presses `Translate now` and hears/reads a partial translation while capture continues.

## Exit Criteria

- The user can force translation during an active session without stopping capture.
- The UI gives immediate feedback for sent, ignored, and failed boundary requests.
- Manual boundary requests do not leave the WebSocket, capture stream, playback stream, or session status stuck.
- Automatic realtime translation remains the default path.
- The fallback is verified with continuous-speech audio where automatic turn detection would otherwise wait too long.

## Non-Goals

- Replacing the realtime translation path with push-to-talk.
- Exposing all low-level VAD thresholds in the main UI.
- Editing already translated segments.
- Solving speaker diarization.
- Sending manual boundary commands while the session is idle or paused.
