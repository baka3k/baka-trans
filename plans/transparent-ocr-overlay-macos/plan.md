# Transparent OCR Translation Overlay for macOS

Status: active
Created: 2026-07-10
Source spec: user request in Codex thread, Vietnamese: add a "xuyen thau" button that opens a semi-transparent square window; wherever the user moves/resizes it, text underneath is translated and shown inside the window.
Mode: `hi-plan --fast`
Blocked by: `plans/realtime-meeting-translation-macos`
Blocks: `plans/look-help-overlay-macos`

## Objective

Add a macOS-only transparent overlay mode to Baka Trans. The main app exposes a "Xuyen thau" control. Clicking it opens a resizable, always-on-top, semi-transparent square overlay. The app captures the screen area under that overlay, OCRs visible text, translates changed text into the selected target language, and renders the translated result inside the overlay.

This plan starts with macOS because the current app and product notes are macOS-first. The feature is not inherently macOS-only; Windows support should be added through a separate platform capture/OCR backend after the macOS MVP proves the UX.

## Feasibility

This is feasible on macOS, with important constraints:

- The overlay window itself is straightforward in Tauri: create a secondary webview window with transparent background, no decorations, always-on-top behavior, fixed minimum size, and resize enabled.
- Capturing the text underneath the overlay is the real feature. It should use macOS screen capture APIs from the Rust side, not DOM inspection, because the underlying text can come from Teams, browser windows, PDFs, images, or any other app.
- OCR is required. The app cannot reliably "read" another app's text directly. Use Apple Vision text recognition for the MVP; it runs on-device and avoids sending screenshots to a cloud OCR service.
- The overlay must be excluded from the captured image, otherwise it will OCR its own translated text. If exclusion is not reliable in the first spike, temporarily hide the overlay for a frame while capturing or capture a slightly offset/background-only region.
- macOS Screen Recording permission is required. The app needs a permission detection and onboarding state before the overlay can work.
- Translation should use the existing Google/Gemini credential path for the MVP, but as a separate text translation request. Do not force OCR text through the Google Live audio WebSocket or the audio realtime session.

## Translation API Decision

Primary MVP choice: Gemini API text generation using the existing `GEMINI_API_KEY`.

- Use the same stored Google credential that powers `TranslationProvider::GoogleLiveTranslate`.
- Call Gemini over normal HTTPS from Rust with `reqwest`; do not open a Live API audio session for overlay text.
- Use a fast Gemini text model configured in app settings, with a strict prompt: translate only the OCR text, preserve line breaks when useful, return only translated text, and avoid commentary.
- This keeps the first implementation simple because the app already has Google credentials and target-language selection.
- This is also better than sending screenshots to a cloud model: OCR remains local through Apple Vision, and only recognized text is sent for translation.

Not selected for MVP:

- Google Live Translation: designed for low-latency audio interpreter sessions, not one-off OCR text snippets.
- OpenAI Realtime Translation: not needed for this overlay if the app default remains Google.
- Google Cloud Translation API: a good future adapter for deterministic text translation, glossaries, or enterprise Google Cloud setups, but it may require separate Cloud Translation enablement/auth and should not block the MVP.
- User-configured LLM profiles: useful as an advanced fallback later, but the default overlay path should not depend on summary-agent profile setup.

## Platform Support Decision

MVP: macOS only.

Reason: the current application is already scoped and packaged for macOS, and macOS has a clean first implementation path:

- Overlay window: Tauri secondary transparent always-on-top window.
- Screen capture: Apple ScreenCaptureKit.
- OCR: Apple Vision text recognition.
- Translation: Gemini HTTPS text translation using the existing `GEMINI_API_KEY`.

Windows: feasible, but not the same backend.

The shared pieces can stay the same:

- Tauri overlay UI.
- OCR/translation state model.
- Gemini text translation adapter.
- Dedupe/cache/debounce logic.

The platform-specific Windows backend should use:

- Overlay window: Tauri transparent always-on-top window.
- Screen capture: Windows Graphics Capture API.
- OCR: Windows.Media.Ocr or the newer Windows App SDK text recognition APIs.
- Translation: same Gemini HTTPS text translation adapter as macOS.

Recommended sequencing:

1. Build macOS MVP first.
2. Extract a trait/interface such as `OverlayCaptureBackend`.
3. Implement `MacOverlayCaptureBackend`.
4. Add `WindowsOverlayCaptureBackend` in a follow-up phase.

Important Windows constraints:

- Windows capture consent and picker behavior differs from macOS Screen Recording permission.
- Capturing the exact region under a transparent overlay may require capturing the full display/window and cropping in-process.
- The overlay must be excluded from OCR, or temporarily hidden during capture, just like on macOS.
- Some capture paths show a system capture border/indicator; this needs manual UX validation.
- Rust integration may require WinRT bindings or a helper crate around Windows Graphics Capture.

## Context Scan

- Existing active plan: `plans/realtime-meeting-translation-macos/plan.md`.
- The current app is Tauri v2 + React + TypeScript + Rust, macOS-scoped in `README.md` and `src-tauri/tauri.conf.json`.
- Frontend command bridge is centralized in `src/api.ts`; commands are registered in `src-tauri/src/commands.rs` and `src-tauri/src/lib.rs`.
- Translation/session state already emits `transcript-update` and audio events. The overlay feature should add its own OCR/overlay events so it does not overload meeting transcript semantics.
- `src-tauri/src/models.rs` already has `TranslationProvider`, `Language`, and provider-aware language validation. Reuse those types where possible.
- `src-tauri/src/session.rs` owns the audio translation runtime. The overlay should use a separate runtime such as `OverlayOcrRuntime` so audio session lifecycle and screen OCR lifecycle do not block each other.
- `mind_mcp` and `serena` were not exposed in this session. `graph_mcp` was available and found current `src/App.tsx`, transcript, and Tauri bridge symbols. Local source is the source of truth.
- There are pre-existing modified source files in the working tree. This plan intentionally touches only `plans/`.

