# Phase 03: Windows Overlays, Packaging, and Release Validation

Status: implemented; manual overlay and installer validation pending

## Goal

Reach feature parity for Look Through and Look & Help, then produce a supportable Windows installer.

## Implementation Scope

1. Extract the native portion of `src-tauri/src/overlay.rs` behind a capture/OCR backend interface while preserving the current shared state, translation, dedupe, and event paths.
2. Implement the Windows backend from `plans/transparent-ocr-overlay-macos/phase-04-windows-capture-ocr-backend.md`. The shipped first implementation uses direct GDI region capture plus Windows OCR to avoid a capture picker and keep the interaction automatic. Move to Windows Graphics Capture if manual validation finds protected/accelerated surfaces that GDI cannot read.
3. Implement Windows equivalents for overlay exclusion or frame-safe temporary hiding, always-on-top behavior, DPI-aware region mapping, multi-monitor coordinates, and permission/consent errors.
4. Reuse the backend for both Look Through and Look & Help; do not duplicate capture/OCR code.
5. Add Windows-facing error text and settings links without exposing macOS Screen Recording terminology.
6. Add icon/version metadata, NSIS upgrade/uninstall behavior, optional MSI production, artifact checksums, Authenticode signing hooks, and a Windows release guide.

## Verification

- Automated tests for geometry conversion, OCR normalization, capture error mapping, and shared overlay state.
- Manual overlay tests over browser, PDF, Teams/chat, images, and the app itself.
- Mixed-DPI multi-monitor test, overlay self-capture test, consent denial/cancel test, sleep/resume test, and repeated open/close test.
- Clean install, upgrade, uninstall, and SmartScreen/signature inspection on supported Windows versions.
- Full Windows CI plus macOS regression checks.

## Exit Criteria

The Windows installer supports the realtime translator and both overlay modes, with documented requirements, actionable failures, signed release hooks, and no macOS regression.
