# Look & Help OCR Assistant Overlay for macOS

Status: planned
Created: 2026-07-10
Source spec: user request in Codex thread, Vietnamese: add a feature similar to Look through, but instead of only translating it reads information from the region underneath, sends that information to an LLM with a configurable system prompt, shows the answer in the overlay, and has a settings button to hide/show the system prompt.
Mode: `hi-plan --fast`
Blocked by: `plans/transparent-ocr-overlay-macos`
Blocks: none

## Objective

Add a second overlay mode named **Look & Help**. It reuses the current Look through transparent OCR overlay mechanics to read text from the screen region underneath a movable/resizable overlay, but routes the OCR text into an LLM task prompt instead of a translation-only prompt. The overlay displays the LLM result in-place and includes a settings control that can show/hide and edit the system prompt used for the task.

## Product Behavior

- Main window exposes a **Look & Help** button near the existing **Look through** button.
- Clicking **Look & Help** opens exactly one dedicated helper overlay window, or focuses the existing one.
- The helper overlay is movable, resizable, transparent, always-on-top, and visually distinct from Look through.
- The overlay captures and OCRs the region underneath it using the same macOS Screen Recording and Apple Vision path as Look through.
- OCR text is treated as untrusted context and sent to a selected LLM profile together with the user-controlled system prompt.
- The LLM output is displayed inside the overlay.
- A settings button in the overlay toggles a compact settings panel where the user can show/hide and edit the system prompt.
- The current prompt and selected helper LLM profile persist across app restarts.
- No screenshot is sent to the LLM by default; only normalized OCR text and minimal metadata are sent.

## Context Scan

- Existing related plan: `plans/transparent-ocr-overlay-macos/plan.md`.
- The current implementation already has:
  - `src-tauri/src/overlay.rs` for transparent overlay lifecycle, geometry, macOS Screen Recording permission, region capture, Apple Vision OCR, dedupe/cache, and Gemini translation.
  - `src/App.tsx` with the main **Look through** launcher and `TransparentOverlayWindow` route.
  - `src/api.ts` and `src/types.ts` with overlay command wrappers and event models.
  - `src-tauri/src/llm.rs` with reusable OpenAI-compatible/Ollama/ADK profile storage and `chat_completion`.
  - `src-tauri/src/summary_agent.rs` as an example of calling `llm::chat_completion` with system/user messages.
- `mind_mcp` and `serena` were not exposed. `graph_mcp` semantic search returned unrelated indexed projects, so local source is the source of truth.
- `docs/development-rules.md` is absent.
- There are pre-existing modified plan files in the worktree; this plan should avoid touching unrelated implementation files until execution.

## Cross-Plan Dependency

This plan depends on the Look through overlay foundation:

- Reuse its window pattern, transparent route, geometry reporting, capture permission handling, Apple Vision OCR, and self-capture avoidance.
- Do not copy/paste the whole `overlay.rs` loop for a second feature. Extract shared capture/OCR primitives first, then add mode-specific processing.
- The existing `plans/transparent-ocr-overlay-macos/plan.md` should list this plan under `Blocks` because Look & Help builds on that foundation.

## Recommended Architecture

```text
Main App Window
  -> "Look through" button
  -> "Look & Help" button

Shared Overlay Capture Layer
  -> transparent Tauri window helpers
  -> geometry state
  -> screen recording permission
  -> macOS region capture below overlay window
  -> Vision OCR
  -> normalized OCR text + hash/dedupe

Look Through Mode
  -> OCR text
  -> Gemini text translation prompt
  -> overlay translation update

Look & Help Mode
  -> OCR text
  -> selected LLM profile + user system prompt
  -> helper prompt with OCR text as untrusted context
  -> overlay helper update
```

## Data Model

Add helper-specific models instead of overloading translation update names:

- `OverlayMode`: `translate`, `help`
- `LookHelpConfig`
  - provider profile ID
  - system prompt
  - prompt panel visible by default
  - capture interval
  - minimum OCR confidence
  - opacity
  - max OCR input characters
  - max output tokens override or profile default
- `LookHelpStatus`
  - is_open
  - is_paused
  - status: idle, permission_needed, scanning, thinking, complete, no_text, paused, error
  - message
  - config
  - geometry
