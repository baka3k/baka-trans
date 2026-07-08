# Phase 06 - Audio Routing Profile

Status: planned
Depends on: phase 02, phase 03, phase 04

## Goal

Let the user explicitly choose three separate audio routes for a meeting:

- translation input source: the device the app listens to and sends for translation, usually BlackHole 2ch or another virtual device carrying Teams audio
- translated output: the private device that plays translated speech, usually headphones or Bluetooth earphones
- original monitor output: an optional output device that plays the original meeting audio locally, such as Mac speakers

This supports the target workflow: keep Teams audio available through Mac speakers while translated audio is sent privately to headphones.

## Implementation Tasks

- Extend session config and backend DTOs:
  - rename the current generic input/output labels in the UI to clarify intent
  - keep `inputDeviceId` as the translation input route
  - keep `outputDeviceId` as translated speech output for backward compatibility
  - add `monitorOutputDeviceId`
  - add `monitorOriginalAudio`
  - add a routing profile summary for UI and validation messages
- Implement original-audio monitor playback:
  - duplicate captured PCM after normalization into a separate monitor playback queue
  - play monitor audio to `monitorOutputDeviceId` only when enabled
  - keep monitor playback independent from translated-output playback
  - cap monitor queue depth to avoid delayed echo after network stalls
- Update UI controls:
  - group audio selectors into `Meeting source`, `Translated audio`, and `Original audio monitor`
  - add a toggle for original monitoring
  - show device intent text near each selector
  - add separate test buttons for translated output and monitor output
  - show a compact routing summary before Start
- Add routing validation:
  - warn when translation input and translated output appear to be the same physical device
  - warn when translation input and monitor output appear to be the same physical device
  - warn when translated output and monitor output are the same device unless the user explicitly accepts duplicated audio
  - prevent session start only for missing required devices, not for non-blocking warnings
- Add persistence:
  - remember last selected translation input
  - remember last selected translated output
  - remember last selected monitor output
  - remember monitor enabled/disabled state
  - fall back by stable device ID first, then by device name if IDs change
- Update setup guidance:
  - document Teams output to BlackHole for translation capture
  - document macOS Multi-Output Device or app-level Teams routing when the user also wants original audio through speakers
  - explain that the app can monitor original audio only from the selected captured source, not from Teams directly

## Verification

- User can select a BlackHole or virtual device as meeting source input.
- User can select headphones as translated output.
- User can enable original monitoring and select Mac speakers as monitor output.
- Test tone works independently for translated output and monitor output.
- During a controlled audio source test, translated audio and original monitor audio go to their selected devices.
- Disabling original monitor stops only the original monitor queue and does not stop translation.
- Start/Pause/Resume/Stop release all capture, translated playback, and monitor playback streams cleanly.
- Routing warnings appear for same-device or feedback-prone combinations.

## Exit Criteria

- The app supports the meeting routing profile requested by the user: selected source audio for translation, selected translated-output headphones, and optional original audio through Mac speakers.
- The feature is documented enough for a user to understand when BlackHole, Multi-Output Device, or Teams device settings are required.
