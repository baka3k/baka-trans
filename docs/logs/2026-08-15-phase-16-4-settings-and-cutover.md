# Phase 16.4: Settings UX and Ollama Cutover — 2026-08-15

## Context

Phase 16.4 of the Hy-MT2 migration plan. Final cutover: removes all
executable and user-facing Ollama code from the local translation path,
renames types to be engine-neutral, and wires the settings UI to
OpenAI-specific fields. The Hy-MT2 offline engine remains gated (stub
error) until Phase 16.1 quality gate records GO.

Plan: `plans/260716-2033-local-llm-audio-translation/phase-16-04-settings-and-cutover.md`

## Change

### Ollama Code Removal (`src-tauri/src/local_translation.rs`)

- Removed `OllamaClient` struct, `normalize_ollama_chat_url`,
  `build_ollama_payload`, `parse_ollama_response`,
  `validate_legacy_ollama_fields`, `translation_system_prompt`, and `compact`.
- Removed all Ollama-specific unit tests (URL normalization, payload building,
  response parsing, native API chat, dispatcher-to-Ollama, missing base URL).
- Removed unused imports: `StatusCode`, `Url`, `Instant`, `json!`, `Value`.

### TranslationClient Dispatcher

- Removed `Ollama` variant from `TranslationClient` enum.
- `HuggingfaceOffline` now returns `AppError("local_hy_mt2_not_available")`.
- `OpenAiCompatible` remains the only active translation path.

### Type Renames (`src-tauri/src/models.rs`, `src/types.ts`)

- `TranslationProvider::LocalWhisperOllama` → `LocalWhisper` (Rust) and
  `"local_whisper_ollama"` → `"local_whisper"` (TypeScript).
- `ollama_reachable`/`ollama_model_accepted` → `engine_reachable`/`engine_accepted`
  in `LocalTranslationTestResult` (both Rust and TypeScript).
- Removed legacy Ollama fields (`base_url`, `model`, `timeout_seconds`,
  `temperature`, `max_output_tokens`, `keep_alive`) from both
  `LocalTranslationConfig` and `LocalTranslationConfigDraft`.

### Module Rename

- `src-tauri/src/ai/local_whisper_ollama.rs` → `local_worker.rs`.
- Removed `local_whisper_ollama_end_to_end_smoke_test` (required Ollama).
- Updated `ai.rs` module declarations and re-exports.

### Migration Path Preserved

- `migrate_legacy_config` still reads legacy v1/v2 JSON with Ollama fields
  and maps them to `HuggingfaceOffline` — but no longer stores the removed
  fields in the target struct. Atomic `.legacy-backup` still created.

### Frontend Settings (`src/components/settings/LocalLlmSettings.tsx`)

- Engine UI now binds `openaiBaseUrl`, `openaiModel`, `openaiTimeoutSeconds`,
  `openaiMaxOutputTokens`, `openaiTemperature` (not legacy fields).
- Health label changed from "Ollama reachable and model accepted" to
  "Translation engine reachable and accepted".
- Default config no longer includes legacy Ollama fields.

### Labels and References

- MainApp: `"local_whisper_ollama"` → `"local_whisper"`, "Local Whisper +
  Ollama" → "Local Whisper" in provider options and display function.
- `languages.ts`: provider filter updated to `"local_whisper"`.
- `security.rs`: error messages updated from "Local Whisper + Ollama" to
  "Local Whisper".
- `session.rs`, `commands.rs`: match arms updated to `LocalWhisper`.

### Documentation

- `ARCHITECTURE.md`: pipeline diagram updated from "native Ollama /api/chat
  Gemma translation" to "selected translation engine (Hy-MT2 offline or
  OpenAI-compatible API)".
- `README.md`: pipeline, setup steps, smoke test section, and error code
  table all updated. Ollama setup instructions removed.

## Impact

- **Users with existing Ollama configs**: Migration still works — legacy v1/v2
  JSON is read, Ollama fields are ignored (not stored), engine set to
  `huggingface_offline`. Atomic backup preserved.
- **Active local sessions**: No disruption — existing sessions use the
  TranslationClient dispatcher which only has `OpenAiCompatible`.
- **Hy-MT2 offline engine**: Returns `"local_hy_mt2_not_available"` error
  until Phase 16.1 gate records GO or explicit owner CAUTION.
- **Cloud LLM profiles**: `LlmProviderKind::Ollama` for meeting summaries
  is intentionally out of scope for this phase.
- **Risk**: Low. All 60 frontend + 110 Rust tests pass.

## Decision

- Removed legacy Ollama fields from the struct entirely rather than keeping
  `skip_serializing` because the OllamaClient is gone — no code reads them.
- Kept `LlmProviderKind::Ollama` for cloud meeting-summary profiles because
  those are a separate feature outside the local translation cutover scope.
- Chose `engine_reachable`/`engine_accepted` over `translation_reachable` for
  brevity and consistency with the "translation engine" selector in the UI.

## References

- Plan: `plans/260716-2033-local-llm-audio-translation/phase-16-04-settings-and-cutover.md`
- Commit: `5a6564d`
- Previous: Phase 16.2 (`docs/logs/2026-08-15-phase-16-2-engine-config-and-api-client.md`)
