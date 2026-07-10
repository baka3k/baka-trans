# Phase 02: Screen Capture Permission, Region Capture, and OCR

## Goal

Capture the screen region underneath the overlay and extract text on-device.

## Tasks

1. Add macOS permission checks for Screen Recording.
2. Add a permission-needed overlay state with a button or instruction to open macOS Settings when permission is missing.
3. Implement `OverlayCaptureService` in Rust:
   - accepts overlay geometry
   - converts logical window bounds to physical display pixels
   - captures only the region under the overlay
   - avoids capturing the overlay itself
4. Implement `OverlayOcrService`:
   - uses Apple Vision text recognition where available
   - returns recognized lines, confidence, and normalized text
5. Add a polling/debounce loop:
   - capture interval default around 500-1000 ms
   - skip OCR when geometry has not changed and recent text hash is unchanged
   - emit `overlay-ocr-update`
6. Add memory/privacy guardrails:
   - screenshots are in memory only
   - no debug image dumps unless explicitly enabled in development

## Files Likely Touched

- `src-tauri/src/overlay.rs` or `src-tauri/src/overlay/mod.rs`
- `src-tauri/src/models.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/error.rs`
- `src/App.tsx`
- `src/types.ts`

## Verification

- Unit tests for geometry conversion, text normalization, and dedupe hashing.
- Manual: region over normal text, image text, empty area, and high-DPI display.
- Manual: permission missing path and permission granted path.

