<cite>
- src/app/MainApp.tsx
- src/api.ts
- src/transcript.ts
- src-tauri/src/commands.rs
- src-tauri/src/session.rs
- src-tauri/src/overlay.rs
- src-tauri/src/look_help.rs
- sidecars/vieneu-tts/server.py
</cite>

# Module Guide

## Table of Contents

- [Renderer](#renderer)
- [Native host](#native-host)
- [Translation and audio](#translation-and-audio)
- [Overlays](#overlays)
- [VieNeu sidecar](#vieneu-sidecar)

## Renderer

**Verified.** `src/app/MainApp.tsx` is the main interactive workspace. `src/api.ts` is the typed Tauri invocation boundary and `src/transcript.ts` owns transcript merging, presentation shaping, export rendering, and summary prompt helpers.

## Native host

**Verified.** `commands.rs` exposes the renderer-facing command surface. `session.rs` coordinates lifecycle and owns in-memory session resources. `models.rs` defines the serialized command/event contracts.

## Translation and audio

**Verified.** `audio.rs` enumerates devices and manages CPAL capture/playback. `ai/` contains cloud and local provider pipelines; `local_translation.rs`, `tts.rs`, and `vieneu.rs` provide the local runtime configuration and voice path.

## Overlays

**Verified.** Transparent translation and Look & Help overlay functionality are separated in `overlay.rs` and `look_help.rs`, with corresponding renderer routes selected in `App.tsx`.

## VieNeu sidecar

**Verified.** The Python bridge is loopback-only, verifies the managed model files, and exposes health, voice enumeration, and synthesis operations to the Rust manager.
