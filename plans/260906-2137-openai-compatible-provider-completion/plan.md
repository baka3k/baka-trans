---
title: "OpenAI-Compatible Translation Provider Completion (DeepSeek-Ready)"
status: completed
created: 2026-09-06
updated: 2026-09-06
mode: hi-plan --full
---

# OpenAI-Compatible Translation Provider Completion (DeepSeek-Ready)

## Overview

Let the user configure and switch the local-Whisper translation engine between the
managed offline Hy-MT2 (HuggingFace) engine and any OpenAI-compatible Chat
Completions API — target provider: **DeepSeek `deepseek-v4-flash`** — with in-app
API key management, provider presets, and an actionable missing-key gate.

The engine switch itself already exists. This plan completes the missing
configuration surface so a hosted provider is actually usable end to end from the
app: today a DeepSeek key cannot be entered anywhere (load-only keychain read plus
the `BAKA_TRANS_LOCAL_API_KEY` env var), there are no presets, and a missing key
surfaces only as a per-utterance request failure instead of a start-time gate.

```text
Settings → Local engine = "OpenAI-compatible API"
  → provider preset (DeepSeek / Ollama / LM Studio / Custom)
      fills editable base URL + model
  → API key (password field) → OS keychain (service dev.baka3k.baka-trans)
  → Save config → Test translation engine
  → Start local session
      → start-time gate: non-loopback endpoint requires a key (env or keychain)
      → per utterance: POST <base>/v1/chat/completions (existing client, unchanged)
```

## Cross-Plan Coordination

- `plans/260716-2033-local-llm-audio-translation` (status: in_progress) owns the
  local translation engine area; its phase-16.2 log claims keychain save/delete
  commands, but current code has no save/delete path — this plan delivers that
  residual and is recorded as umbrella phase 16.5 in that plan's table. The
  umbrella's "egress warning acknowledged" runtime prerequisite is
  intentionally downgraded in 16.5 to advisory text in the settings UI with no
  persisted acknowledgment flag.
- No `blockedBy`: every prerequisite (engine dispatcher, OpenAI-compatible client,
  keychain load, settings UI) is implemented on `main`.
- Touches no cloud (`GoogleLiveTranslate`, `OpenaiRealtime`) path and no Hy-MT2
  sidecar/runtime code.

## Scope Challenge Decisions

1. **New engine variant `deepseek`, or reuse `openai_compatible`?**
   Reuse `openai_compatible`. DeepSeek's API is OpenAI-compatible
   (`POST {base}/v1/chat/completions`, bearer auth); the existing client, URL
   normalizer, and settings fields already model it. Adding an enum variant would
   fork four `match` sites and bump the config schema for zero behavioral gain.
   "DeepSeek" becomes a **frontend-only preset** that fills the editable base URL
   and model fields; the config schema stays at v4 and no migration is needed.
2. **Where does the API key live?**
   OS keychain via the existing `keyring` entry (service `dev.baka3k.baka-trans`,
   user `local-translation-api-key`), with `BAKA_TRANS_LOCAL_API_KEY` remaining the
   highest-priority source, matching the current load order in
   `src-tauri/src/local_translation/api_key.rs`. The key is never written to
   `local-translation-config.json`, never returned to the frontend (status only),
   and never logged (existing `redact_error` tests keep covering this).
