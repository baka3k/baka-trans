# Phase 05 - Hardening, Privacy, and Packaging

Status: planned
Depends on: phase 04

## Goal

Prepare the MVP for real meeting use on macOS with stability, privacy, and packaging checks.

## Implementation Tasks

- Add long-session stability checks:
  - 30-minute synthetic audio run
  - 2-hour soak run before release
  - memory and CPU observation
- Add device resilience:
  - input device disappears
  - output device disappears
  - Bluetooth reconnect
  - sample rate changes
- Add network resilience:
  - reconnect with backoff
  - avoid duplicate transcript items after reconnect
  - clear user messaging when recovery fails
- Add privacy controls:
  - explicit notice that audio is sent to external services
  - no raw audio persistence
  - transcript stored only for current session unless exported
  - Keychain storage for API key
- Add app settings persistence:
  - preferred languages
  - selected devices by stable ID/name fallback
  - voice
  - translation style
- Add packaging:
  - Tauri bundle config
  - macOS permissions review
  - app icon placeholder or final asset
  - notarization checklist if distribution is required
- Add release checklist:
  - BlackHole setup doc
  - first-run guide
  - troubleshooting doc
  - known limitations

## Verification

- Two-hour session does not crash or leak unbounded memory.
- Device removal and reconnection do not crash the app.
- API key is stored in Keychain and not in app logs.
- App bundle launches outside dev mode.
- Exported transcript contains no API key or raw audio data.

## Exit Criteria

- MVP is usable for a real Teams meeting on macOS 13+ with documented setup steps and known limitations.
