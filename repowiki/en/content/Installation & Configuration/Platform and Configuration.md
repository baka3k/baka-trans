<cite>
- README.md
- src-tauri/tauri.conf.json
- src-tauri/tauri.macos.conf.json
- src-tauri/tauri.windows.conf.json
- src-tauri/src/security.rs
- src-tauri/src/local_translation.rs
- src-tauri/src/llm.rs
</cite>

# Platform and Configuration

## Table of Contents

- [Platform packaging](#platform-packaging)
- [Credentials](#credentials)
- [Local configuration](#local-configuration)
- [Security posture](#security-posture)

## Platform packaging

**Verified.** Tauri builds a DMG/app on macOS and a per-user NSIS installer on Windows. The macOS configuration enables the private API; Windows build configuration targets NSIS. The common configuration bundles the VieNeu bridge resource.

## Credentials

**Verified.** Cloud credentials can come from `OPENAI_API_KEY` or `GEMINI_API_KEY`; environment values take precedence over the system keychain/credential store. The UI saves cloud keys through the native keyring integration. Local Whisper + Ollama does not use a cloud translation API key.

## Local configuration

**Verified.** Local translation settings persist in the application data directory, including model path, Ollama base URL/model, segmentation thresholds, and TTS selection. LLM summary profiles are persisted separately in the application data directory. The docs intentionally omit user-specific paths and values.

## Security posture

**Verified.** The Tauri capability manifest grants default core permissions and window dragging to the three application windows. The main Tauri CSP is explicitly `null`; this is a known configuration to review before exposing untrusted web content. API keys are redacted in status using a fingerprint, but secrets must still be treated as sensitive.
