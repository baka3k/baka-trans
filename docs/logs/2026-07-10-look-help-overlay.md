# Look & Help Overlay — 2026-07-10
## Context
Implemented `plans/look-help-overlay-macos/plan.md`, adding a second macOS OCR overlay mode that reads the screen region beneath a transparent window and asks a configured LLM profile for help instead of translating through Gemini.

## Change
Added the Look & Help backend state machine, persisted helper config, prompt construction, LLM profile execution, dedupe cache, and stale-response suppression in `src-tauri/src/look_help.rs:1`. Registered Tauri state and commands in `src-tauri/src/lib.rs:14` and `src-tauri/src/commands.rs:216`. Added frontend DTOs/API wrappers in `src/types.ts:119` and `src/api.ts:139`, plus the main launcher and dedicated overlay UI in `src/App.tsx:874` and `src/App.tsx:1871`.

## Impact
Users can open Look & Help independently from Look through, select or persist an LLM profile, edit the system prompt in-overlay, and get answers from OCR text without sending screenshots. Risk level: medium, because the feature touches macOS capture, LLM calls, and always-on-top overlay UX.

## Decision
Kept Look & Help as a sibling state machine instead of rewriting the existing translation overlay. This reuses shared OCR/capture helpers while limiting regression risk for Look through. OCR text is explicitly framed as untrusted context in the system message, and blank/error regions invalidate in-flight helper requests.

## References
- plan: `./plans/look-help-overlay-macos/plan.md`
- commit: `c167857`
