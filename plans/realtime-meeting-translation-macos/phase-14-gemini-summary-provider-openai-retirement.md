# Phase 14: Gemini Summary Provider and Final OpenAI Retirement

Status: planned
Depends on: phases 11, 12, 13
Primary files: `src-tauri/src/llm.rs`, `src-tauri/src/summary_agent.rs`, `src-tauri/src/models.rs`, `src/App.tsx`, `src/transcript.ts`, `README.md`, `ARCHITECTURE.md`

## Objective

Complete the service migration by making Google usable for meeting summaries and by retiring OpenAI-first defaults, labels, tests, and docs after Google translation parity is proven.

## Implementation Steps

1. Add Gemini summary provider support.
   - Add `google_gemini` or equivalent to `LlmProviderKind`.
   - Choose one implementation path:
     - Native Gemini API for structured JSON summaries.
     - Google OpenAI-compatible endpoint only if it supports the required chat-completions behavior and JSON reliability.
   - Keep the summary-agent prompt/output contract independent of provider.

2. Update summary defaults.
   - Offer a Google Gemini default profile when `GEMINI_API_KEY` or Keychain credential exists.
   - Preserve OpenAI-compatible and Ollama profiles as optional alternatives.
   - Avoid requiring Google Live API credentials for summary if the user wants a separate key/profile.

3. Clean product language.
   - Replace OpenAI-first labels in Settings, README, architecture docs, and errors.
   - Rename "OpenAI key" compatibility commands only after frontend/backward compatibility is handled.
   - Document Google as the primary path and OpenAI as legacy/optional until removal.

4. Remove or deprecate OpenAI translation.
   - Keep OpenAI provider until Google passes smoke, soak, and live meeting validation.
   - After acceptance, mark OpenAI translation as legacy or remove it in a separate change.
   - Do not remove OpenAI-compatible summary profiles unless the user explicitly wants no non-Google providers.

## Acceptance Criteria

- The summary agent can run with a Google Gemini provider.
- The app can complete the core workflow using Google only: capture, translate, playback, transcript, and meeting summary.
- New-user defaults point to Google.
- OpenAI-specific UI text is removed or clearly marked as legacy.
- Documentation reflects Google Live API constraints and setup.

## Verification

- Unit tests for Gemini provider request building and response parsing.
- Summary-agent smoke test with a saved Gemini profile.
- Full manual workflow with Google translation plus Google summary.
- Final `rg -n "OpenAI|openai|OPENAI"` review to confirm only intentional legacy references remain.
