# Runtime dependencies

Cloud translation uses OpenAI Realtime or Google Live provider modules. Local translation uses Whisper and Ollama `/api/chat`. Managed Vietnamese TTS uses the local VieNeu bridge. Evidence: `src-tauri/src/ai.rs`, `src-tauri/src/local_translation.rs`, `src-tauri/src/vieneu.rs`.
