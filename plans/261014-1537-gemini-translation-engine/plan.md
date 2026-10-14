# Gemini Translation Engine + Batch Translation Optimization

Status: planned
Created: 2026-10-14
Mode: `hi-plan --full`
Blocked by: none (phase 16.2/16.3 of the parent plan are implemented)
Blocks: none
Parent plan: [plans/260716-2033-local-llm-audio-translation/plan.md](../260716-2033-local-llm-audio-translation/plan.md) (registered as phase 16.6)
Coordinates with: OCR overlay Gemini text translation (`src-tauri/src/overlay.rs`) — shares the Google credential

## Objective

Add **Gemini** (Google `generativelanguage` REST `generateContent`) as a third selectable
translation engine in the Local Whisper workspace, alongside Offline model (Hy-MT2 /
TranslateGemma 4B) and OpenAI-compatible API. The Gemini engine translates **batches of
Whisper sentences as one JSON-array request** with a cabin-interpretation system prompt
(context retention, stitching fragmented ASR chunks, natural conversational tone,
`temperature 0.3`), and per-index 1:1 mapping back to transcript items.

## Context Scan (research digest, verified against source)

- Engine enum: `LocalTranslationEngine { HuggingfaceOffline, OpenaiCompatible }`
  (`src-tauri/src/models.rs:701-707`, serde snake_case) + TS mirror
  `src/types.ts:225`. Both gain a third variant `Gemini` / `"gemini"`.
- Dispatcher is a Rust enum, not a trait: `TranslationClient { OpenAiCompatible, HyMt }`
  (`src-tauri/src/local_translation.rs:352-356`); construction keyed on
  `config.translation_engine` (`local_translation.rs:465-494`); `translate(&str) ->
  (String, latency_ms)` dispatch (`local_translation.rs:496-502`). A third variant needs
  one construction arm + one dispatch arm.
- Per-sentence hot loop: `spawn_translation_worker`
  (`src-tauri/src/ai/local_worker.rs:454-557`) pulls one `TranslationJob` at a time and
  calls `client.translate()` (`local_worker.rs:484`). Stable transcript ids
  (`utterance_id[:index]`, `local_worker.rs:805-811`) + `upsert_snapshot`
  (`local_worker.rs:778-791`) mean a batch call can emit per-item Final/Error snapshots
  without any transcript/event contract change.
- Gemini REST prior art exists in the OCR overlay: `translate_with_gemini`
  (`src-tauri/src/overlay.rs:375-413`) POSTs
  `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`,
  `parse_gemini_text` (`overlay.rs:415-441`). **No `response_mime_type:
  "application/json"` usage exists yet** — that is new work.
- Credential: `security.rs` stores the Google key as keyring user
  `google-gemini-api-key` (service `dev.baka3k.baka-trans`), env override
  `GEMINI_API_KEY`, load order env → keychain → in-memory cache
  (`src-tauri/src/security.rs:5-9, 73-108`). Generic commands already exist:
  `save_translation_api_key(provider, key)` and `translation_credential_status(provider)`
  (`src-tauri/src/commands.rs:94-115`). The overlay already reuses this key for Gemini
  text translation — one Google credential for all Google usage.
- Key gate pattern: `enforce_runtime_key_gate` / `ensure_openai_key_available`
  (`local_translation.rs:521-545`), called from session start (`session.rs:385-395`).
  Engine test path: `test_engine` (`local_translation.rs:726-737`) →
  `TranslationEngineTestResult` (`models.rs:732-741`); Tauri command
  `test_translation_engine` (`commands.rs:180-186`).
- Config contract: `LocalTranslationConfig(Draft)` camelCase + `#[serde(default)]`
  (`models.rs:603-660`); schema version constant `CONFIG_SCHEMA_VERSION = 4` with a
  migration history v1→v4 and inline migration tests (`local_translation.rs:20,
  1094-1239, 1382-1617`). Adding optional fields + defaulting model requires a v5 bump.
- Settings UI: engine `<select>` with two options
  (`src/components/settings/LocalLlmSettings.tsx:310-319`), engine-conditional field
  groups, egress advisory precedent (`LocalLlmSettings.tsx:385-389`), status-only
  `ApiKeyRow` (`LocalLlmSettings.tsx:933-1014`), validation
  `validateLocalTranslationDraft` (`LocalLlmSettings.tsx:192-236`).
- Engine-specific validation pattern: `validate_openai_compatible_fields` early-return
  unless engine matches (`local_translation.rs:1028-1050`).
