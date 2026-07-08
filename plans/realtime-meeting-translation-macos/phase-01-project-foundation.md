# Phase 01 - Project Foundation

Status: planned
Depends on: none

## Goal

Scaffold the Tauri React TypeScript application and establish stable frontend/backend contracts before implementing audio or API logic.

## Implementation Tasks

- Initialize a Tauri v2 app using React, TypeScript, and Vite.
- Add base frontend layout:
  - top status bar
  - language/source-target controls
  - input/output device selectors with placeholder data
  - Start/Stop/Pause/Resume controls
  - transcript panel shell
  - settings dialog for API key and translation style
- Add Rust module skeleton:
  - `audio`
  - `ai`
  - `session`
  - `security`
  - `commands`
  - `events`
- Define shared DTOs for:
  - device info
  - session config
  - session status
  - transcript item
  - normalized app error
- Add typed Tauri commands:
  - `get_app_status`
  - `list_audio_devices`
  - `start_session`
  - `pause_session`
  - `resume_session`
  - `stop_session`
  - `save_api_key`
  - `has_api_key`
  - `export_transcript`
- Add a minimal event contract:
  - `session-status`
  - `transcript-update`
  - `audio-level`
  - `app-error`
- Configure formatting, linting, and basic tests for TypeScript and Rust.

## Verification

- `npm install` or chosen package manager install completes.
- Tauri dev server opens the app.
- Rust compiles with the module skeleton.
- Frontend renders without console errors.
- Tauri command smoke tests return placeholder values.

## Exit Criteria

- The app shell runs locally.
- Frontend and Rust share a documented command/event contract.
- The next phase can replace placeholder devices with real device enumeration.
