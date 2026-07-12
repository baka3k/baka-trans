# Phase 16: Conversation Translation UI Redesign

Status: implemented baseline; broader visual integration planned in `../260712-2234-application-ui-modernization/plan.md`
Depends on: phases 04, 08
Compatible with: phases 12, 13, 15
Primary files: `src/App.tsx`, `src/styles.css`, `src/types.ts`, `src/transcript.ts`, `src/transcript.test.ts`

## Objective

Redesign the live transcript area into a calm conversation-style translation surface. Each spoken line should read like a chat utterance: speaker context first, source speech as the main line, and the translation immediately underneath. The user should always know whether source audio is arriving, speech is being detected, translation is pending, translated audio is playing, or the stream needs attention.

Reading this as: desktop product UI redesign for meeting users under realtime pressure, with a calm translation-console language, leaning toward custom CSS variables with Cupertino-style softness and Material-style state clarity.

Design dials:

- `DESIGN_VARIANCE: 4` because this is a work utility, not a marketing surface.
- `MOTION_INTENSITY: 3` because status feedback should feel alive but not distract during meetings.
- `VISUAL_DENSITY: 6` because users need controls, audio state, transcript, and summary access in one desktop window.

## Current State Audit

- `src/App.tsx` already has a two-column shell: settings on the left and live translation on the right.
- The transcript currently renders as a source/translated table with `Original speech` and `Translated speech` headers.
- `TranscriptItem` contains `id`, `timestampMs`, `sourceText`, `translatedText`, `status`, and optional `latencyMs`, but no speaker fields yet.
- `audio-level` events already update a raw `level` meter, while `translated-audio-level` updates translated playback level.
- Phase 08 already defines the right source-signal states: `waiting`, `receiving`, `silent`, `stale`, and `error`.
- Phase 15 will add optional diarization labels later. This UI must accept future speaker metadata without pretending speaker identity exists today.
- The current theme uses CSS variables, `prefers-color-scheme`, 8px radii, muted green accent, semantic info/warn/danger colors, and the existing `lucide-react` icon family.

## Design Contract

1. Keep one design system.
   - Continue with the existing React, Vite, CSS variable, and lucide stack.
   - Do not add Material or Cupertino packages for this pass.
   - Use Cupertino-style spacing, softened surfaces, and segmented status controls only as visual inspiration.

2. Keep one theme model.
   - Preserve automatic light/dark mode through CSS variables.
   - Keep the existing green accent for live/healthy states.
   - Use blue only for in-progress information, amber for warnings, and red for errors.
   - Avoid new gradients, glows, or decorative surfaces that do not communicate state.

3. Make the conversation stream the primary workspace.
   - Replace the transcript table with a vertical `conversation-feed`.
   - Render each transcript item as one stable `utterance-card`.
   - Card order is chronological, with newest at the bottom.
   - Preserve auto-scroll when the user is near the bottom.
   - If the user scrolls up, show a compact "New translation" affordance instead of forcing the scroll position.

4. Render each utterance line by line.
   - Header: speaker chip, timestamp, and status.
   - Source line: original text, larger and higher contrast.
   - Translation line: translated text directly below the source line, with a subtle accent edge or tinted surface.
   - Partial source text stays readable but visually lighter.
   - If source exists but translation is empty, show a stable `Translating` placeholder row.
   - If an item has `status: "error"`, show an inline error state inside that card.

5. Treat speaker attribution honestly.
   - Add optional frontend support for `speakerLabel`, `speakerSegmentId`, and `speakerConfidence`.
   - Until diarization exists, display a neutral label such as `Speaker` or `Source`.
   - Do not infer names, avatars, colors, or identities from text.
   - When phase 15 lands, map real labels into the same card header without changing the feed structure.

6. Keep live state visible above the transcript.
   - Add a compact status rail at the top of the translation column.
   - Required state chips:
     - Source signal: `Waiting`, `Receiving audio`, `Source silent`, `No recent audio`, `Capture error`.
     - Session state: idle, starting, listening, translating, speaking, paused, stopping, error.
     - Translation progress: `Listening`, `Translating`, `Ready`, or `Needs attention`.
     - Playback signal: translated output level when available.
   - Status chips must use icons, labels, accessible names, and stable widths where possible.

7. Preserve meeting controls.
   - Keep Start, Pause, Resume, Stop, and Translate now behavior unchanged.
   - Keep provider, routing, local monitor, test tone, key, and summary controls in the left column.
   - Move only the layout and presentation needed for the chat-style translation experience.

## Data And State Plan

Add small frontend helpers rather than pushing presentation logic into JSX:

- `deriveSourceSignalState(event, selectedInputDeviceId, sessionStatus, nowMs)`:
  - Reuse Phase 08 thresholds.
  - Ignore events from stale input devices.
  - Distinguish healthy silence from stale/no events.

- `deriveConversationItems(transcript)`:
  - Returns display items with stable ids, source lines, translation lines, status, timestamp, optional latency, and optional speaker fields.
  - Does not merge unrelated finalized items.
  - Allows future grouping by speaker only after real speaker labels exist.

