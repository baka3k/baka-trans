# Phase 11: Translation Provider Abstraction and Google Credentials

Status: planned
Depends on: phases 03, 04, 09, 10
Primary files: `src-tauri/src/models.rs`, `src-tauri/src/security.rs`, `src-tauri/src/session.rs`, `src-tauri/src/commands.rs`, `src/api.ts`, `src/types.ts`, `src/App.tsx`, `src/languages.ts`

## Objective

Prepare the app for a controlled OpenAI to Google migration by separating provider-neutral session logic from provider-specific translation clients and credentials.

## Current State

- `src-tauri/src/ai.rs` is OpenAI-specific: URL, client-secret minting, payload schema, event names, manual-boundary behavior, and error labels.
- `src-tauri/src/security.rs` stores one translation key under the Keychain user `openai-api-key` and reads `OPENAI_API_KEY`.
- `src-tauri/src/models.rs` validates target languages against OpenAI Realtime's 13-language output set.
- `src/languages.ts` is OpenAI-oriented and collapses some language variants that Google exposes as BCP-47 regional/script codes.
- The summary-agent provider system is separate, but its first-class labels still bias toward OpenAI-compatible chat completions.

## Implementation Approach

1. Add provider models.
   - Add `TranslationProvider` with at least `OpenaiRealtime` and `GoogleLiveTranslate`.
   - Add provider to `SessionConfig` and frontend state.
   - Default new installs to `GoogleLiveTranslate`; preserve existing sessions/settings by falling back to `OpenaiRealtime` only when legacy config is detected.

2. Split translation clients.
   - Move the current OpenAI code into a provider-specific module, for example `src-tauri/src/ai/openai_realtime.rs`.
   - Add a provider-neutral facade in `src-tauri/src/ai.rs` with the existing public functions where practical.
   - Keep `RealtimeControl`, transcript emission, playback, and session status contracts stable for the UI.

3. Make credentials provider-aware.
   - Replace generic `save_api_key`, `has_api_key`, and `test_api_key` internals with provider-aware storage.
   - Store Google translation credentials under a new Keychain user such as `google-gemini-api-key`.
   - Support `GEMINI_API_KEY` for development.
   - Keep legacy OpenAI key loading for rollback until phase 14.

4. Update UI settings.
   - Replace "Translation OpenAI key" with a provider selector and provider-specific credential panel.
   - Show the active provider, key source, fingerprint, and test result.
   - Avoid exposing standard Gemini API keys to React beyond save/test commands.

5. Update language metadata boundaries.
   - Keep one source of truth for language metadata with provider-specific `supportsTargetByProvider`.
   - Preserve `Auto` only where provider behavior supports or requires automatic detection.
   - Do not reuse OpenAI's 13-target validation for Google.

## Data Model Notes

Recommended additions:

```text
TranslationProvider:
  - openai_realtime
  - google_live_translate

TranslationCredentialStatus:
  - provider
  - has_key
  - source
  - fingerprint
  - last_test_message

LanguageMetadata:
  - code
  - label
  - supports_source_by_provider
  - supports_target_by_provider
  - is_auto
```

## Acceptance Criteria

- The user can choose OpenAI or Google as translation provider.
- Google key save/test/status works through Keychain or `GEMINI_API_KEY`.
- Existing OpenAI path still works after the refactor.
- Unsupported target-language errors include the selected provider name.
- UI copy no longer describes all translation credentials as OpenAI keys.

## Verification

- Rust tests for provider enum serialization, credential lookup, and target-language validation.
- TypeScript tests for target-language lists per provider.
- Manual settings test for saving, replacing, and testing Google and OpenAI credentials independently.
