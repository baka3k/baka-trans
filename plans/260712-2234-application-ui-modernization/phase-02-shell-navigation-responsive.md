# Phase 2: Application Shell, Navigation, and Responsive Settings

Status: planned
Depends on: phase 01
Primary files: `src/App.tsx`, `src/app/MainApp.tsx`, `src/components/shell/*`, `src/components/session/SessionCommandBar.tsx`, `src/styles/app.css`

## Objective

Replace the settings-first responsive stack with a live-first Fluent shell while preserving the existing main-window state and handlers.

## Work

1. Extract the main window container.
   - Move existing main-window state, effects, memos, refs, and handlers into `src/app/MainApp.tsx` without logical rewrites.
   - Keep `src/App.tsx` responsible only for choosing the main or overlay route.

2. Build the app bar.
   - Preserve brand, session state, key warning, settings toggle, overlay launchers, device refresh, and export actions.
   - Use Fluent tooltips and accessible icon buttons.
   - Group overlay actions and export actions into menus only when width requires it.
   - Retain current disabled logic and accessible names.

3. Build the persistent session command bar.
   - Keep source and target language, Start, Pause, Resume, Stop, Translate now, and fallback-chain behavior.
   - Use a stable Pause or Resume slot instead of showing both disabled actions when possible, provided the underlying handler logic stays unchanged.
   - Preserve a clearly distinct destructive Stop treatment.

4. Add setup navigation.
   - Create Live, Audio, Translation, and Summary destinations.
   - Navigation changes only which settings surface is visible; all controlled form state stays mounted in `MainApp`.
   - Mark the active destination semantically and support keyboard navigation.

5. Add responsive context panels.
   - At wide desktop sizes, show the selected context panel beside the live workspace.
   - At compact desktop sizes, show it as a right-side drawer over the live workspace.
   - At narrow sizes, use a full-width drawer that traps focus, closes on Escape, and restores focus to the trigger.
   - Never place the full settings form above the live workspace.

6. Simplify surface hierarchy.
   - Replace nested bordered panels with Fluent cards only where containment communicates hierarchy.
   - Use headings, spacing, and dividers for ordinary grouping.
   - Remove the top-bar gradient and non-semantic shadowing.

## Verification

- Test active navigation and focus movement.
- Test drawer open, Escape close, focus trap, and focus return.
- Test that settings navigation does not invoke backend commands.
- Test that entered profile, key, language, and routing values persist when changing destinations.
- Visual checks at 1,440px, 1,280px, 1,024px, 960px, and 720px.
- Assert no horizontal document overflow at every target width.

## Acceptance Criteria

- Live translation remains immediately visible at every supported main-window width.
- Primary session actions remain visible across all settings destinations.
- The 1,040px breakpoint no longer creates a multi-thousand-pixel settings stack before translation.
- App-bar actions do not wrap into an oversized header.
- All current main-window actions retain the same command behavior and disabled rules.
