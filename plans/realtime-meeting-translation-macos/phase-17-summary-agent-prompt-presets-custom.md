# Phase 17: Summary Agent Prompt Presets and Custom Instructions

Status: planned
Depends on: phase 09
Compatible with: phases 14, 15, 16
Primary files: `src/types.ts`, `src/transcript.ts`, `src/transcript.test.ts`, `src/App.tsx`, `src/styles.css`, `src-tauri/src/models.rs`, `src-tauri/src/summary_agent.rs`

## Objective

Let users choose how MeetingSummaryAgent writes its notes without changing the provider profile or the structured result format. Provide concise built-in choices for balanced, professional, gentle, detailed, and timeline-oriented summaries, plus a custom system-prompt editor for user-authored instructions.

The selected style must apply to every transcript chunk while the backend continues to enforce JSON-only output, source item IDs, enabled sections, output language, and the rule that owners, dates, decisions, and timestamps cannot be invented.

## Current State

- `MeetingSummaryConfig` currently carries provider, trigger, transcript scope, output language, section toggles, transcript limit, and rolling-memory state, but no prompt choice.
- `src/transcript.ts` builds a single default config and `src/App.tsx` keeps it in React state for the current app run.
- `src-tauri/src/summary_agent.rs::system_prompt()` is hard-coded and is used for every transcript chunk.
- The current Rust prompt combines two different concerns: invariant response/grounding rules and the desired writing style. They must be separated before custom instructions are accepted.
- Summary responses must still parse into `MeetingSummaryResult`; this feature does not introduce free-form output.

## Prompt Catalog

Use stable IDs so the Tauri contract does not depend on display labels:

| ID | UI label | Intended behavior |
| --- | --- | --- |
| `balanced` | Balanced | Clear, concise meeting notes with neutral tone; remains the default. |
| `professional` | Professional | Business-ready language, direct decisions, risks, and accountable action items. |
| `gentle` | Gentle | Warm, tactful, non-judgmental wording without weakening facts or blockers. |
| `detailed` | Detailed | Preserve context, rationale, dependencies, open questions, and important nuance. |
| `timeline` | Timeline | Organize events and milestones chronologically; include times only when present in the transcript. |
| `custom` | Custom | Apply the user's non-empty custom system prompt within the invariant contract. |

Preset copy should be short and provider-neutral. The timeline preset must explicitly say not to infer missing dates, times, or sequence.

## Data Contract

Add a mirrored Rust/TypeScript preset type:

```ts
export type MeetingSummaryPromptPreset =
  | "balanced"
  | "professional"
  | "gentle"
  | "detailed"
  | "timeline"
  | "custom";

export interface MeetingSummaryConfig {
  // existing fields remain unchanged
  promptPreset: MeetingSummaryPromptPreset;
  customSystemPrompt: string;
}
```

Rust should use a `MeetingSummaryPromptPreset` enum with `snake_case` serialization and the same values. `buildMeetingSummaryConfig()` defaults to `balanced` and an empty custom prompt, preserving current behavior for new runs.

Keep `customSystemPrompt` in the existing non-secret run config. Do not place it in an LLM provider profile, Keychain entry, transcript item, exported notes, status event, or log. Cross-restart persistence for all Summary Agent options is outside this focused phase; the choice remains in the existing React `summaryConfig` state for the app run.

## Backend Prompt Composition

Replace the monolithic `system_prompt()` with small, testable prompt-building functions:

1. An invariant prefix owned by the backend:
   - identifies MeetingSummaryAgent.
   - requires the existing JSON keys and action-item shape.
   - requires source item IDs where applicable.
   - forbids invented owners, due dates, decisions, timestamps, and facts.
   - states that style instructions cannot override the schema or grounding rules.
2. A preset resolver that maps each built-in ID to its provider-neutral style instructions.
3. A custom resolver that trims the input, rejects blank custom prompts, and enforces a documented character limit such as 8,000 characters.
4. A composer that appends the resolved style/custom instructions after the invariant prefix under a clearly delimited `User-selected summary instructions` section.

Use the composed system message for every chunk and rolling-memory step. Keep transcript text only in the existing user message; do not interpolate transcript content into the system prompt.

Return a structured `summary_agent_invalid_prompt` error when `custom` is selected with blank or over-limit text. Unknown preset values should fail DTO deserialization rather than silently changing the user's choice.

## UI Behavior

Add a compact `Summary style` control near the existing Transcript and Language options:

- Use the existing `SelectField` pattern to avoid overcrowding the Summary Agent panel.
- Show the preset label and one short explanatory line for the active choice.
- When `custom` is selected, reveal a labeled multiline `Custom system prompt` editor with a character count and the same limit enforced by Rust.
- Preserve custom text when the user temporarily switches to a built-in preset, so switching back to `custom` does not erase work.
- Disable `Run summary` and show an inline validation message when custom mode is blank or over the limit.
- Built-in choices do not need a large editable preview; their descriptions make their behavior clear while keeping the operational UI compact.
- Keep the current transcript scope, language, section toggles, provider selection, and run/status behavior unchanged.