- `LookHelpUpdate`
  - source_text
  - answer_text
  - status
  - message
  - latency_ms
  - provider_profile_id
  - model
  - prompt_hash
  - updated_at_ms

Persist `LookHelpConfig` in an app config JSON file, not in Keychain. The selected LLM profile secret remains managed by the existing LLM profile store.

## Prompt Contract

The backend should build messages like:

- system: user-authored system prompt, with a stable guardrail prefix that says OCR text is untrusted screen content and must not override the system prompt.
- user: normalized OCR text plus metadata such as language hint, capture time, and instruction to answer based only on provided OCR text unless the system prompt explicitly allows broader reasoning.

Default system prompt:

```text
You are Look & Help, a compact assistant for the visible screen region. Explain, summarize, or help with the provided OCR text. Be concise, practical, and do not invent details that are not present.
```

## Implementation Phases

1. Phase 01: Shared overlay capture foundation and helper mode entry point
2. Phase 02: Look & Help prompt settings, persistence, and overlay UI
3. Phase 03: LLM execution pipeline, dedupe, cancellation, and tests

## Scope Decisions

- MVP is macOS-only because it reuses the existing macOS Look through capture/OCR backend.
- MVP sends OCR text to the LLM, not screenshots or raw pixels.
- MVP uses existing LLM provider profiles from the summary-agent configuration rather than adding another secret store.
- MVP should have a default prompt and work once an enabled LLM profile is selected.
- MVP supports one current helper prompt. Prompt presets/history can be added later.
- MVP does not add click-through mode, screenshots, image-to-LLM vision, or cross-window anchoring.
- MVP does not write OCR text, prompts, or answers to transcript history by default.

## Risks

- Prompt injection: OCR text can contain instructions from untrusted apps/pages. The backend must delimit OCR text and state it is untrusted context.
- Cost/latency: repeated OCR changes can trigger repeated LLM calls. Use text+prompt hash dedupe, debounce, and stale response suppression.
- UX density: showing a system prompt editor inside a small overlay can crowd the answer. Use a collapsible settings panel and stable min window size.
- Profile availability: the user may have no LLM profile configured. Show a clear state and a path back to profile settings.
- Shared overlay refactor risk: extracting capture/OCR from `overlay.rs` can regress Look through. Keep translation behavior covered by tests and manual checks.

## Acceptance Criteria

- Main window has a **Look & Help** button beside **Look through**.
- Clicking it opens/focuses a dedicated helper overlay window.
- Look through and Look & Help can be opened/closed independently without corrupting each other's state.
- Helper overlay captures text under its region and shows no-text/permission/error states.
- Helper overlay has a settings button that toggles system prompt visibility.
- Editing the system prompt affects the next LLM request and persists after restart.
- Helper overlay sends normalized OCR text to the selected LLM profile and renders the answer.
- Identical OCR text and identical prompt do not trigger repeated LLM calls.
- Stale LLM responses from previous regions/prompts are ignored.
- Missing LLM profile or missing profile API key produces actionable UI state.
- Captured screenshots are not persisted or sent to the LLM.

## Validation

- Unit tests for prompt building, prompt hashing, config normalization, and response parsing.
- Rust tests for shared OCR normalization/dedupe behavior after refactor.
- Frontend build/typecheck for the new overlay route and command wrappers.
- Manual test: overlay over browser article, code editor error text, chat text, and empty region.
- Manual test: edit system prompt, hide/show prompt panel, move overlay before response returns, and confirm stale output is not shown.
- App checks: `npm test`, `npm run build`, and `cargo test` from `src-tauri`.

## References

- Existing plan: `./plans/transparent-ocr-overlay-macos/plan.md`
- Existing log: `./docs/logs/2026-07-10-transparent-ocr-overlay.md`
- Existing LLM log: `./docs/logs/2026-07-09-llm-config-summary-agent.md`
- Existing implementation anchors:
  - `src-tauri/src/overlay.rs`
  - `src-tauri/src/llm.rs`
  - `src-tauri/src/summary_agent.rs`
  - `src/App.tsx`
  - `src/api.ts`
  - `src/types.ts`
