# VieNeu-TTS Provider — 2026-07-18

## Context

Local spoken translation previously used only installed system voices. The requested extension adds `pnnbao97/VieNeu-TTS` as an optional Vietnamese speech provider while preserving the existing Whisper -> Gemma -> normalized PCM -> selected CPAL output pipeline (`plans/260716-2033-local-llm-audio-translation/plan.md:114`, `src-tauri/src/tts.rs:10`).

## Change

Added a loopback-only Python bridge pinned to `vieneu==3.2.3`; it keeps v3 Turbo loaded, exposes preset voices, validates synthesis inputs, and returns mono PCM16 WAV at 48 kHz (`sidecars/vieneu-tts/pyproject.toml:7`, `sidecars/vieneu-tts/server.py:26`, `sidecars/vieneu-tts/server.py:90`). The Rust adapter accepts only HTTP loopback origins, discovers voices, bounds and cancels whole-utterance requests, validates WAV responses, and reuses the existing decoder/resampler to produce the application's 24 kHz audio contract (`src-tauri/src/tts.rs:19`, `src-tauri/src/tts.rs:59`, `src-tauri/src/tts.rs:140`, `src-tauri/src/tts.rs:837`).

Provider, bridge URL, and reading style now flow through persisted Rust/TypeScript config and the Local LLM settings UI; provider-specific voice refresh and readiness checks keep System TTS as the default (`src-tauri/src/models.rs:624`, `src-tauri/src/local_translation.rs:315`, `src-tauri/src/local_translation.rs:432`, `src/types.ts:208`, `src/components/settings/LocalLlmSettings.tsx:213`, `src/app/MainApp.tsx:901`).

## Impact

**Risk level: medium.** Local users can select VieNeu preset voices without changing transcript ordering or output-device/channel routing. Operational risk comes from a separately launched Python environment, first-run model download, upstream package/model compatibility, and non-streaming whole-utterance latency; loopback-only URL validation, request/response size limits, serialized model access, and retaining System TTS as the default constrain exposure (`sidecars/vieneu-tts/server.py:54`, `sidecars/vieneu-tts/server.py:186`, `src-tauri/src/tts.rs:11`, `src-tauri/src/tts.rs:178`).

The implementation was developed while a separate Whisper downloader change touched some of the same backend files. That work landed independently before final staging, keeping its history separate from the VieNeu-TTS provider (`src-tauri/src/commands.rs:161`, `src-tauri/src/local_translation.rs:62`, `src/app/MainApp.tsx:901`).

## Decision

Use a long-lived Python sidecar to isolate the Python-only model runtime and avoid reloading it per utterance, while keeping the Rust TTS boundary and 24 kHz playback contract unchanged. Whole-response WAV was chosen for the first integration; direct Rust inference, 48 kHz playback changes, bundled process lifecycle, and chunked streaming remain deferred alternatives (`sidecars/vieneu-tts/README.md:3`, `sidecars/vieneu-tts/README.md:25`, `src-tauri/src/tts.rs:226`).

## References

- bridge runtime and API: `sidecars/vieneu-tts/server.py:26`, `sidecars/vieneu-tts/server.py:90`
- Rust provider adapter: `src-tauri/src/tts.rs:19`, `src-tauri/src/tts.rs:140`
- provider validation and readiness: `src-tauri/src/local_translation.rs:432`, `src-tauri/src/local_translation.rs:603`
- settings integration: `src/components/settings/LocalLlmSettings.tsx:200`, `src/app/MainApp.tsx:901`
