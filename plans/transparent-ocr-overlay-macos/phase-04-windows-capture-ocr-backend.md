# Phase 04: Windows Capture and OCR Backend

## Goal

Add Windows support for the transparent OCR overlay after the macOS MVP is stable.

## Decision

Windows should reuse the same overlay UI and Gemini text translation pipeline, but it needs its own capture/OCR backend.

## Backend Shape

Introduce or finalize a platform interface such as:

- `OverlayCaptureBackend`
- `capture_region(region) -> CapturedFrame`
- `recognize_text(frame) -> OverlayOcrResult`
- `permission_status() -> OverlayPermissionStatus`

Implement:

- macOS: ScreenCaptureKit + Apple Vision.
- Windows: Windows Graphics Capture + Windows.Media.Ocr or Windows App SDK text recognition.

## Windows Tasks

1. Add Windows platform gating in Rust.
2. Implement capture through Windows Graphics Capture.
3. Convert overlay bounds to display pixel coordinates and crop the captured frame.
4. Run local OCR through Windows.Media.Ocr or Windows App SDK text recognition.
5. Ensure the overlay's own translated text is not OCRed.
6. Validate Windows capture permission/consent behavior.
7. Reuse the existing `gemini_text` adapter for translation.
8. Add Windows-specific manual validation cases.

## Risks

- Windows capture APIs may require a picker or explicit user consent flow that feels different from macOS.
- The capture session may include a border/indicator depending on Windows version and capture mode.
- Rust/WinRT integration may require extra crates or a small Windows-specific helper module.
- Multi-monitor and display scaling behavior must be validated separately from macOS.

## Verification

- Manual: overlay over browser text, PDF text, chat text, and image text.
- Manual: move overlay across monitors with different scale factors.
- Manual: permission denied/cancelled flow.
- Manual: confirm OCR never re-reads the overlay translation itself.
- `npm run build`
- `npm test`
- Windows Rust test/build path once Windows CI or a Windows machine is available.