3. **Is a key required, and what happens without one?**
   Required for non-loopback endpoints, optional for loopback (Ollama/LM Studio
   style local servers). Enforced at session start and at "Test translation
   engine" with a new `local_openai_api_key_missing` error — not at config save,
   because the validator cannot see env/keychain state atomically and env-only
   users must keep working. No per-utterance fallback to Hy-MT2 (preserves the
   umbrella plan's explicit no-silent-fallback decision).

## Current-State Evidence

Implemented and reusable as-is:

| Piece | Location |
| --- | --- |
| Engine enum `LocalTranslationEngine { huggingface_offline, openai_compatible }` (serde snake_case), schema v4 | `src-tauri/src/models.rs:683-689`, `local_translation.rs:20` |
| `TranslationClient` dispatcher selecting by `config.translation_engine` | `src-tauri/src/local_translation.rs:451-497` |
| Client loads keychain/env key and passes it to the OpenAI-compatible client | `src-tauri/src/local_translation.rs:475-483`, `:778-796` |
| `OpenAiCompatibleClient`: `POST <base>/v1/chat/completions`, bearer only when key present AND (https or loopback), TLS enforced non-loopback, no redirects, 1 MiB cap, error redaction, 21 unit tests | `src-tauri/src/local_translation/openai_compatible.rs:25-192, 283-298, 310-555` |
| Keychain/env load only (`load_local_translation_api_key`, env var wins, in-memory cache) | `src-tauri/src/local_translation/api_key.rs` |
| Settings UI engine dropdown + conditional URL/model/timeout/tokens/temperature fields + engine test | `src/components/settings/LocalLlmSettings.tsx:199-279` |
| Cloud-key precedent for save/status commands (`save_translation_api_key`, `ApiKeySource`, `api_key_source_label`) | `src-tauri/src/commands.rs:93-145, 357` |
| Config validate/normalize (`validate_openai_compatible_fields`: URL+model required when engine is openai_compatible) | `src-tauri/src/local_translation.rs:873-982` |
| Session start validates runtime config | `src-tauri/src/session.rs:382-386` |

Gaps (this plan's work):

1. No `save`/`delete`/`status` command for the local translation key; nothing in
   `src-tauri/src/lib.rs` command registration; no UI field in
   `LocalLlmSettings.tsx`. A DeepSeek key cannot be entered from the app today.
2. No provider presets; base URL and model must be typed from memory.
3. Missing key surfaces as a per-utterance `local_openai_request_error` mid-session
   instead of a start-time, actionable error.
4. No data-egress note for hosted endpoints (the umbrella plan lists egress
   acknowledgment as a local-mode runtime prerequisite).
5. Docs: README Local Whisper section and error-code table do not document the
   keychain entry or env var.

DeepSeek facts used for presets (verified 2026-09-06 against
[api-docs.deepseek.com](https://api-docs.deepseek.com/updates/) and current
pricing pages): V4 family is live; legacy `deepseek-chat`/`deepseek-reasoner`
names are deprecated in favor of `deepseek-v4-flash` / `deepseek-v4-pro`;
OpenAI-compatible base URL `https://api.deepseek.com` (the app's URL normalizer
appends `/v1/chat/completions`, which matches DeepSeek's documented base-URL
behavior); peak/off-peak pricing means per-utterance cost is small but nonzero.

## Architecture

No new backend module. The OpenAI-compatible client, dispatcher, and keychain
load path are reused unchanged; the work is (a) key write-path commands, (b)
preset metadata + validation gate, (c) settings UX, (d) docs/tests.

### Backend additions

```text
local_translation/api_key.rs
  + save_local_translation_api_key(key: &str) -> AppResult<()>      // trim, non-empty, ≤4096 chars, keyring set, cache update
  + delete_local_translation_api_key() -> AppResult<()>             // delete_credential, cache clear, NoEntry → Ok
  + local_translation_credential_status() -> AppResult<LocalCredentialStatus>  // { has_key, source: Option<ApiKeySource> }, no key material

commands.rs (run_blocking, like the cloud-key commands)
  + save_local_translation_api_key(api_key: String)
  + delete_local_translation_api_key()
  + local_translation_credential_status() -> LocalCredentialStatus

local_translation.rs
  + pure gate fn ensure_openai_key_available(endpoint_host, key: Option<&str>):
    engine == OpenaiCompatible && host non-loopback (is_loopback_url, exposed
    pub(crate) from openai_compatible.rs) && key == None
      → AppError "local_openai_api_key_missing" (actionable message naming
        both the settings field and BAKA_TRANS_LOCAL_API_KEY)
  ~ wired into validated_runtime_config() (session start) and
    probe_openai_engine() (both engine-test paths) with the loaded key
```

Loopback detection reuses the existing helper in `openai_compatible.rs` (same rule
that already permits keyless loopback requests and enforces TLS elsewhere).

### Frontend additions

```text
src/types.ts        + LocalTranslationCredentialStatus { hasKey, source: ApiKeySource | null }
src/api.ts          + saveLocalTranslationApiKey / deleteLocalTranslationApiKey /
                      getLocalTranslationCredentialStatus   (sole invoke caller)
src/components/settings/LocalLlmSettings.tsx
  + provider preset select (Custom | DeepSeek | Ollama | LM Studio)
      — fills editable openaiBaseUrl + openaiModel draft fields; never saved itself
  + API key row, rendered only when engine == openai_compatible:
      password input (never pre-filled), "Save key", "Clear key",
      status line "Key: macOS Keychain / Environment variable / Not set"
  + egress warning paragraph when the draft base URL host is non-loopback
```

Presets (frontend constant; all fields remain user-editable after selection):

| Preset | Base URL | Model | Key |
| --- | --- | --- | --- |
| DeepSeek | `https://api.deepseek.com` | `deepseek-v4-flash` | required |
| Ollama | `http://127.0.0.1:11434` | (user's installed model) | optional |
| LM Studio | `http://127.0.0.1:1234` | (user's loaded model) | optional |
| Custom | empty | empty | depends on host |

## Phases

| Phase | Document | Outcome |
| --- | --- | --- |
| 01 | [phase-01-local-api-key-management.md](phase-01-local-api-key-management.md) | Keychain save/delete/status commands + password field + status line in settings |
| 02 | [phase-02-deepseek-preset-and-key-gate.md](phase-02-deepseek-preset-and-key-gate.md) | Provider presets, start-time `local_openai_api_key_missing` gate, egress warning |
| 03 | [phase-03-regression-tests-and-docs.md](phase-03-regression-tests-and-docs.md) | Full test matrix, README/ARCHITECTURE/log updates, release gates |

## File Impact Map

| Area | Files |
| --- | --- |
| Key write path | `src-tauri/src/local_translation/api_key.rs` |
| Validation gate | `src-tauri/src/local_translation.rs` |
| Commands/registration | `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/models.rs` (status struct) |
| Frontend contracts/bridge | `src/types.ts`, `src/api.ts` |
| Settings UX | `src/components/settings/LocalLlmSettings.tsx` (+ colocated test) |
| Docs | `README.md`, `ARCHITECTURE.md`, new `docs/logs/2026-09-06-*.md` |
| Tests | `src-tauri/src/local_translation.rs` / `openai_compatible.rs` test modules, `src/components/settings/LocalLlmSettings.test.tsx` |

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Keyring unavailable/fails on some platform or CI | Cloud-key commands already use `keyring` in production; follow the existing `security.rs` error mapping (`keychain_error`); unit-test the trim/length/validation logic directly and keep keyring I/O behind the existing function seam |
| Users running keyless https gateways (non-loopback) break at session start | Error is explicit and actionable, env var remains an escape hatch, and loopback stays keyless; no silent behavior change mid-session (gate runs at start and test only) |
| Key leaks via logs, config JSON, or transcript errors | Key never enters `LocalTranslationConfig`; extend existing `redact_error` tests to cover the new paths; status command returns presence + source only |
| DeepSeek peak/off-peak latency spikes exceed timeout | Existing clamp (5–300 s, default from config) plus the established per-utterance error snapshot path; no queue growth (single-flight worker unchanged) |
| Preset fills a URL the normalizer rejects | Presets use exact hosts verified against `normalize_openai_chat_completions_url` tests; a preset test asserts normalization for each row |
| Config schema drift | No `LocalTranslationConfig` field added or removed; schema stays v4; existing v3→v4 migration tests must stay green |

## Out of Scope

- Streaming (`stream:true`), retry/backoff, automatic provider failover — the
  umbrella plan explicitly rejects per-utterance silent fallback.
- Multiple saved keys per provider, `/v1/models` listing, key expiry/rotation UX.
- Cloud engines (Google Live, OpenAI Realtime) and the `llm.rs` summary-agent
  profile system.
- Widening the hardcoded local `ja` source language.

## Success Criteria

- With only in-app actions: choose engine "OpenAI-compatible API", pick the
  DeepSeek preset, paste an API key, save it (status shows Keychain), save the
  config, run "Test translation engine" successfully, start a local session, and
  receive live translations with requests hitting
  `https://api.deepseek.com/v1/chat/completions`.
- Switching the engine back to "Offline Hy-MT2 1.8B" works and neither clears nor
  reads the stored key.
- Non-loopback engine with no key (env and keychain both empty): session start
  fails with the `local_openai_api_key_missing` code and "Test translation
  engine" fails with the same actionable message; the utterance pipeline never
  starts.
- Loopback engine without a key keeps working (Ollama regression).
- The key never appears in `local-translation-config.json`, logs, error messages,
  or the DOM after save (only presence/source status is rendered).
- `npm test`, `npm run build`, `cargo test`, `cargo fmt --check`, and
  `cargo clippy --all-targets -- -D warnings` all pass.

## Review and Validation

- Adversarial review: [reports/red-team.md](reports/red-team.md) — verdict
  **GO**, no blockers; findings 1-7 folded into the phase docs, 8-10 recorded
  as decisions/notes.
- Validation notes are embedded in Scope Challenge Decisions above; open product
  question carried to the user: none blocking — preset list contents (which local
  servers to ship) may be adjusted during Phase 02 review.

## Implementation Handoff

Suggested command:

```text
/hi-craft plans/260906-2137-openai-compatible-provider-completion/plan.md --full
```
