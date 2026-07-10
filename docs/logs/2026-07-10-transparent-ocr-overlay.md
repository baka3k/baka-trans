# Transparent OCR Overlay — 2026-07-10
## Context
The transparent OCR overlay plan at `plans/transparent-ocr-overlay-macos/plan.md` needed the macOS overlay shell, region capture, OCR, and translation flow implemented in the desktop app.

## Change
Added a dedicated overlay backend with Tauri window lifecycle, state, status events, capture loop, Gemini text translation, caching, and macOS Screen Recording permission handling in `src-tauri/src/overlay.rs:71` and `src-tauri/src/overlay.rs:236`. The macOS OCR path captures below the overlay window and runs Apple Vision text recognition in `src-tauri/src/overlay.rs:447`, with the overlay window id resolved from AppKit in `src-tauri/src/overlay.rs:523`.

Added overlay models and commands in `src-tauri/src/models.rs:294` and `src-tauri/src/commands.rs:204`, registered the state/commands in `src-tauri/src/lib.rs:13`, and enabled transparent/private macOS window support plus the overlay capability in `src-tauri/tauri.conf.json:13` and `src-tauri/capabilities/default.json:5`.

Added the launcher, overlay route, draggable transparent UI, pause/copy/permission controls, geometry reporting, and event listeners in `src/App.tsx:811` and `src/App.tsx:1523`, with API wrappers in `src/api.ts:132` and overlay styling in `src/styles.css:1217`.

## Impact
Users can open a transparent always-on-top region overlay, place it over on-screen text, and receive OCR-backed Gemini translations without routing audio through a meeting session. Risk level: medium, because the feature uses macOS Screen Recording permission, CoreGraphics capture, Vision OCR, and a live Gemini text request path.

## Decision
Used native CoreGraphics plus Vision instead of bundling an external OCR binary to keep the macOS path local and lightweight. Capturing below the overlay window avoids OCR feedback from the translated overlay text, and the window is also content-protected for extra self-capture resistance. Translation reuses the existing Google translation credential path to avoid adding a second secret store.

## References
- plan: ./plans/transparent-ocr-overlay-macos/plan.md
- commit: a3877c4
