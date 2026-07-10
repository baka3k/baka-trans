# Look & Help Overlay — 2026-07-10

## Context
Look & Help initially scanned continuously and placed controls around a single answer surface. The follow-up requested deliberate capture for accuracy, persistent visibility of the recognized screen text, request, and LLM response, and a layout where toolbars cannot cover text.

## Change
Replaced the timed scan loop with the one-shot `capture_look_help` command and guarded capture/OCR/LLM pipeline in `src-tauri/src/commands.rs:256` and `src-tauri/src/look_help.rs:173`. The frontend invokes it only from the Capture action (`src/api.ts:159`, `src/App.tsx:2089`, `src/App.tsx:2160`). Rebuilt the workspace as three persistent regions—Captured screen, Request, and LLM result—in `src/App.tsx:2209`, with independent scrolling, a spanning result panel, fixed action/status rows, and narrow-window fallback in `src/styles.css:1557`.

## Impact
Users can align the overlay before spending an OCR/LLM request, review exactly what OCR captured, edit the request, and keep the result visible until the next capture. Text regions now scroll inside reserved workspace panels instead of underneath controls. Risk level: medium, because the change alters capture lifecycle and the minimum/responsive overlay layout while retaining the existing OCR and LLM execution path.

## Decision
Made manual capture specific to Look & Help while leaving realtime Look through behavior unchanged. An explicit backend command was preferred over pausing a background loop so opening, moving, or editing the overlay cannot accidentally trigger a request. The three regions remain visible together and own their scrolling areas; profile and opacity stay in collapsible settings. Existing OCR-only LLM input, prompt-injection guardrails, caching, and stale-response suppression remain in place.

## References
- plan: `./plans/look-help-overlay-macos/plan.md`
