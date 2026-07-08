# Phase 08 - Source Audio Signal Indicator

Status: planned
Depends on: phase 02, phase 04

## Goal

Show an explicit incoming source-audio signal when data from the selected meeting source reaches the app, so the user can tell that Teams/BlackHole/audio routing is connected before waiting for translation results.

This phase should reuse the current capture path. `src-tauri/src/audio.rs` already emits `audio-level` events with `input_device_id`, `rms`, and `peak`, and `src/App.tsx` already renders a simple peak meter. The work is to turn that raw meter into a reliable connection/signal state.

## Scope Challenge

Question 1: Should this add a new audio capture path?
Decision: No. Reuse the existing `audio-level` event and only extend it if freshness, device matching, or diagnostics require extra metadata.

Question 2: Should silence mean the stream is broken?
Decision: No. Silence with fresh events means the source stream is connected but currently quiet. No recent events means stale or disconnected.

Question 3: Should this indicator depend on OpenAI translation events?
Decision: No. It must prove the audio pipe into the app is flowing even if API translation is delayed, paused, or unavailable.

## Architecture

```text
Selected input device / BlackHole
  -> cpal capture callback
  -> handle_input computes RMS/peak
  -> Tauri event: audio-level
  -> React signal reducer
  -> Audio routing panel:
       signal dot
       compact meter
       text state: Waiting / Receiving / Silent / No recent audio / Error
```

The frontend should ignore `audio-level` events whose `input_device_id` does not match the currently selected meeting source, unless no source is selected during hydration.

## Data Model

Frontend state:

- `level`: current display level, derived from peak or RMS
- `lastAudioLevelAtMs`: timestamp of the most recent matching event
- `sourceSignalState`
  - `waiting`: session not active or no event seen yet
  - `receiving`: recent matching events above signal threshold
  - `silent`: recent matching events below signal threshold
  - `stale`: session active but no matching event in the freshness window
  - `error`: capture error received
- `sourceSignalMessage`: short UI label

Recommended thresholds:

- active signal threshold: peak >= 0.015 or RMS >= 0.008
- silent threshold: fresh event below active threshold
- stale window: no matching event for 1500-2500 ms while session status is `listening`, `translating`, or `speaking`
- decay: reduce displayed meter toward zero when no fresh signal is present

Backend extension only if needed:

- Add `received_at_ms` or `sequence` to `AudioLevelEvent` if frontend `Date.now()` is not enough for reliable freshness.
- Optionally emit a capture-started event when the cpal stream becomes ready, but avoid adding it unless UX needs a distinct "stream opened, waiting for samples" state.

## Implementation Tasks

1. Add source signal state logic.
   - Add a small reducer/helper in the frontend to classify incoming `AudioLevelEvent` values.
   - Track the selected input device and ignore events from stale selections.
   - Add a timer that marks the signal stale when active-session events stop arriving.
   - Reset signal state to `waiting` on session stop, input-device change, and hydrate.

2. Improve the Audio routing UI.
   - Replace the anonymous meter with an icon, live dot, concise label, and stable-width meter.
   - Suggested labels: `Waiting`, `Receiving audio`, `Source silent`, `No recent audio`, `Capture error`.
   - Keep it compact near the Meeting source selector.
   - Do not add instructional paragraphs; use the existing setup checklist for routing guidance.

3. Wire capture errors into the signal state.
   - Existing backend events include `audio_capture_error` through `app-error`.
   - When that code arrives, mark source signal as `error` and keep the normal error banner behavior.
   - Clear error signal when the user restarts the session or changes the input source.

4. Extend types/tests.
   - Add TypeScript helper tests for state transitions and stale timer behavior.
   - If backend metadata is added, update `src-tauri/src/models.rs`, `src/types.ts`, and event parsing together.
   - Keep Rust changes minimal unless tests reveal frontend timestamps are insufficient.

5. Verify with routed audio.
   - Start a session with BlackHole as meeting source.
   - Play fixture/Teams/system audio into BlackHole and verify `Receiving audio`.
   - Stop playback while the capture stream continues and verify `Source silent`.
   - Stop or break capture and verify `No recent audio` or `Capture error`.

## Verification

- `npm test`
- `npm run build`
- `cargo fmt --check`
- `cargo check`
- Unit tests:
  - matching input events above threshold become `receiving`
  - matching input events below threshold become `silent`
  - unmatched device events are ignored
  - no fresh matching event during an active session becomes `stale`
  - stop/input-change resets to `waiting`
  - capture error becomes `error`
- Manual test:
  - Teams/system audio routed to BlackHole moves the meter and shows `Receiving audio`
  - silence while capture continues shows `Source silent`
  - disconnected or failed capture does not look like normal silence

## Exit Criteria

- The user can confirm at a glance that the selected meeting source is delivering audio data to the app.
- The UI does not claim the stream is broken during normal silence.
- The indicator does not react to events from an old input selection.
- Translation session behavior, transcript updates, manual boundary controls, playback, and original monitor routing remain unchanged.

## Non-Goals

- Building a full waveform display.
- Recording raw audio.
- Adding per-speaker detection.
- Changing OpenAI realtime translation behavior.
- Reworking audio routing setup beyond the source signal indicator.
