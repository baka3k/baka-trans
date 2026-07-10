# Phase 01: Shared Overlay Capture Foundation and Helper Mode Entry Point

## Goal

Prepare the existing overlay implementation so Look through and Look & Help can share capture/OCR/window primitives without duplicating the current `overlay.rs` runtime.

## Tasks

1. Introduce explicit overlay modes or separate labels:
   - `transparent-overlay` for Look through translation.
   - `look-help-overlay` for Look & Help assistant.
2. Extract shared capture/OCR helpers from `src-tauri/src/overlay.rs`:
   - screen recording permission detection/request.
   - macOS window id lookup.
   - region capture below the overlay window.
   - Apple Vision OCR and confidence filtering.
   - OCR text normalization and hashing.
3. Keep Look through behavior unchanged after extraction.
4. Add Rust state shell for Look & Help:
   - open/focus/close helper overlay window.
   - status query.
   - geometry update.
   - pause/resume.
5. Register Tauri commands and capabilities for the new helper overlay window.
6. Add frontend API wrappers and type stubs for Look & Help state.
7. Add main window **Look & Help** launcher beside **Look through**.

## Files

- `src-tauri/src/overlay.rs`
- Optional split: `src-tauri/src/overlay/capture.rs`, `src-tauri/src/overlay/translate.rs`, `src-tauri/src/overlay/help.rs`
- `src-tauri/src/models.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/default.json`
- `src-tauri/gen/schemas/capabilities.json`
- `src/App.tsx`
- `src/api.ts`
- `src/types.ts`

## Acceptance

- Existing Look through still opens, OCRs, and translates.
- New Look & Help button opens a distinct helper overlay route.
- Helper overlay can report geometry and display scanning/paused/permission states.
- No LLM call is required in this phase.

## Validation

- `cargo test`
- `npm test`
- `npm run build`
- Manual open/close/focus test for both overlay modes.
