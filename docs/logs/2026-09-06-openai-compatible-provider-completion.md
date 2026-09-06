# OpenAI-Compatible Provider Completion (DeepSeek-Ready) — 2026-09-06

Umbrella phase 16.5 of
`plans/260716-2033-local-llm-audio-translation`, delivered by
`plans/260906-2137-openai-compatible-provider-completion`.

## Context

Phase 16 already shipped the OpenAI-compatible engine dispatcher, the secure
`POST <base>/v1/chat/completions` client, and the load-only keychain read
(`BAKA_TRANS_LOCAL_API_KEY` → in-memory cache → keychain). The 16.2 log claimed
keychain save/delete commands, but the code had no write path: a DeepSeek key
could not be entered anywhere in the app, there were no provider presets, a
missing key surfaced only as a per-utterance `local_openai_request_error`
mid-session, and no egress note accompanied hosted endpoints.

## Change

### Key lifecycle (`src-tauri/src/local_translation/api_key.rs`)

- `save_local_translation_api_key` — trim, reject empty and >4096 chars with
  `credential_input_invalid`, keyring `set_password`, cache refresh.
- `delete_local_translation_api_key` — keyring `delete_credential`
  (`NoEntry` → `Ok`), cache clear.
- `local_translation_credential_status` — resolves `{ hasKey, source }` in the
  same order as load (env wins, then keychain; the cache only mirrors the
  keychain). Never returns key material; keychain I/O is skipped when the env
  or cache answer is already known.
- New `LocalCredentialStatus` model (`models.rs`) reusing `ApiKeySource`.
- Commands `save_local_translation_api_key`, `delete_local_translation_api_key`,
  `local_translation_credential_status` (`commands.rs`, `run_blocking`) and
  registration in `lib.rs`.

### Start-time key gate (`src-tauri/src/local_translation.rs`)

- Pure decision `ensure_openai_key_available(endpoint, key)` errors with
  `local_openai_api_key_missing` for non-loopback endpoints without a key
  (empty/whitespace counts as missing); loopback hosts stay keyless. Loopback
  detection now shares the client's `is_loopback_url` (`pub(crate)`).
- Wired into `validated_runtime_config()` (session start fails before
  capture/Whisper begin; offline Hy-MT2 never reaches the gate) and into
  `probe_openai_engine()` via the new `probe_openai_engine_with_key` seam
  (the engine test reports the actionable message text before any network
  round trip).
- `normalize_and_validate` stays key-agnostic on purpose: it cannot observe
  env/keychain state atomically and env-only setups must keep validating.

### Settings UX (`src/components/settings/LocalLlmSettings.tsx`)

- Frontend-only `TRANSLATION_PROVIDER_PRESETS` (Custom, DeepSeek
  `https://api.deepseek.com` + `deepseek-v4-flash`, Ollama
  `http://127.0.0.1:11434`, LM Studio `http://127.0.0.1:1234`). Selecting a
  preset fills the editable draft URL/model; the select value is derived from
  the current draft so unsaved edits are never clobbered. The preset itself is
  not persisted and the config schema stays at v4.
- API key row (engine `openai_compatible` only): password input (never
  pre-filled), Save/Clear buttons wired through parent callbacks to the new
  api.ts commands, `aria-live` status line (Not set / OS keychain /
  Environment variable / In-memory cache), and an override hint when the env
  var currently wins.
- Advisory `role="note"` egress warning when the draft base URL host looks
  non-loopback (JS mirror of the Rust rule; backend gate is authoritative).
  No persisted acknowledgment flag — the umbrella "egress warning
  acknowledged" prerequisite is intentionally downgraded for 16.5.

### Wiring

- `src/types.ts` `LocalTranslationCredentialStatus`, `src/api.ts`
  `saveLocalTranslationApiKey` / `deleteLocalTranslationApiKey` /
  `getLocalTranslationCredentialStatus` (api.ts remains the sole `invoke`
  caller), `MainApp.tsx` state + hydrate load + error-surfacing handlers.

## Tests

- Rust: key input validation truth table, credential-status resolution table
  (pure, no keychain), key-gate truth table (DeepSeek hosted, loopback IPv4/
  IPv6, empty key), DeepSeek URL normalization, offline-engine gate skip, and
  `probe_openai_engine_with_key` missing-key/loopback cases asserting message
  text — 8 new tests.
- Vitest (`LocalLlmSettings.test.tsx`): key row engine-conditional rendering,
  save/clear flows with refreshed status, failed save keeps the draft, empty
  password input on mount, env-override hint, preset fill/dirty/detection, and
  egress-note presence — 11 new tests.
- Full gates: `npm test`, `npm run build`, `cargo test`,
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.

## Security notes

- The key is never written to `local-translation-config.json`, never returned
  by any command (presence/source only), and never pre-filled into the DOM.
- Save/clear run through the same `keychain_error` mapping as the cloud keys;
  the existing `redact_error` tests keep covering client error paths.

## Verification matrix (Phase 03)

| Scenario | Result |
| --- | --- |
| Fresh config → DeepSeek preset + key → save, test, session | Covered by unit tests for preset fill, gate pass with key, normalization; live end-to-end needs a real key (manual) |
| Engine switch `openai_compatible` → `huggingface_offline` | `runtime_key_gate_ignores_offline_engine` — gate not consulted, key untouched |
| Loopback endpoint, no key | `openai_key_gate_truth_table` + `engine_test_skips_the_key_gate_for_loopback_endpoints` |
| Non-loopback, no key | Session start + engine test fail with `local_openai_api_key_missing` (gate + probe tests) |
| `BAKA_TRANS_LOCAL_API_KEY` set | Status resolution prefers environment (`credential_status_prefers_the_environment_variable`); documented override |
| Legacy v3→v4 migration | Existing migration tests untouched and green; no schema field changed |
| Cloud sessions | No diff under `src-tauri/src/ai/*`, `hy_mt.rs`, `llm.rs` |
