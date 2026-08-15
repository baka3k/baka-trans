# Native desktop overview

The Rust host starts the Tauri application, registers commands, and manages shared application state. It owns audio capture/playback, translations, credentials, session lifecycle, overlays, local model configuration, and transcript export.

The command module is the renderer-facing boundary. Provider, audio, and persistence code remain behind that boundary so credentials and native resources are not handled by React.
