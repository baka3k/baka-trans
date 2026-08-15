# Phase 16.2: Engine Config and OpenAI-Compatible Client — 2026-08-15

## Context

Phase 16.2 of the Hy-MT2 migration plan. Removes Ollama from the persisted
config contract (schema v3) and adds a secure OpenAI-compatible Chat
Completions client with keychain-based API key management. Supersedes the
Ollama assumptions from Phases 11-13.

Plan: `plans/260716-2033-local-llm-audio-translation/phase-16-02-engine-config-and-api-client.md`

## Change

### Schema v3 Migration (`src-tauri/src/models.rs`, `src-tauri/src/local_translation.rs`)

- Added `openaiBaseUrl`, `openaiModel`, `openaiTimeoutSeconds`,
  `openaiTemperature`, `openaiMaxOutputTokens` to `LocalTranslationConfig` and
  `LocalTranslationConfigDraft`.
- Marked legacy Ollama fields (`baseUrl`, `model`, `keepAlive`,
  `timeoutSeconds`, `temperature`, `maxOutputTokens`) with
  `#[serde(skip_serializing, default)]` — they remain in-memory for the
  existing OllamaClient until Phase 16.3 replaces the dispatcher but are
  never written to persisted JSON.
- `read_config_from_path` now detects legacy schema versions (v1, v2) and
  runs `migrate_legacy_config` which creates an atomic `.legacy-backup` file,
  maps the engine to `HuggingfaceOffline` (never to a remote API), and
  preserves all non-secret settings.

### OpenAI-Compatible Client (`src-tauri/src/local_translation/openai_compatible.rs`)

- `OpenAiCompatibleClient` with full security hardening:
  - URL normalization to `/v1/chat/completions` path
  - Rejects embedded credentials and URL fragments
  - Requires HTTPS for non-loopback endpoints
  - Redirects disabled (`Policy::none()`)
  - API key never sent over HTTP
  - Source text bounded to 10,000 characters
  - Response body bounded to 1 MiB
  - Error messages redacted (strips `sk-*`, `Bearer *`, `token=*` patterns)
  - Only non-empty `choices[0].message.content` accepted

### API Key Management (`src-tauri/src/local_translation/api_key.rs`)

- `save_local_translation_api_key` / `load_local_translation_api_key` /
  `delete_local_translation_api_key` using OS keychain via `keyring` crate.
- `BAKA_TRANS_LOCAL_API_KEY` environment variable override (checked first).
- In-memory cache for runtime performance.
- `fingerprint_key` for UI display without exposing secrets.

### Engine-Specific Validation (`src-tauri/src/local_translation.rs`)

- `validate_legacy_ollama_fields`: only validates when both `base_url` and
  `model` are non-empty (backward compat).
- `validate_openai_compatible_fields`: requires URL and model when engine is
  `OpenaiCompatible`; delegates URL normalization to the secure normalizer.
- `test_config` dispatches by engine: `HuggingfaceOffline` skips translation
  probe (sidecar not yet available), `OpenaiCompatible` uses the new client.

### Frontend Types (`src/types.ts`, `src/components/settings/LocalLlmSettings.tsx`)

- Added `openaiBaseUrl`, `openaiModel`, `openaiTimeoutSeconds`,
  `openaiTemperature`, `openaiMaxOutputTokens` to `LocalTranslationConfig`.
- Legacy Ollama fields marked optional.
- Default config updated with empty legacy fields.
- Frontend validation updated to check `openaiBaseUrl` / `openaiModel`.

## Impact

- **Users with existing Ollama configs**: Automatically migrated to
  `huggingface_offline` engine on next load. Original config backed up as
  `.legacy-backup`. No data loss.
- **Security**: OpenAI-compatible API keys never persisted to JSON, never
  sent over unencrypted HTTP, and redacted from error messages.
- **Risk**: Low. OllamaClient remains functional for Phase 16.3 transition.
  All 111 Rust tests and 60 frontend tests pass.

## Decision

- Used `skip_serializing` rather than removing Ollama fields from the struct
  because the OllamaClient is still the active worker until Phase 16.3 adds
  the dispatcher. This preserves the working pipeline while removing Ollama
  from the persisted contract.
- API key uses keychain + env override (not JSON config) per plan requirement
  "never serialize it to the local translation config."
- Separate `api_key.rs` and `openai_compatible.rs` submodules to keep
  `local_translation.rs` manageable.

## References

- Plan: `plans/260716-2033-local-llm-audio-translation/phase-16-02-engine-config-and-api-client.md`
- Commits: `433775d`, `7f7bcc5`, `60f5487`, `a9c2f9f`
- Next: Phase 16.3 — Rust Dispatcher and Session