The editor must be usable at the current narrow desktop breakpoint. Add only Summary Agent-specific CSS for the description, editor, validation state, and character counter.

## Implementation Steps

1. Extend the shared config contract.
   - Add `MeetingSummaryPromptPreset`, `promptPreset`, and `customSystemPrompt` in `src/types.ts` and `src-tauri/src/models.rs`.
   - Update frontend defaults and Rust test fixtures together.
   - Keep Tauri camelCase/snake_case serialization aligned.

2. Add frontend preset metadata and validation helpers.
   - Keep stable preset metadata in a small exported constant/helper near the existing summary config helpers, or in `src/summary-prompts.ts` if separation improves readability.
   - Add pure helpers for custom prompt trimming/validation and active preset description.
   - Unit test the default, every preset ID, blank custom input, boundary length, over-limit input, and preserving custom text while switching modes.

3. Compose the runtime prompt safely.
   - Separate invariant rules from style instructions in `src-tauri/src/summary_agent.rs`.
   - Resolve built-in/custom instructions from `MeetingSummaryConfig` once per run, before provider calls.
   - Reuse the resolved system message for all chunks.
   - Add Rust tests that verify each preset resolves, custom text is present, invariant rules remain present, and invalid custom prompts fail before an LLM request.

4. Add the Summary Agent controls.
   - Render the preset selector and active description in `src/App.tsx`.
   - Conditionally render the custom textarea, counter, and inline error.
   - Pass the full config through the existing `runMeetingSummaryAgent()` wrapper; no new Tauri command is required.
   - Add focused responsive styles in `src/styles.css`.

## Verification

- `npm test`
- `npm run build`
- `cargo fmt --check` from `src-tauri`
- `cargo check` from `src-tauri`
- `cargo test` from `src-tauri`
- TypeScript tests:
  - default config selects `balanced`.
  - every preset has a stable ID, label, and description.
  - custom prompt validation handles whitespace, exact limit, and over-limit input.
  - changing presets does not overwrite the stored custom text.
- Rust tests:
  - every built-in preset produces its expected style directive.
  - custom instructions are trimmed and included in the composed system message.
  - invariant JSON and grounding rules are present for built-in and custom modes.
  - blank and over-limit custom prompts return `summary_agent_invalid_prompt` before provider access.
- Manual checks:
  - run the same short transcript with Professional, Gentle, Detailed, and Timeline and confirm tone/organization changes while the result still renders in all structured sections.
  - select Custom, enter a Vietnamese instruction, run the summary, switch away and back, and confirm the text is retained during the app run.
  - confirm a timeline summary does not add dates/times absent from the transcript.
  - confirm the custom editor and validation remain usable at narrow and normal window widths.

## Acceptance Criteria

- Summary Agent settings offer Balanced, Professional, Gentle, Detailed, Timeline, and Custom choices.
- Balanced is the default and remains behaviorally compatible with the current summary output.
- Selecting a built-in preset changes the next summary run's tone or organization without changing the provider profile.
- Custom mode accepts user-authored instructions and prevents blank or over-limit runs.
- Custom text survives switching between preset choices during the current app run.
- All modes preserve the existing structured result schema, enabled-section filtering, source IDs, output language, and transcript-grounding rules.
- No prompt or transcript content is added to logs, events, provider-profile secrets, or exports.

## Risks

- Conflicting custom instructions could ask for non-JSON output or fabricated content. Mitigation: backend-owned invariant rules are always first, schema parsing remains authoritative, and invalid output follows the existing error path.
- Detailed prompts can increase response size and latency. Mitigation: keep existing provider token limits and make the preset describe detail rather than override provider limits.
- Timeline wording may encourage fabricated timestamps. Mitigation: require transcript-sourced times only and cover absent-time behavior manually.
- Duplicating preset text in Rust and TypeScript can drift. Mitigation: keep executable instructions in Rust and frontend metadata limited to labels/descriptions; test all stable IDs on both sides.
- `App.tsx` is already large. Mitigation: keep prompt metadata and validation in pure helpers and limit JSX changes to the existing Summary Agent panel.

## Non-Goals

- User-created libraries of multiple named prompts.
- Syncing prompts across devices or profiles.
- Persisting the complete Summary Agent settings object across app restarts.
- Letting custom prompts replace the structured meeting-note schema.
- Changing summary provider APIs, retry policy, transcript chunking, or rolling-memory behavior.
- Adding new meeting-note result sections beyond the current schema.
