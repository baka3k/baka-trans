# Module summary

The frontend is a React renderer that calls a Rust/Tauri host through typed IPC. The native host owns session/audio/provider state and can supervise the Python VieNeu TTS bridge. Release tooling packages the application for macOS and Windows.

Evidence: `src/api.ts`, `src-tauri/src/lib.rs`, `src-tauri/src/session.rs`, `sidecars/vieneu-tts/server.py`, `scripts/release-macos.sh`.
