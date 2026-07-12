# Phase 3: Workflows, Forms, and Live Translation Surface

Status: planned
Depends on: phase 02
Primary files: `src/app/MainApp.tsx`, `src/components/session/*`, `src/components/audio/*`, `src/components/translation/*`, `src/components/summary/*`, `src/styles/app.css`

## Objective

Migrate every functional main-window workflow into accessible Fluent components and polish the live translation experience without changing its data or behavior.

## Work

1. Session and readiness.
   - Present readiness problems next to the command they block.
   - Keep existing source and target options, button enablement, boundary feedback, and fallback behavior.
   - Use clear progress and error states without adding new session states.

2. Audio routing.
   - Preserve auto refresh, manual refresh, device selectors, output channel, source signal, line monitor, test tone, local monitor, original monitoring, routing summary, warnings, and Windows-specific visibility.
   - Group routine routing choices before diagnostics and test tools.
   - Keep meters as labeled progress bars with visible numeric or textual state.

3. Translation provider.
   - Preserve provider switching, credential field, save, status, test, and setup guidance.
   - Use password-field affordances only if they do not expose or persist secrets differently.
   - Keep validation and key-source messaging adjacent to the field.

4. Summary Agent.
   - Preserve profile selection, provider kind, profile fields, key status, CRUD, test, transcript scope, language, prompt preset, custom prompt validation, section toggles, and Run summary behavior.
   - Break the long form into named groups within the Summary destination.
   - Use accordions only for advanced tuning, not for required fields.

5. Live status and conversation feed.
   - Reuse `deriveSourceSignalState`, `deriveTranslationActivity`, and `deriveConversationItems` unchanged.
   - Refine the four-category status rail so icons, labels, meter values, and tones remain readable in light, dark, and forced-colors modes.
   - Preserve auto-scroll, scrolled-away behavior, New translation action, neutral speaker labels, partial/final states, and sentence pairing.
   - Keep final items polite in the live region and partial updates silent.

6. Notes and errors.
   - Present meeting notes with a clear reading hierarchy and compact action-item metadata.
   - Keep errors contextual when possible and retain the global error surface for session-level failures.
   - Add stable loading, empty, disabled, error, and success layouts without new data states.

7. Complete the icon migration.
   - Replace all remaining Lucide imports with equivalent Fluent icons.
   - Remove `lucide-react` only after `rg` confirms no imports remain.

## Verification

- Component tests for all controls in the Phase 1 contract inventory.
- Exact-once handler assertions for session, routing, provider, profile, summary, export, and refresh actions.
- Tests for empty, pending, partial, final, error, and new-translation states.
- `npm test`
- `npm run build`
- Manual keyboard, light/dark, and 200% zoom checks.

## Acceptance Criteria

- Every existing main-window workflow remains present and functional.
- Advanced configuration is easier to scan and no longer competes with the live transcript.
- Form labels, descriptions, validation, and statuses have consistent placement.
- State is never communicated by color alone.
- The conversation workspace remains stable during partial-to-final updates.
- No Lucide icon imports remain after the Fluent icon migration is complete.
