# Phase 01 — Local Translation API Key Management

Parent: [plan.md](plan.md)

Red-team adjustments applied: status type reuses the existing `ApiKeySource`
contract, save-input errors use `credential_input_invalid`, the settings
component stays presentational (parent-wired props), and unit tests avoid the
real keychain and the process-global key cache.

## Objective

Give the OpenAI-compatible engine a complete key lifecycle: save to the OS
keychain, delete, and report presence/source — from both the backend command
layer and the settings UI. After this phase a user can paste a DeepSeek key in
the app; Phases 02-03 add presets and enforcement.

## Backend Changes

`src-tauri/src/local_translation/api_key.rs` (next to the existing
`load_local_translation_api_key`, same `SERVICE` / `LOCAL_TRANSLATION_KEY_USER` /
`ENV_VAR_NAME` constants and `key_cache()`):

- `save_local_translation_api_key(key: &str) -> AppResult<()>`
  Trim; reject empty and >4096 chars with `credential_input_invalid`
  (matches the cloud-key input-validation precedent at `security.rs:24`'s
  role, without overloading the Phase-02 gate code);
  `Entry::set_password` (keyring 3.6.3, same API as `security.rs:30`);
  refresh `key_cache()`.
- `delete_local_translation_api_key() -> AppResult<()>`
  `Entry::delete_credential`; `NoEntry` maps to `Ok(())`; clear `key_cache()`.
- `local_translation_credential_status() -> AppResult<LocalCredentialStatus>`
  Resolution order identical to load (env → cache → keychain); returns
  `{ has_key: bool, source: Option<ApiKeySource> }` — mirroring the existing
  `TranslationCredentialStatus` shape (`models.rs:583`, `src/types.ts:186`),
  where `ApiKeySource` serializes to `"environment" | "keychain" | "memory"`
  (`models.rs:831-837`) and `None`/`null` means no key. Never returns key
  material.

`src-tauri/src/models.rs`: add `LocalCredentialStatus` (serde camelCase)
reusing `ApiKeySource`.

`src-tauri/src/commands.rs` (mirror `save_translation_api_key` at
`commands.rs:93-98`, `run_blocking`):

- `save_local_translation_api_key(api_key: String)`
- `delete_local_translation_api_key()`
- `local_translation_credential_status() -> LocalCredentialStatus`

`src-tauri/src/lib.rs`: register all three in `invoke_handler`.

## Frontend Changes

- `src/types.ts`: `LocalTranslationCredentialStatus { hasKey: boolean;
  source: ApiKeySource | null }` using the existing `ApiKeySource` union
  (`src/types.ts:5`).
- `src/api.ts`: `saveLocalTranslationApiKey`, `deleteLocalTranslationApiKey`,
  `getLocalTranslationCredentialStatus` (`api.ts` stays the sole `invoke`
  caller; `MainApp.tsx` legitimately imports other `@tauri-apps/api` members
  and is not touched).
- `src/components/settings/LocalLlmSettings.tsx` is presentational: it owns no
  state and makes no Tauri calls — every side effect arrives as a parent
  callback (`LocalLlmSettings.tsx:52-87`, parent `MainApp.tsx`). Follow that:
  - New props: `credentialStatus: LocalTranslationCredentialStatus | null`,
    `onSaveKey(key: string): Promise<void>`, `onClearKey(): Promise<void>`;
    `MainApp.tsx` implements them over `src/api.ts` and loads the status when
    the local settings surface opens (same moment the config is fetched).
  - Render the key row inside the existing `engine === "openai_compatible"`
    conditional branch (`LocalLlmSettings.tsx:222-266`):
    `<label>` + `<input type="password" autoComplete="new-password">`, never
    pre-filled; "Save key" and "Clear key" buttons calling the props.
  - Status line (`aria-live="polite"`, existing row conventions):
    `Key: OS Keychain | Environment variable | Not set`.
  - When `source === "environment"`, render a hint that an environment
    variable currently overrides the keychain entry (the env var wins at
    load, `api_key.rs:16-31`), so "Clear key" clearing the keychain will not
    change the key in use until the variable is removed.
  - Keep Fluent tokens and existing class conventions (`key-test-row`-style
    rows); no new design system.

## Tests

Rust — pure logic only; the keyring I/O stays behind the existing function
seam (there is no keyring test seam in this repo, and the module-global
`CACHED_KEY` plus the real macOS keychain would make integration tests flaky
and destructive):

- Input validation: empty, whitespace-only, and >4096-char keys return
  `credential_input_invalid` without touching the Entry.
- Status resolution as a pure function of (env present, cached, entry result):
  env wins; `None`/`source: None` when all absent.

Vitest (`LocalLlmSettings.test.tsx` + `MainApp`-level wiring where the props
live):

- Key row renders only when engine is `openai_compatible`.
- "Save key" calls `onSaveKey` with the typed value, clears the input, and the
  status line reflects the refreshed prop.
- "Clear key" is disabled when `!hasKey`; after a clear the status reads
  "Not set".
- The password input is empty after mount even when `hasKey` is true.
- With `source === "environment"`, the override hint is rendered.

## Acceptance

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `npm test`, `npm run build` pass.
- Manual (not unit-tested): save a dummy key, status shows OS Keychain, delete
  returns to "Not set", the entry appears under service
  `dev.baka3k.baka-trans` / user `local-translation-api-key`, and
  `local-translation-config.json` contains no key material.
