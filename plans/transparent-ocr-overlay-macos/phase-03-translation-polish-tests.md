# Phase 03: Text Translation Pipeline, Polish, and Tests

## Goal

Translate OCR text reliably and present it cleanly in the overlay.

## Tasks

1. Add a text translation adapter separate from the live audio translation runtime.
2. Implement the MVP adapter as `gemini_text`:
   - reuse the existing `GEMINI_API_KEY` credential path used by Google Live Translation
   - call Gemini over HTTPS from Rust with `reqwest`
   - do not use the Google Live audio WebSocket for OCR text
   - send only recognized OCR text, never screenshots
3. Use a strict translation prompt:
   - translate only the provided OCR text
   - preserve useful line breaks
   - return only translated text
   - keep terminology literal enough for UI/code/document text
4. Add translation debounce:
   - translate only when normalized OCR text changes
   - cancel or ignore stale translation jobs when the overlay moves again
   - cache recent source text to translated text mappings
5. Emit `overlay-translation-update` with source text, translated text, status, latency, provider, model, and error state.
6. Render overlay states:
   - permission needed
   - scanning
   - no text
   - translating
   - translated
   - error
7. Add UI controls that matter in daily use:
   - opacity slider
   - pause/resume scanning
   - copy translated text
   - optional source/translation toggle
8. Keep Google Cloud Translation API as a later optional adapter, not part of the MVP.
9. Add tests and smoke validation.

## Files Likely Touched

- `src-tauri/src/overlay.rs`
- `src-tauri/src/llm.rs` or a new text translation module
- `src-tauri/src/commands.rs`
- `src-tauri/src/models.rs`
- `src/App.tsx`
- `src/api.ts`
- `src/types.ts`
- `src/styles.css`

## Verification

- `npm run build`
- `npm test`
- `cargo test` from `src-tauri`
- Manual: drag overlay across text-heavy windows and confirm translation updates without repeated duplicate calls.
