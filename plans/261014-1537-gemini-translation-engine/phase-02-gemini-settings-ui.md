# Phase 02 — Gemini settings UI + frontend contracts

Status: implemented (2026-10-14, commit b8217a4)
Parent: [plan.md](plan.md)
Blocked by: phase 01
Test scope: Vitest + Testing Library (`src/components/settings/LocalLlmSettings.test.tsx`, `src/App.test.tsx` if wiring touches it). Expected size: **S**.

## Goal

Expose Gemini as the third engine option in the Local translation settings panel with a
model field, status-only Google key row, and egress advisory — following the existing
engine-conditional UI patterns.

## Tasks

1. **Types + api** — `src/types.ts`: extend
   `LocalTranslationEngine` with `"gemini"` (`types.ts:225`) and add `geminiModel` to
   the config draft type; extend the engine-test result union if needed (`types.ts:307`
   already types `engine` — add `"gemini"`). `src/api.ts`: reuse the existing provider
   save/status wrappers for the Google credential; add none if the Cloud panel already
   wraps `translation_credential_status(google)`.
2. **`src/components/settings/LocalLlmSettings.tsx`**
   - Third `<option value="gemini">Gemini (Google AI)</option>` in the engine select
     (`LocalLlmSettings.tsx:310-319`).
   - Conditional `gemini` field group:
     - Model text input bound to `draft.geminiModel` (placeholder
       `gemini-2.5-flash-lite`).
     - Status-only Google API key row (reuse the `ApiKeyRow` pattern,
       `LocalLlmSettings.tsx:933-1014`): shows has-key + source (env/keychain), save
       and clear via the existing provider-generic commands; the key is never echoed.
     - Egress advisory text (mirror `LocalLlmSettings.tsx:385-389`): this engine sends
       audio transcripts to Google's API.
   - `validateLocalTranslationDraft` (`:192-236`): `gemini` branch — model required
     after normalization (client-side mirrors the Rust normalize: trim, strip
     `models/`, empty → default is allowed); no URL fields.
   - "Test translation engine" button already generic — confirm it passes the draft
     through unchanged and renders the result row for `engine: "gemini"`.
3. **`src/app/MainApp.tsx`** — no new state expected: `engineTest`/`engineTesting` and
   dirty-reset are engine-agnostic (`MainApp.tsx:379-380, 2115-2123`). Verify only.

## Acceptance criteria → test mapping

| Rule | Test |
|---|---|
| Engine select offers exactly three options | render test asserting option list |
| Gemini branch shows model + key row + egress note; hidden for other engines | toggle test per engine value (table: 3 engines × field-group visibility) |
| Key row never echoes the key; save/clear call provider-generic commands | interaction test with mocked api module |
| Validation: empty model falls back to default; whitespace-only rejected with message | table-driven: typical / empty / `models/`-prefixed / whitespace |
| Engine test result row renders for `engine: "gemini"` | async test with mocked `testTranslationEngine` |

Budget: ~6 test functions, within the ceiling.

## Out of scope

- Any Cloud-mode credential UI changes; batching UI (no new user settings exist for it).