- `deriveTranslationActivity(status, latestItem, sourceSignalState, translatedLevel)`:
  - Produces the rail labels and icon states used by the translation column.
  - Treats empty translation on a partial item as `translating`.
  - Treats stale source signal during an active session as `needs attention`.

Recommended TypeScript additions:

```ts
export interface TranscriptItem {
  id: string;
  timestampMs: number;
  sourceText: string;
  translatedText: string;
  status: TranscriptStatus;
  latencyMs?: number;
  speakerLabel?: string;
  speakerSegmentId?: string;
  speakerConfidence?: number;
}

export type SourceSignalState = "waiting" | "receiving" | "silent" | "stale" | "error";
```

## Implementation Steps

1. Add source signal classification.
   - Move raw `audio-level` handling from a meter-only state into a reducer/helper.
   - Track `lastAudioLevelAtMs` and selected input device.
   - Reset to `waiting` when the session stops, the app hydrates, or input device changes.
   - Mark active sessions as `stale` when no matching audio event arrives within 1500-2500 ms.

2. Build conversation display helpers.
   - Add display item derivation in `src/transcript.ts`.
   - Preserve existing `mergeTranscriptDelta` behavior.
   - Add unit tests for partial source, pending translation, final translation, error item, and optional speaker label.

3. Replace the transcript table.
   - Remove the two-column `transcript-head` and `transcript-row` layout.
   - Add `ConversationFeed`, `UtteranceCard`, `TranslationLine`, and `LiveStatusRail` components inside `src/App.tsx` or a new small local component file if `App.tsx` becomes too large.
   - Keep styles in `src/styles.css` unless the project introduces component CSS conventions later.

4. Add status rail and empty states.
   - Render the live state rail above the feed.
   - Empty state should show current readiness: setup needed, waiting for source audio, or ready to listen.
   - Loading and translating states should reserve the same card height as final content to avoid layout jump.
   - Errors should be contextual in the rail and existing error bar.

5. Polish responsive desktop layout.
   - Keep the settings column usable at the current desktop minimum.
   - Make the feed readable at narrow widths by stacking source and translation within the same card, not by returning to columns.
   - Keep buttons single-line and icon-led.
   - Use stable dimensions for chips, meters, and cards.

6. Verify accessibility and motion.
   - Use `aria-live="polite"` for newly finalized transcript items, not every partial token.
   - Keep progress meters as real `role="progressbar"` elements with values.
   - Respect `prefers-reduced-motion`.
   - Animate only opacity/transform for new cards or status changes.

## Visual Direction

- Main feed background: calm app surface with subtle contrast from cards.
- Cards: 8px radius to match the existing app, light border, no heavy shadows.
- Source text: primary text, 15-17px, comfortable line height.
- Translation text: slightly tinted surface, accent left edge or top border, same readability as source.
- Partial status: lower contrast and optional subtle skeleton shimmer disabled under reduced motion.
- Final status: no extra decoration beyond a small status label or check icon.
- Signal rail: compact, horizontal, icon plus short label, no decorative dots except real semantic live state.

## Verification

- `npm test`
- `npm run build`
- Unit tests:
  - source signal becomes `receiving` above threshold.
  - source signal becomes `silent` below threshold with fresh events.
  - source signal becomes `stale` when active session events stop.
  - unmatched input-device events are ignored.
  - conversation display items show source and translation in the same card.
  - pending translation renders a stable placeholder state.
  - optional speaker labels render when present and fall back neutrally when absent.
- Manual checks:
  - Start a session and confirm the rail transitions from waiting to receiving/silent.
  - Send partial transcript updates and confirm cards do not jump.
  - Confirm translation appears directly under its source line.
  - Confirm final transcript history remains readable after many utterances.
  - Confirm dark and light modes both pass visual contrast.
  - Confirm narrow window layout stays usable.

## Acceptance Criteria

- The transcript no longer looks like a table. It reads as a meeting conversation stream.
- Each utterance shows source speech and its translation directly underneath.
- Users can tell at a glance whether audio is arriving, translation is in progress, translated output is playing, or attention is needed.
- The UI handles partial, final, empty, error, and scrolled-away states without layout shift.
- Future speaker labels from diarization can be displayed without another transcript redesign.
- No backend translation behavior changes are required for the UI pass.

## Risks

- Chat styling can imply real speaker identity before diarization exists. Mitigation: use neutral labels until phase 15 supplies real labels.
- Too much motion can distract during meetings. Mitigation: keep motion intensity low and disable animation under reduced motion.
- A status rail can become noisy if every backend detail is exposed. Mitigation: show only source, session, translation, and playback states.
- `App.tsx` is already large. Mitigation: extract local presentational components only if it improves readability without introducing a new architecture.

## Non-Goals

- Implementing speaker diarization.
- Changing translation providers or backend WebSocket behavior.
- Rewriting summary-agent settings.
- Adding a full design-system dependency.
- Recording or storing raw meeting audio.
