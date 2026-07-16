# Phase 01: Contracts and Local Configuration

## Context

The app has cloud translation credentials and generic summary-agent LLM profiles, but no dedicated local translation provider, Whisper configuration, or native Ollama `/api/chat` client. This phase creates stable contracts before changing the realtime pipeline.

## Requirements

- Add `local_whisper_ollama` without changing serialized values for existing providers.
- Persist one dedicated local translation configuration outside `llm-profiles.json`.
- Validate the Japanese-to-Vietnamese language constraint, Whisper model path, fixed sample rate, tuning ranges, Ollama URL, and model name.
- Expose get/save/test commands through Tauri and TypeScript.
- Test native Ollama request/response behavior without requiring a running Ollama instance.

## Architecture

- `models.rs` owns serializable API types and validation-visible enums.
- A new `local_translation.rs` owns config defaults, disk persistence, native Ollama URL normalization, request construction, response parsing, and the config test operation.
- The local config file uses the same application config directory strategy as `llm.rs`, but has its own schema/version so summary profiles can evolve independently.
- Add transcript `revision` and `updateMode` fields with serde/default compatibility so existing serialized fixtures remain readable.

## Related Files

- `src-tauri/src/models.rs`
- new `src-tauri/src/local_translation.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src/api.ts`
- `src/types.ts`
- `src-tauri/Cargo.toml`

## Implementation Steps

1. Define `LocalTranslationConfig`, `LocalTranslationConfigDraft`, validation/test result types, and provider/status enums in Rust and TypeScript.
2. Add schema versioning and atomic persistence for `local-translation-config.json`; preserve the last valid file if serialization or replacement fails.
3. Normalize `http://localhost:11434`, a trailing slash, and a full `/api/chat` URL to the exact native endpoint; reject empty or non-HTTP(S) values.
4. Build a non-streaming Ollama payload with translation-only messages and native `options`; parse `message.content` and provider errors.
5. Add `get_local_translation_config`, `save_local_translation_config`, and `test_local_translation_config` commands and frontend wrappers.
6. Make translation credential status provider-aware so the local provider is ready without an API key instead of calling `security::load_translation_api_key`.
7. Add unit tests for defaults, bounds, persistence round-trip, invalid model paths, URL normalization, exact request JSON, success, HTTP error, native error, malformed JSON, and empty content.

## Todo

- [ ] Rust and TypeScript contracts compile.
- [ ] Config persistence is versioned and tested.
- [ ] Native `/api/chat` client is isolated from summary `/chat/completions` code.
- [ ] Tauri commands are registered and callable.
- [ ] Local provider never requests a cloud credential.

## Risks

- Config fields can drift between Rust and TypeScript. Keep names camelCase and add round-trip fixtures.
- A test command that only checks the port can miss model errors. Send a minimal real chat payload and report the configured model in the result.
- Model paths differ by platform. Store a user-selected absolute path without platform-specific rewriting.

## Success Criteria

- A valid local configuration can be saved, reloaded, and tested through the same Tauri bridge used by the UI.
- A mock server proves the client calls `/api/chat` with `stream: false` and parses `message.content`.
- Existing LLM summary profiles and translation credential tests remain unchanged and pass.
