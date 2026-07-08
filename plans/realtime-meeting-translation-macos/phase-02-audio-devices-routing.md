# Phase 02 - Audio Devices and Routing

Status: planned
Depends on: phase 01

## Goal

Implement reliable macOS audio device enumeration, capture, and playback primitives using BlackHole 2ch as the default Teams routing path.

Phase 02 owns low-level device primitives. User-facing multi-route behavior for meeting input, translated output, and optional original-audio monitoring is planned separately in `phase-06-audio-routing-profile.md`.

## Implementation Tasks

- Implement audio device enumeration with `cpal`:
  - input devices
  - output devices
  - default input/output markers
  - supported sample rates and channel counts
- Add device refresh and device-disconnected handling.
- Implement capture stream:
  - selected input device
  - non-blocking callback
  - bounded channel into async pipeline
  - basic audio level metering for UI
- Implement format normalization:
  - convert sample types to `f32` internally
  - downmix to mono
  - resample to 24 kHz when required by realtime API path
  - encode PCM16 chunks
- Implement output playback queue:
  - selected output device
  - playable local test tone
  - controlled buffer size to avoid runaway latency
- Expose reusable playback primitives so phase 06 can run a second output queue for original-audio monitoring.
- Add setup guide UI:
  - install/select BlackHole 2ch
  - set Teams speaker output to BlackHole or a multi-output route
  - select BlackHole as app input
  - select headphones as app output
  - defer advanced original-audio monitor selection to phase 06
- Add explicit warning when input and output devices can create feedback.

## Verification

- Device list updates after adding/removing devices.
- BlackHole appears as an input when installed.
- Captured audio level moves when Teams/system audio is routed into BlackHole.
- Test tone plays through the selected output.
- Stopping capture/playback releases devices cleanly.

## Exit Criteria

- The app can capture from a selected input and play to a selected output.
- Audio is available to phase 03 as PCM16 mono chunks at the expected sample rate.
