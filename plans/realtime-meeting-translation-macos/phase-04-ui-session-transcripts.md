# Phase 04 - UI, Session Controls, and Transcripts

Status: planned
Depends on: phase 03

## Goal

Build the user-facing workflow for running a translation session during a meeting.

## Implementation Tasks

- Replace placeholder controls with real backend data.
- Implement language controls:
  - source: Auto, English, Japanese, Vietnamese
  - target: English, Japanese, Vietnamese
  - prevent same-language confusion with a non-blocking warning
- Implement translation style:
  - Literal
  - Natural
  - Technical/meeting-safe
- Implement session controls:
  - Start
  - Stop
  - Pause
  - Resume
  - disabled/loading states
- Implement live status display:
  - idle
  - starting
  - listening
  - translating
  - speaking
  - paused
  - stopping
  - error
- Implement transcript panel:
  - source text column
  - translated text column
  - partial/final styling
  - latest item auto-scroll
  - compact history for current session
- Implement export:
  - plain text
  - Markdown
  - no raw audio export
- Implement API key settings:
  - save/update key
  - show stored/not stored state
  - do not render full key after save
- Add a setup checklist for Teams + BlackHole routing.
- Add accessible keyboard navigation and focus states.

## Verification

- Start/Pause/Resume/Stop transitions are visually clear and match backend state.
- Transcript updates do not resize or shift the main controls.
- Exported transcript includes source and translated text in chronological order.
- API key cannot be read back in full from the UI.
- UI remains responsive while audio pipeline runs.

## Exit Criteria

- A user can complete the main flow without touching developer tools.
- Errors and setup issues are visible enough to self-correct.
