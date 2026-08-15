<cite>
- src-tauri/src/lib.rs
- src-tauri/src/commands.rs
- src/api.ts
- src/app/MainApp.tsx
</cite>

# Tauri Command and Event Reference

## Table of Contents

- [Introduction](#introduction)
- [Command groups](#command-groups)
- [Events](#events)
- [Trust boundary](#trust-boundary)

## Introduction

**Verified.** This is an in-process Tauri IPC surface, not an HTTP API. Commands are registered in `lib.rs`, implemented in `commands.rs`, and invoked by name from `src/api.ts`.

## Command groups

| Group | Representative commands | Handler responsibility |
| --- | --- | --- |
| Session | `start_session`, `pause_session`, `resume_session`, `stop_session`, `force_translate_boundary` | Validate and transition `AppState` |
| Audio | `list_audio_devices`, `play_test_tone`, `start_local_monitor` | Device discovery and audio test/monitor resources |
| Credentials | `save_translation_api_key`, `translation_credential_status`, `test_translation_api_key` | Keyring-backed cloud credentials and connectivity checks |
| Local runtime | `get_local_translation_config`, `download_whisper_model`, `install_vieneu_runtime` | Local model/TTS configuration and lifecycle |
| Workspace | `run_meeting_summary_agent`, `export_transcript`, LLM profile commands | Transcript transformation and persisted provider profiles |
| Overlay | open/close/status/config commands | Separate overlay-window state and OCR/LLM interactions |

## Events

**Verified.** The native host emits renderer events including `session-status`, `transcript-update`, `app-error`, audio-level updates, summary status/results, and overlay/Look & Help status updates. `MainApp.tsx` subscribes through `@tauri-apps/api/event` and updates renderer state.

## Trust boundary

**Verified.** Renderer input crosses Tauri IPC into native commands. Commands validate key session inputs, while secret loading stays native. New commands should be registered in `lib.rs`, implemented in `commands.rs`, exposed in `src/api.ts`, and accompanied by validation and a renderer call site/test where applicable.
