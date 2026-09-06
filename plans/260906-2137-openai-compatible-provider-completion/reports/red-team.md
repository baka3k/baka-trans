# Red-Team Review — OpenAI-Compatible Provider Completion (2026-09-06)

Verdict: **GO** — no blockers. Findings 1-7 were folded into the phase docs
before handoff; 8-10 are recorded decisions/notes.

## Verified-correct load-bearing claims

- `api_key.rs` gap is real: only `load_local_translation_api_key`
  (`src-tauri/src/local_translation/api_key.rs:15-52`); the phase-16.2 log's
  save/delete claim is confirmed absent
  (`docs/logs/2026-08-15-phase-16-2-engine-config-and-api-client.md:44-45`).
- keyring 3.6.3 exposes `set_password` / `delete_credential`
  (`src-tauri/Cargo.lock:1834`; production use at `security.rs:30`).
- Cloud-key precedent fits: `commands.rs:93-98` (`run_blocking`),
  `translation_credential_status` at `commands.rs:111-115`, registration
  `lib.rs:58-63`.
- Loopback helper exists: `is_loopback_url`
  (`openai_compatible.rs:194-212`) — module-private; visibility change required.
- `validated_runtime_config()` at `local_translation.rs:510-513`, called from
  `session.rs:386`; failure path (`session.rs:121-141`) returns to Idle before
  capture — the gate seam is correct.
- Both engine-test paths funnel through `probe_openai_engine`
  (`local_translation.rs:771-789`, key load `:778`).
- Bare-origin normalization → `/v1/chat/completions`
  (`openai_compatible.rs:185-186`, test `:315-320`); live check:
  `POST https://api.deepseek.com/v1/chat/completions` → 401 (route exists,
  auth-gated). `deepseek-v4-flash` confirmed live per DeepSeek docs.

## Findings and disposition

1. **[MAJOR] Env-var/keychain-based gate tests would flake**: no existing
   env-var test pattern, no `serial_test` dep, and the process-global
   `CACHED_KEY` plus the real macOS keychain make "empty keychain" unreliable.
   → **Fixed**: Phase-02 gate re-specified as a pure decision function
   `ensure_openai_key_available(endpoint, key: Option<&str>)` with truth-table
   unit tests; env/keychain integration testing dropped from unit scope.
2. **[MINOR] Status contract contradicted `ApiKeySource`**: Rust serializes
   `"environment"|"keychain"|"memory"` (`models.rs:831-837`), TS mirror exists
   (`types.ts:5`); "none" is `Option`/`null` in the cloud precedent
   (`models.rs:583`, `types.ts:186`). → **Fixed**: status is
   `{ hasKey: boolean; source: ApiKeySource | null }`.
3. **[MINOR] `is_loopback_url` is module-private.** → **Fixed**: Phase-02 now
   specifies exposing it as `pub(crate)`.
4. **[MINOR] "api.ts is the only @tauri-apps/api importer" is false**
   (`MainApp.tsx:1-3` imports `isTauri`/`listen`). → **Fixed**: reworded to
   "only `invoke` caller".
5. **[MINOR] `LocalLlmSettings` is presentational; placement note referenced
   the wrong branch.** → **Fixed**: Phase-01 specifies new parent-wired props
   (`credentialStatus`, `onSaveKey`, `onClearKey`) matching the `onTestEngine`
   precedent; key row lives in the `openai_compatible` conditional branch.
6. **[MINOR] "Clear key" UX when `BAKA_TRANS_LOCAL_API_KEY` is set**: env wins
   at load, so clearing the keychain cannot take effect while the env var
   exists. → **Fixed**: hint shown when `source === "environment"`.
7. **[MINOR] Empty-key save error code**: reuse `credential_input_invalid`
   (cloud precedent `security.rs:24`) instead of the gate code.
   → **Fixed** in Phase-01.
8. **[MINOR/DECISION] Egress acknowledgment**: umbrella plan lists an
   acknowledged egress warning as a local-mode runtime prerequisite; 16.5
   intentionally ships it as advisory text with no persisted ack. Recorded in
   plan.md and the umbrella phase table.
9. **[NOTE] JS loopback check may differ from the Rust rule** (`0.0.0.0`,
   `[::1]`, `127.0.0.2`). Advisory-only; backend gate is authoritative. Noted
   in Phase-02.
10. **[NOTE] `TranslationEngineTestResult` carries `message` only, no error
    code** (`models.rs:715-719`): the `local_openai_api_key_missing` code is
    asserted on the session-start path only, never on the engine-test path.
