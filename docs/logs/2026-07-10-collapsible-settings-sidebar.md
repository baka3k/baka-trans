# Collapsible Settings Sidebar — 2026-07-10

## Context

Phase 16 makes the conversation feed the primary workspace while preserving meeting controls in the left column (`plans/realtime-meeting-translation-macos/phase-16-conversation-translation-ui-redesign.md:43` and `plans/realtime-meeting-translation-macos/phase-16-conversation-translation-ui-redesign.md:73`). The fixed settings column still consumed translation-reading space when those controls were not needed.

## Change

Added open-by-default `settingsOpen` state and an accessible top-bar toggle with matching panel semantics in `src/App.tsx:239` and `src/App.tsx:994`. The settings panel now hides when collapsed and the workspace switches to a single translation column in `src/App.tsx:1055`, `src/styles.css:177`, and `src/styles.css:190`.

## Impact

Users can reclaim the full workspace for reading live translations and reopen settings without losing the default expanded experience. Risk level: low, because the change is frontend-only, does not alter session settings or translation behavior, and keeps the existing layout when open.

## Decision

Used a single header toggle and conditional grid class instead of introducing a drawer or persistent preference. This keeps the interaction visible, preserves the phase-16 desktop layout, and avoids adding storage or lifecycle complexity for a reversible view choice.

## References

- plan: [Phase 16 — Conversation Translation UI Redesign](../../plans/realtime-meeting-translation-macos/phase-16-conversation-translation-ui-redesign.md)
- source: `src/App.tsx:239`
- source: `src/App.tsx:994`
- source: `src/App.tsx:1055`
- source: `src/styles.css:177`
- source: `src/styles.css:190`
- commit: `4657601a10df72d7f25cc56c1f539d25fc7ddd76`
