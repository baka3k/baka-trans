# Transparent OCR Overlay — 2026-07-10
## Context
The transparent OCR overlay plan at `plans/transparent-ocr-overlay-macos/plan.md` needed the macOS overlay shell, region capture, OCR, and translation flow implemented in the desktop app.

## Change
Added a dedicated overlay backend with Tauri window lifecycle, state, status events, capture loop, Gemini text translation, caching, and macOS Screen Recording permission handling in `src-tauri/src/overlay.rs:71` and `src-tauri/src/overlay.rs:236`. The macOS OCR path captures below the overlay window and runs Apple Vision text recognition in `src-tauri/src/overlay.rs:447`, with the overlay window id resolved from AppKit in `src-tauri/src/overlay.rs:523`.

Added overlay models and commands in `src-tauri/src/models.rs:294` and `src-tauri/src/commands.rs:204`, registered the state/commands in `src-tauri/src/lib.rs:13`, and enabled transparent/private macOS window support plus the overlay capability in `src-tauri/tauri.conf.json:13` and `src-tauri/capabilities/default.json:5`.

Added the launcher, overlay route, draggable transparent UI, pause/copy/permission controls, geometry reporting, and event listeners in `src/App.tsx:811` and `src/App.tsx:1523`, with API wrappers in `src/api.ts:132` and overlay styling in `src/styles.css:1217`.

Phase 05 reorganized Look Through into persistent **Detected screen** and **Translation** panels in `src/App.tsx:1960` and `src/App.tsx:1981`. The existing `overlay-status-update` and `overlay-translation-update` listeners remain the data path in `src/App.tsx:1840`; the polling OCR/translation loop was intentionally left unchanged, so Look Through remains realtime and has no manual capture action (`plans/transparent-ocr-overlay-macos/plan.md:166`).

Pause/resume now sits in the realtime status row in `src/App.tsx:1931`, while the title-bar settings action reveals a compact opacity row in `src/App.tsx:1911` and `src/App.tsx:1941`. The workspace and panels use bounded grid rows plus independently scrollable panel bodies in `src/styles.css:1615`, `src/styles.css:1638`, and `src/styles.css:1685`, keeping toolbars outside the text surfaces. The native overlay window now opens at 480x560 with a 360x420 minimum in `src-tauri/src/overlay.rs:95`, backed by the compact layout rules in `src/styles.css:1954`.

Corrected title-bar action hit testing to accept SVG descendants as interactive elements in `src/App.tsx:177`, so clicking the settings and close icons no longer starts a window drag. Validation recorded in `plans/transparent-ocr-overlay-macos/plan.md:219` passed 26 frontend tests, the TypeScript/Vite production build, Rust formatting, and 49 Rust tests; browser checks also passed at 480x560 and 360x420 with opacity settings expanded.

## Impact
Users can open a transparent always-on-top region overlay, place it over on-screen text, and receive OCR-backed Gemini translations without routing audio through a meeting session. The persistent split workspace makes both detected text and its translation inspectable during realtime updates, including at the supported compact size, without controls obscuring either text surface. Risk level: medium, because the feature still depends on macOS Screen Recording permission, CoreGraphics capture, Vision OCR, and a live Gemini text request path; those native paths still require a manual Tauri runtime check (`plans/transparent-ocr-overlay-macos/plan.md:226`).

## Decision
Used native CoreGraphics plus Vision instead of bundling an external OCR binary to keep the macOS path local and lightweight. Capturing below the overlay window avoids OCR feedback from the translated overlay text, and the window is also content-protected for extra self-capture resistance. Translation reuses the existing Google translation credential path to avoid adding a second secret store.

Kept Look Through realtime because its purpose is continuous detection as the window moves, while borrowing the separated workspace structure from Look & Help. Persistent panels and independent scrolling were chosen over collapsible text previews so source and translation remain comparable; only low-frequency opacity controls collapse. Pause/resume stays beside live status to make the runtime state explicit without changing the OCR/translation lifecycle (`plans/transparent-ocr-overlay-macos/plan.md:168`).

## References
- plan: ./plans/transparent-ocr-overlay-macos/plan.md
- commit: a3877c40b113b226a5bcfb4895992f8fe81a00f9
- commit: e4ca0429b4c5d040e27d8ec1376df7dda0b580b6
