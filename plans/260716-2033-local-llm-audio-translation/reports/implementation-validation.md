# Implementation Validation

Date: 2026-07-18

## Implemented

- Main-route Cloud API / Local Whisper chooser with overlay route isolation.
- Focused cloud and local navigation while retaining one `MainApp` controller and one Tauri listener set.
- Gemma default, persisted voice/rate/volume configuration, voice discovery, and routed voice preview.
- Windows `Windows.Media.SpeechSynthesis` stream adapter and macOS platform `say` buffer adapter.
- WAV validation, PCM16 stereo downmix, 24 kHz normalization, and selected CPAL output/channel playback.
- Bounded ordered local TTS queue after final Gemma transcript snapshots.
- Activity arbitration across overlapping translation and speech workers, plus local translated-audio meter events.
- Transcript rehydration when switching between the mode chooser and a workspace.
- Immediate local playback drop on Stop, shared cancellation checks, and speech-specific errors that preserve final text.
- User and architecture documentation for the spoken local path.

## Automated Evidence

- `npm test -- --run`: 53 passed.
- `npm run build`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml`: 74 passed, 4 opt-in tests ignored.
- `cargo test --manifest-path src-tauri/Cargo.toml windows_system_voice_synthesis_smoke_test -- --ignored --nocapture`: passed with an installed Windows system voice.
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`: passed after formatting.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`: passed.
- `git diff --check`: passed.

## Remaining Release Gates

- Run a complete Whisper -> Gemma -> Windows TTS session against a real selected non-default headset and verify all/left/right routing.
- Run voice discovery, synthesis, cancellation, and selected-output playback on macOS.
- Confirm macOS `say` buffer format on supported release versions and replace it with a direct AVSpeechSynthesizer buffer adapter if release policy requires the original planned framework boundary literally.
- Inspect the chooser and both themes in the desktop webview. The integrated browser test runtime did not initialize in this Windows session; DOM accessibility checks passed.

The plan remains `in_progress` until both Windows routed-device and macOS hardware gates are recorded.
