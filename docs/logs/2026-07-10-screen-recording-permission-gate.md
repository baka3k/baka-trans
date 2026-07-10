# Screen Recording Permission Gate — 2026-07-10

## Context

Since `12e29b7`, Look Through and Look & Help have shared the macOS capture/OCR path required to read browsers and other applications (`plans/transparent-ocr-overlay-macos/plan.md:20`, `plans/look-help-overlay-macos/plan.md:19`). The previous path only converted an empty capture into a permission error, so macOS privacy-limited captures could continue without exposing the missing Screen Recording grant.

## Change

Commit `34b3696a153c1c79a25b2dfd2aa0ce2900c15b86` adds an external-app-specific permission message and one-shot request state in `src-tauri/src/overlay.rs:37` and `src-tauri/src/overlay.rs:40`. Capture now preflights or requests Screen Recording access, then rejects capture before CoreGraphics/Vision work when access is unavailable (`src-tauri/src/overlay.rs:442`, `src-tauri/src/overlay.rs:629`, `src-tauri/src/overlay.rs:640`, `src-tauri/src/overlay.rs:657`). A regression test covers both denied and granted gate outcomes at `src-tauri/src/overlay.rs:684`.

## Impact

Look Through and Look & Help now surface the actual macOS permission requirement instead of appearing to work only on Baka Trans content. Risk level: medium, because both overlay modes share this gate and macOS TCC may still require the user to quit and reopen the exact app binary; native browser capture remains a manual runtime check (`plans/transparent-ocr-overlay-macos/plan.md:226`, `plans/look-help-overlay-macos/plan.md:189`). The targeted regression test passed with 1 test passed, 0 failed, and 51 filtered out.

## Decision

Gate capture on `CGPreflightScreenCaptureAccess`, request access at most once per process with `CGRequestScreenCaptureAccess`, and return the existing `screen_recording_permission_needed` error immediately. This replaces inference from `overlay_capture_empty`, which cannot reliably distinguish a missing TCC grant from a valid empty region, while preserving the shared capture design required by Look & Help (`plans/look-help-overlay-macos/phase-01-shared-overlay-capture-foundation.md:12`).

## References

- plan: [Transparent OCR overlay](../../plans/transparent-ocr-overlay-macos/plan.md)
- plan: [Look & Help overlay](../../plans/look-help-overlay-macos/plan.md)
- plan: [Shared overlay capture foundation](../../plans/look-help-overlay-macos/phase-01-shared-overlay-capture-foundation.md)
- commit: `34b3696a153c1c79a25b2dfd2aa0ce2900c15b86`
