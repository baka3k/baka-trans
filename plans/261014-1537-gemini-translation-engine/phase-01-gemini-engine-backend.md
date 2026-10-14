# Phase 01 — Gemini engine backend (schema v5, client, gate, engine test)

Status: implemented (2026-10-14, commit b8217a4)
Parent: [plan.md](plan.md)
Test scope: Rust unit tests (inline `#[cfg(test)]`), mapped acceptance criteria below. Expected size: **M**.

## Goal

Add `LocalTranslationEngine::Gemini` end to end on the Rust side so the engine is
constructible, gated on the Google credential, testable from the settings panel, and
able to translate a JSON array of sentences in one `generateContent` call.

## Tasks

1. **Enum + config contract**
   - `src-tauri/src/models.rs`: add `Gemini` to `LocalTranslationEngine` (serde
     `gemini`); add `gemini_model: String` to `LocalTranslationConfig` +
     `LocalTranslationConfigDraft` with serde default `gemini-2.5-flash-lite`.
   - `src-tauri/src/local_translation.rs`: bump `CONFIG_SCHEMA_VERSION` 4→5 with a
     v4→v5 migration (old configs default `gemini_model`), following the existing
     migration table style (`local_translation.rs:1094-1239`).
   - `normalize_and_validate`: validate the Gemini branch via a new
     `validate_gemini_fields` (early-return shape like
     `validate_openai_compatible_fields`, `local_translation.rs:1028-1050`): trim
     `gemini_model`, strip a leading `models/`, fall back to the default when empty.
2. **Client module `src-tauri/src/local_translation/gemini.rs`**
   - POST `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`.
   - Auth: `x-goog-api-key` **header** (do not copy the overlay's `?key=` query param).
   - `reqwest` client: explicit timeout, `redirect(Policy::none())`, 1 MiB response cap —
     mirror `openai_compatible.rs:10-11, 56-60`.
   - Request body: `system_instruction` = cabin-interpretation prompt (const, see
     below), `contents[0].parts[0].text` = JSON array of source sentences,
     `generationConfig: { temperature: 0.3, maxOutputTokens: 4096,
     response_mime_type: "application/json" }`. Latency hardening: verify against the
     current Google API docs at implementation time whether
     `generationConfig.thinkingConfig.thinkingBudget: 0` applies to the chosen model
     (`unknown` — 2.5-family thinking defaults are not evidenced in-repo); set it if
     supported to keep interpretation latency flat.
   - System prompt (parameterize source/target language names from the session
     `Language`s, do not hardcode English→Vietnamese):
     expert simultaneous cabin interpreter; read the chunk list as one continuous flow;
     stitch fragments cut awkwardly by ASR; return a valid JSON array of translated
     strings matching the input length and order exactly; natural, conversational,
     fluid interpretation tone; no markdown fences, no explanations.
   - Response parse: take `candidates[0].content.parts[].text` joined (reuse the
     `overlay.rs:415-441` shape), strip optional code fences, `serde_json` parse to
     `Vec<String>`; empty/short/long handling per plan Scope Decision 7.
   - Errors: `local_gemini_request_error`, `local_gemini_api_error`,
     `local_gemini_parse_error`, `local_gemini_empty_response`,
     `local_gemini_batch_length_mismatch`; run every message through the redaction
     helper (`openai_compatible.rs:284-299` pattern) so the key never appears.
   - `translate_batch(&[String]) -> AppResult<Vec<(String, u64)>>` on this client;
     single-element batches still use the JSON-array contract (one code path).
3. **Dispatcher wiring** (`local_translation.rs`)
   - `TranslationClient::Gemini(GeminiClient)` variant; construction arm in
     `TranslationClient::new` loading the key via
     `security::load_translation_api_key(TranslationProvider::GoogleLiveTranslate)`;
     `translate` dispatch arm; `translate_batch` dispatch (default sequential impl for
     the other two engines keeps today's behavior byte-for-byte).
   - Runtime gate: `ensure_gemini_key_available` in `enforce_runtime_key_gate` —
     Gemini always requires a key; error `local_gemini_api_key_missing` with a message
     naming both fixes (Cloud settings panel or `GEMINI_API_KEY`).
   - `test_engine` arm: validate the credential with
     `GET /v1beta/models` + `x-goog-api-key` (cheap key probe, prior art
     `ai/google_live.rs:32-51`); map HTTP outcome to
     `TranslationEngineTestResult { reachable, accepted, message }` with the
     `local_gemini_*` error family; endpoint reported as the generateContent URL.
4. **No session.rs changes** — verify local sessions flow through `TranslationClient`
   untouched (`session.rs:466-510` only matches cloud providers).

## Acceptance criteria → test mapping

| Rule (plan decision) | Test |
|---|---|
| v5 migration defaults `gemini_model` | migration test: v4 config JSON → v5 with default model (table row joins the existing migration table) |
| Model normalization (`models/` prefix, empty → default) | table-driven: typical / `models/x` prefix / empty |
| Gate: missing Google key blocks session start | gate truth-table row → `local_gemini_api_key_missing` |
| Request shape: header auth, JSON-array body, JSON mode, temp 0.3 | request-shape test against a local mock server (fixture pattern from `openai_compatible.rs:311-556`) |
| Parse: clean array / fenced array / non-array / empty parts | table-driven parse tests (4 rows) |
| Length mismatch mapping (short/long) | boundary test: fewer items → missing indexes error snapshot values; extra → truncated |
| Key never in error strings | redaction test: inject key into a mock error body, assert absent |
| Engine test arm maps HTTP outcomes | reachable-yes / reachable-no / key-rejected table |

Budget: ~11 test functions (several table-driven), within the 10-15 ceiling.

## Out of scope

- Batch window in the worker (phase 03); settings UI (phase 02); overlay `?key=`
  migration; making OpenAI-compatible batch-capable.
