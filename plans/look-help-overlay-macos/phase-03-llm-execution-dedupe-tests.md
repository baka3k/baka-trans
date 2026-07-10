# Phase 03: LLM Execution Pipeline, Dedupe, Cancellation, and Tests

## Goal

Route OCR text through the selected LLM profile with the configured system prompt, render answers in the helper overlay, and make repeated captures safe, bounded, and testable.

## Tasks

1. Implement prompt builder:
   - stable guardrail prefix.
   - user system prompt.
   - OCR text as delimited untrusted context.
   - max input character trimming.
   - optional capture metadata.
2. Call `llm::chat_completion` with the selected profile:
   - `json_output = false` for free-form helper answers.
   - use profile timeout/max tokens unless helper config overrides them.
3. Add dedupe and debounce:
   - hash normalized OCR text plus system prompt plus profile ID.
   - skip repeated identical inputs.
   - cache recent helper answers.
4. Add stale response protection:
   - sequence ID or request hash stored before call.
   - ignore LLM response if geometry/prompt/profile/text changed before completion.
5. Emit helper-specific events:
   - `look-help-status-update`
   - `look-help-update`
6. Render answer updates in the overlay:
   - thinking state while awaiting LLM.
   - error state with provider message.
   - copy answer/source controls if useful.
7. Add tests:
   - prompt builder includes guardrails and delimits OCR.
   - max OCR input trimming.
   - hash changes when prompt/profile/text changes.
   - stale response handling.
   - config normalization.

## Files

- `src-tauri/src/overlay/help.rs` or `src-tauri/src/overlay.rs`
- `src-tauri/src/llm.rs`
- `src-tauri/src/models.rs`
- `src/App.tsx`
- `src/types.ts`
- `src/api.ts`
- `src/styles.css`

## Acceptance

- Helper overlay displays LLM answers for OCR text under the region.
- Repeated identical OCR/prompt/profile combinations do not call the LLM repeatedly.
- Moving the overlay or changing the prompt before a response returns does not show stale output.
- Provider/profile errors show clear status in the overlay.
- Tests cover prompt safety and runtime dedupe behavior.

## Validation

- `cargo test`
- `npm test`
- `npm run build`
- Manual: run with an Ollama or OpenAI-compatible profile over browser/code/chat text.
- Manual: move overlay rapidly while a request is pending and confirm stale answers are ignored.
