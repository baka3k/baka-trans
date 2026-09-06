# Phase 02 — DeepSeek Preset, Key Gate, and Egress Warning

Parent: [plan.md](plan.md)

Depends on: Phase 01 (status command feeds the UI warning; the gate reports the
missing key the UI helps fix).

Red-team adjustments applied: the gate is a pure decision function with
truth-table tests (no env-var/keychain integration tests — none exist in this
repo and the global key cache would flake), `is_loopback_url` visibility change
is explicit, the JS loopback mirror is documented as advisory-only, and the
engine-test path asserts on message text rather than an error code.

## Objective

Make selecting a hosted provider a two-click affair and fail fast — at session
start and engine test, never mid-session — when a non-loopback endpoint has no
key. No config schema change, no new engine variant.

## Preset Metadata (frontend-only)

`src/components/settings/LocalLlmSettings.tsx` (constant next to the draft
defaults; if the list grows, move to `src/translationProviders.ts` with a
colocated test):

```ts
const TRANSLATION_PROVIDER_PRESETS = [
  { id: "custom",   label: "Custom",   baseUrl: "",                         model: "",                  },
  { id: "deepseek", label: "DeepSeek", baseUrl: "https://api.deepseek.com", model: "deepseek-v4-flash", },
  { id: "ollama",   label: "Ollama",   baseUrl: "http://127.0.0.1:11434",   model: "",                  },
  { id: "lmstudio", label: "LM Studio", baseUrl: "http://127.0.0.1:1234",   model: "",                  },
] as const;
```

- A `<select>` above the Server URL field; choosing a preset writes
  `openaiBaseUrl` / `openaiModel` into the **editable draft** (user can still
  change the model, e.g. `deepseek-v4-pro`). It is not persisted as a config
  field.
- Selecting a preset with empty URL/model (Custom) must not clobber typed
  values until the user explicitly picks it — derive the initial select value
  from a match against the current draft fields.

## Start-Time Key Gate (Rust)

`src-tauri/src/local_translation.rs`:

- Add a pure decision function (no I/O, trivially testable):

  ```rust
  fn ensure_openai_key_available(
      endpoint: &str,
      key: Option<&str>,
  ) -> AppResult<()>
  ```

  It errors with `local_openai_api_key_missing` when the endpoint host is
  non-loopback and `key` is `None`/empty, with a message naming both fixes
  (the settings key field; `BAKA_TRANS_LOCAL_API_KEY`).
- Loopback detection calls `is_loopback_url`
  (`openai_compatible.rs:194-212`) — change its visibility to `pub(crate)`
  within the `local_translation` module tree so gate and client share one rule
  and can never disagree.
- Wire the gate at the two seams, passing the key loaded from
  `api_key::load_local_translation_api_key()`:
  - `validated_runtime_config()` (`local_translation.rs:510-513`, called from
    `session.rs:386`) — session start fails before capture/Whisper begin; the
    existing failure path (`session.rs:121-141`) returns cleanly to Idle.
  - `probe_openai_engine()` (`:771-789`) — shared by `test_config` and
    `test_engine`, so the engine test checks the key before spending a network
    round trip. Note: `TranslationEngineTestResult` carries `message` only
    (`models.rs:715-719`), so the test path surfaces the gate as message text;
    the `local_openai_api_key_missing` code is asserted on the session-start
    path only.
- Loopback endpoints keep working keyless (Ollama/LM Studio regression).
- Save-time `normalize_and_validate` stays key-agnostic (it cannot observe
  env/keychain atomically; env-only setups must keep validating).

`src-tauri/src/error.rs` / README error table: register
`local_openai_api_key_missing`.

## Egress Warning (frontend)

In `LocalLlmSettings.tsx`, when engine is `openai_compatible` and the draft
base-URL host looks non-loopback (JS check for `localhost`, `127.*`, `::1`;
advisory only — it may differ from the Rust rule at the edges, e.g. `0.0.0.0`
or `127.0.0.2`, and the backend gate is authoritative):

> "Sentences from your meeting transcript will be sent to this endpoint for
> translation."

Static advisory text, `role="note"`, no persisted acknowledgment flag in this
iteration — the umbrella plan's "egress warning acknowledged" runtime
prerequisite is intentionally downgraded to advisory in 16.5 (recorded in
[plan.md](plan.md) and the umbrella phase table).

## Tests

Rust — truth table on the pure function; no environment manipulation:

| Endpoint | Key | Result |
| --- | --- | --- |
| `https://api.deepseek.com/v1/chat/completions` | `None` | `local_openai_api_key_missing` |
| same | `Some("sk-…")` | `Ok` |
| `http://127.0.0.1:11434/v1/chat/completions` | `None` | `Ok` |
| `http://[::1]:11434/…` | `None` | `Ok` |
| any | `Some("")` | treated as missing → error for non-loopback |

Plus: `https://api.deepseek.com` normalizes to
`https://api.deepseek.com/v1/chat/completions` (extends the existing
`normalize_openai_chat_completions_url` test table at `:315-320`); engine
`huggingface_offline` never reaches the gate; the `probe_openai_engine` wiring
is covered by its existing fake-runtime test pattern extended with a
missing-key case asserting the message text.

Vitest:

- Preset select fills URL + model; Custom appears when the draft matches no
  preset; switching presets updates the draft and marks the form dirty.
- Egress note appears for `https://api.deepseek.com`, absent for
  `http://127.0.0.1:11434`.

## Acceptance

- With a real DeepSeek key: Test engine succeeds; a short local session
  produces live translations; request URL is
  `https://api.deepseek.com/v1/chat/completions` (observe via proxy/log level
  agreed in review, never logging the key).
- Without any key: session start fails immediately with
  `local_openai_api_key_missing` and an actionable message; no utterances are
  consumed.
- Ollama loopback without key still translates.
