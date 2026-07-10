# Summary Agent Prompt Presets and Custom Instructions — 2026-07-10

## Context

Phase 17 requested selectable writing styles and user-authored summary instructions while retaining JSON-only output, source IDs, enabled sections, output language, and transcript-grounding rules (`plans/realtime-meeting-translation-macos/phase-17-summary-agent-prompt-presets-custom.md:10` and `plans/realtime-meeting-translation-macos/phase-17-summary-agent-prompt-presets-custom.md:12`).

## Change

Added matching TypeScript and Rust preset/config contracts for Balanced, Professional, Gentle, Detailed, Timeline, and Custom (`src/types.ts:35`, `src/types.ts:293`, `src-tauri/src/models.rs:683`, and `src-tauri/src/models.rs:712`). Frontend helpers now own preset metadata, an 8,000-character custom-prompt limit, validation, a Balanced default, and preset switching that retains custom text (`src/transcript.ts:15`, `src/transcript.ts:58`, `src/transcript.ts:84`, and `src/transcript.ts:344`); focused tests cover the catalog, boundaries, defaults, and retention (`src/transcript.test.ts:153`).

The Summary Agent panel now exposes the style selector, description, conditional custom editor, character count, inline errors, and run gating, with narrow-layout styling (`src/App.tsx:416`, `src/App.tsx:1630`, `src/styles.css:810`, and `src/styles.css:1696`). Rust resolves the selected instructions before provider access, prepends immutable schema/grounding rules, reuses the composed system message for every chunk, and rejects blank or over-limit custom prompts (`src-tauri/src/summary_agent.rs:43`, `src-tauri/src/summary_agent.rs:126`, `src-tauri/src/summary_agent.rs:139`, `src-tauri/src/summary_agent.rs:160`, and `src-tauri/src/summary_agent.rs:186`).

## Impact

Users can change meeting-note tone or organization without changing provider profiles or the structured result shape, and can supply custom instructions for the current app run. Risk level: medium, because prompts can influence provider output, but backend-owned invariants, validation before provider calls, and structured parsing continue to constrain schema and fabrication risk.

## Decision

Used stable serialized preset IDs with descriptive frontend metadata and executable Rust directives to keep the Tauri contract provider-neutral. Custom instructions are appended after, rather than substituted for, backend invariants; the same character limit is enforced on both sides. Balanced remains the compatibility default, custom text survives temporary preset changes in React state, and persistence or named prompt libraries remain outside this phase.

## References

- plan: [Phase 17 — Summary Agent Prompt Presets and Custom Instructions](../../plans/realtime-meeting-translation-macos/phase-17-summary-agent-prompt-presets-custom.md)
- source: `src/transcript.ts:15`
- source: `src/App.tsx:1630`
- source: `src-tauri/src/summary_agent.rs:126`
- source: `src-tauri/src/summary_agent.rs:186`
- commit: `76866d14885cb150ae11a481bb3d71149ccf8df1`
- commit: `aa192d6b47d83e2f0c484565113d85f4b3809c9d`