## Recommended Architecture

```text
Main App Window
  -> "Xuyen thau" button
  -> Tauri command: open_overlay_window()

Overlay Window
  -> transparent, resizable, always-on-top
  -> emits move/resize region updates
  -> renders latest translated text and OCR status

Rust Overlay Runtime
  -> receive overlay bounds in global screen coordinates
  -> route capture/OCR through platform backend
  -> capture screen region under bounds
  -> run local OCR on captured bitmap
  -> normalize/dedupe OCR text
  -> translate changed text via text translation provider
  -> emit overlay-translation-update to overlay window
```

## Data Model

Overlay config:

- enabled flag
- platform capture backend: `mac_screen_capture_vision` for MVP; `windows_graphics_capture_ocr` later
- target language
- source language or auto
- capture interval in milliseconds
- minimum OCR confidence
- translation provider: `gemini_text` for MVP; optional future adapters `google_cloud_translate` and `llm_profile`
- Gemini text model ID
- overlay opacity
- pinned/click-through mode flag for later

Overlay capture region:

- display ID
- x, y, width, height in physical pixels
- scale factor
- updated_at_ms

Overlay OCR result:

- raw_text
- normalized_text
- confidence
- line boxes when available
- captured_at_ms

Overlay translation result:

- source_text
- translated_text
- status: idle, permission_needed, scanning, translating, translated, no_text, error
- latency_ms
- provider: `gemini_text` for MVP
- updated_at_ms

## Implementation Phases

1. Phase 01: Overlay window shell and lifecycle
2. Phase 02: Screen capture permission, region capture, and OCR spike
3. Phase 03: Text translation pipeline, dedupe, UI polish, and tests
4. Phase 04: Windows capture/OCR backend after macOS MVP
5. Phase 05: Realtime overlay workspace redesign

### Phase 05: Realtime Workspace Redesign

- Preserve the existing polling, OCR, dedupe, Gemini translation, and pause/resume runtime. Look through remains realtime and does not gain a manual Capture action.
- Replace the single translation surface and collapsible source preview with two persistent regions: **Detected screen** and **Translation**.
- Give both text regions independent scrolling areas so title, status, settings, and metadata controls cannot cover text.
- Move pause/resume into the realtime status row and keep close/settings controls in the title bar.
- Move opacity into a compact collapsible settings row and keep copy translation in the Translation panel header.
- Increase the default and minimum overlay size for the two-region workspace while retaining an explicit compact-window layout.
- Keep Look through visually related to Look & Help while preserving its green realtime identity and distinct capture behavior.

## Scope Decisions

- MVP is macOS only, but the feature should be designed with a platform backend boundary so Windows can be added later.
- MVP translates visible text, not semantic UI objects.
- MVP uses a polling loop with debouncing, not continuous video OCR on every frame.
- MVP uses Gemini text generation through the existing Google/Gemini API key for translation.
- MVP shows translated text inside the overlay. It does not replace or draw over the original text underneath.
- MVP requires Screen Recording permission and should fail gracefully with a clear status.
- Do not store screenshots by default. Keep captured images in memory and discard immediately after OCR.
- Do not add Windows support in the macOS MVP implementation. Add it as a follow-up backend once the overlay UX and translation pipeline are stable.

## Risks

- ScreenCaptureKit integration from Rust may require an Objective-C/Swift bridge or a Rust crate with enough macOS API coverage.
- OCR quality depends on source text size, contrast, animation, and whether the underlying app uses images or anti-aliased text.
- Overlay capture can accidentally include the overlay itself unless excluded or hidden during capture.
- Frequent OCR plus translation calls can become expensive or jittery. Add text hashing, debounce, and a minimum capture interval.
- Screen Recording permission changes may require app restart or macOS Settings intervention.

## Acceptance Criteria

- Main window has a visible "Xuyen thau" control.
- Clicking it opens exactly one overlay window, or focuses the existing one.
- Overlay is resizable, movable, semi-transparent, and stays above normal app windows.
- The app detects missing Screen Recording permission and shows overlay status instead of failing silently.
- Moving/resizing the overlay changes the capture region.
- Text visible under the overlay is OCRed and translated into the selected target language.
- Detected OCR text and translated output remain visible in separate regions during realtime updates.
- Pause/resume stops and restarts realtime detection without changing the workspace layout.
- Toolbars remain outside the scrollable text regions and do not obscure content at the minimum window size.
- Repeated identical OCR text does not trigger repeated translation requests.
- Captured screenshots are not persisted to disk by default.
- Overlay can be closed without stopping the existing audio translation session.

## Validation

- Manual test with text in browser, PDF, image-based text, and Teams/chat window.
- Manual test with overlay moved across multiple regions of the same display.
- Manual test with empty region and rapidly changing region.
- Unit tests for OCR text normalization/dedupe and overlay state reducer.
- Rust tests for model serialization and text hash/debounce behavior.
- App build/test: `npm run build`, `npm test`, and `cargo test` from `src-tauri`.

## References

- https://developer.apple.com/documentation/screencapturekit/
- https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-in-macos
- https://developer.apple.com/documentation/vision/recognizing-text-in-images
- https://learn.microsoft.com/en-us/windows/apps/develop/media-authoring-processing/screen-capture
- https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture
- https://learn.microsoft.com/en-us/uwp/api/windows.media.ocr
- https://learn.microsoft.com/en-us/windows/ai/apis/text-recognition
- https://v2.tauri.app/learn/window-customization/
- https://v2.tauri.app/reference/javascript/api/namespacewindow/