- JSON repair precedent: `llm.rs::parse_json_object` handles fenced/embedded JSON
  (`src-tauri/src/llm.rs:324-359`) — reuse the fence-stripping idea for the array
  response.
- Session layer needs **no changes**: dispatch is per-`TranslationProvider`
  (`session.rs:466-510`); local engines are transparent behind `TranslationClient`.

## Scope Challenge Decisions

1. **Plan placement** — new plan directory cross-linked into the parent plan's phase
   table (exact precedent: phase 16.5 → `260906-2137` delegated plan). The parent plan
   stays the owner of shared local-pipeline contracts; this plan owns the Gemini engine
   end to end.
2. **Credential reuse vs new keyring entry** — reuse the existing Google credential
   (`google-gemini-api-key` / `GEMINI_API_KEY`). Prior art: the OCR overlay already
   translates with this key; `save_translation_api_key` is provider-generic. A dedicated
   local entry would force two keys for one Google account. Gate error message must
   point at the Cloud settings panel so a cleared cloud key does not confuse local users.
3. **Batch scope** — batch JSON-array translation ships **Gemini-only first**.
   `TranslationClient` gains `translate_batch` whose default implementation loops
   `translate()` per item (Hy-MT2 / OpenAI keep today's behavior and latency exactly);
   the window accumulator in the worker engages only for engines that declare a batch
   window. No OpenAI-compatible prompt change in this plan.
4. **Model configurability** — optional `geminiModel` config field, default
   `gemini-2.5-flash-lite`, free-text like the overlay's `geminiModel`; requires
   schema v4→v5. Temperature `0.3` and max output tokens stay engine constants (not
   user settings) to minimize churn.
5. **Transport security** — authenticate with the `x-goog-api-key` **header**, not the
   overlay's `?key=` query param (query params leak into URLs/logs). Overlay migration
   to the header is out of scope but noted as follow-up.
6. **Batch flush policy** — window size 5 sentences, max wait 500 ms (bounded extra
   latency vs the 1-3 s end-to-end promise in README; active speech fills the window
   fast, slow speech pays ≤500 ms). Constants in code with a comment, not settings.
7. **No auto-retry / no silent fallback** — on batch request failure every item in the
   window gets an Error snapshot (same semantics as today's per-sentence failure).
   Length mismatch: map returned indexes; missing indexes get Error snapshots with
   `local_gemini_batch_length_mismatch`; extra results are truncated.

## Risks

| Risk | Mitigation |
|------|------------|
| "Local Whisper" workspace is marketed offline (`ModeChooser.tsx:38-40`) | Egress advisory in the Gemini field group, mirroring the OpenAI-compatible note (`LocalLlmSettings.tsx:385-389`) |
| Batch window adds latency to the first sentence of a window | 500 ms max wait; window fills immediately during active speech; constant documented |
| JSON array output drift (fences, prose, length mismatch) | `response_mime_type: application/json` + fence-strip repair + length-mismatch error mapping; covered by table-driven parse tests |
| Key deleted from Cloud settings breaks local sessions with a confusing error | Gate error `local_gemini_api_key_missing` names both fixes (set key in Cloud settings or env `GEMINI_API_KEY`) |
| `TranslationEngineTestResult.engine` is a free `String` in Rust but a TS union (`types.ts:307`) | Add `"gemini"` to the TS union in the same change set |
| `?key=` precedent in overlay (`overlay.rs:388`) invites copy-paste | Plan mandates the header; redact keys from all error strings |

## Phases

| # | Phase doc | Scope |
|---|-----------|-------|
| 1 | [phase-01-gemini-engine-backend.md](phase-01-gemini-engine-backend.md) | `LocalTranslationEngine::Gemini`, schema v5, `local_translation/gemini.rs` client (JSON-array + system prompt), key gate, engine test arm, Rust tests |
| 2 | [phase-02-gemini-settings-ui.md](phase-02-gemini-settings-ui.md) | Third engine option, conditional field group (model, key row, egress note), validation, api.ts/types.ts, Vitest |
| 3 | [phase-03-batch-translation-optimization.md](phase-03-batch-translation-optimization.md) | Batch window in `spawn_translation_worker` (Gemini-only), per-index snapshot mapping, docs, full regression |

## Gates

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (in `src-tauri/`)
- `npm test`, `npm run build`
- Engine test button returns `engine_accepted` for a real key; session start gate fails with `local_gemini_api_key_missing` when the key is absent.

## Test Budget

Per `dev-shared/test-budget.md`: mapped behaviors only, `{typical, boundary, invalid}`
per dimension, table-driven where cases repeat, soft ceiling ~10-15 new test functions
per phase. Phase docs carry the rule → test checklist.
