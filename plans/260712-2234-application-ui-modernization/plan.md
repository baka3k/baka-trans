# Application UI Modernization

Status: planned
Created: 2026-07-12 22:34 Asia/Bangkok
Mode: `hi-plan --fast`
Blocked by: none
Coordinates with: `plans/realtime-meeting-translation-macos/phase-16-conversation-translation-ui-redesign.md`
Coordinates with: `plans/260716-2033-local-llm-audio-translation`
Updates: `plans/realtime-meeting-translation-macos/plan.md`

## Objective

Modernize the Baka Trans desktop interface into a clean, professional, accessible Fluent 2 product experience while preserving every existing workflow and business-logic contract. The work covers the main session window, settings and summary configuration, live translation workspace, transparent OCR translation overlay, and Look & Help overlay.

## Design Read

Reading this as: a desktop real-time translation utility for meeting users under pressure, with a calm and professional Fluent 2 language, optimized for fast state recognition and low-distraction operation.

Design dials:

- `DESIGN_VARIANCE: 3`: predictable placement and strong alignment matter more than expressive composition in a live meeting tool.
- `MOTION_INTENSITY: 3`: transitions communicate focus, disclosure, and state change without competing with transcript content.
- `VISUAL_DENSITY: 6`: the app has many controls, but primary session actions and live output need more space than advanced configuration.

Design system decision:

- Adopt official Fluent UI React v9 through `@fluentui/react-components`.
- Adopt the matching `@fluentui/react-icons` family and remove `lucide-react` only after all icons have migrated.
- Use one custom Fluent theme for the main app and both overlay routes.
- Preserve automatic light and dark mode. Add forced-colors and reduced-motion behavior.
- Keep the existing green brand accent, with blue for informational progress, amber for warnings, and red for errors.

## Current-State Audit

### Architecture

- `src/App.tsx` is a 3,077-line component containing main-window state, backend event subscriptions, command handlers, main-window markup, both overlay routes, and shared presentational helpers.
- `src/styles.css` is a 2,106-line global stylesheet covering the main shell, forms, transcript, overlays, themes, and responsive behavior.
- The frontend is React 18, TypeScript, Vite, and Tauri v2 with no component framework.
- The existing icon family is `lucide-react`.
- Business logic is already separated at useful boundaries in `src/api.ts`, `src/transcript.ts`, `src/languages.ts`, and `src/types.ts`.
- Existing automated tests cover transcript helpers and language rules, but not component accessibility, navigation, or responsive layout.

### Visual and UX Findings

- The 1,440px layout has a workable two-column structure, but the settings column is visually dense and requires deep scrolling.
- At the existing 1,040px breakpoint, settings and translation stack vertically. The settings column grows to roughly 2,457px, pushing live translation below several screens of configuration.
- At 720px, the settings column grows to roughly 3,316px, the top bar grows to roughly 244px, and live translation begins around 3,604px down the page.
- Collapsing settings at 1,024px produces a strong translation-focused workspace, confirming that a live-first layout should be the responsive default.
- Main controls are available, but primary and advanced actions share similar visual weight.
- The top bar contains brand, health state, overlay launchers, refresh, and export actions in one row. At narrow widths this becomes a large action grid.
- The current theme is consistent but visually heavy: many bordered panels, low separation between hierarchy levels, a decorative top-bar gradient, and small secondary text.
- The live status rail and conversation feed are already implemented and form a good functional baseline for this redesign.
- Both overlay routes are coherent, but their labels and secondary text are too small for quick reading and their panels do not yet share a polished component language with the main app.

### Patterns to Preserve

- Existing green brand identity and automatic light/dark behavior.
- Existing session, routing, provider, summary, transcript, export, overlay, and capture workflows.
- Existing labels where they are part of established user workflows or accessible names.
- Existing status semantics: healthy/live, information/in-progress, warning, and error.
- Existing conversation feed behavior, new-translation affordance, partial/final transcript distinction, and neutral speaker labeling.
- Existing Tauri window dragging, overlay geometry, capture, pause, close, and settings behavior.

### Patterns to Retire

- Vertical responsive stacking that places the live workspace after the full settings form.
- A monolithic markup and CSS structure that makes visual changes risky.
- Equal visual weight for routine, advanced, destructive, and primary actions.
- Excess bordered-panel nesting where spacing and headings can communicate grouping.
- Decorative gradients and shadows that do not communicate elevation or focus.
- Tiny overlay metadata and low-contrast helper text.

## Functional Preservation Contract

The redesign must not change:

- Tauri command names, arguments, or invocation timing in `src/api.ts`.
- Event names, payloads, or subscription behavior.
- Session-state transitions or command enablement rules.
- Translation-provider, language, audio-routing, summary-profile, prompt, transcript, or overlay data contracts.
- Key storage, profile persistence, routing persistence, or security behavior.
- Form field names, values, validation rules, or saved payload shapes.
- Transcript merge, display derivation, export, or summary-generation logic.
- Overlay capture cadence, OCR processing, LLM execution, geometry, or window behavior.

UI-only state may be added for active navigation, responsive drawers, disclosures, focus return, and theme presentation.

## Target UX Architecture

### Main Window

| Region | Purpose | Contents |
| --- | --- | --- |
| App bar | Global identity and current health | Brand, session status, credentials warning, settings toggle, overlay launch menu, refresh, export menu |
| Primary command bar | Actions required during a meeting | Source and target language, Start, Pause or Resume, Stop, Translate now |
| Navigation rail | Move between setup areas without scrolling through all forms | Live, Audio, Translation, Summary |
| Live workspace | The default and dominant view | Live status rail, contextual errors, meeting notes when present, conversation feed, empty and pending states |
| Context panel | Edit the selected setup area | Session basics, audio routing, translation provider, or Summary Agent controls |

Navigation rules:

- The Live workspace remains mounted so transcript and scroll state are not lost when settings sections change.
- Session controls remain visible in the primary command bar in every main-window view.
- Audio, Translation, and Summary navigation changes presentation only. Existing values and handlers remain the same.
- Overlay launchers move into one clearly named app-bar menu at compact widths while remaining direct actions at wide widths.
- Export actions share one menu at compact widths and remain disabled according to current transcript rules.

### Responsive Behavior

| Width | Layout |
| --- | --- |
| `>= 1280px` | App bar, compact navigation rail, live workspace, and a 360-420px context panel visible together |
| `1041-1279px` | Navigation rail plus live workspace; settings open as a non-destructive side panel over the right edge |
| `721-1040px` | Live workspace stays first; navigation condenses; settings use a modal drawer with focus trap and focus return |
| `<= 720px` | Compact app bar, horizontally scrollable or menu-based navigation, single-column live workspace, full-width settings drawer, no action-label wrapping |

The live workspace must never be placed after the full settings form. No horizontal document overflow is allowed at 720px, 960px, 1,024px, 1,280px, or 1,440px.

### Overlay Windows

- Use the same semantic tokens, type scale, control heights, icons, focus rings, and state colors as the main app.
- Preserve transparent window material and Tauri drag regions.
- Keep the capture or pause action visually dominant.
- Use readable 13-16px type for labels, body text, and results, adjusted to overlay dimensions.
- Maintain layout-specific behavior: two stacked panes for Look Through and captured/request/result composition for Look & Help.
- Make settings a clear disclosure panel without resizing the critical result area unexpectedly.

## Visual System

### Theme and Color

- Create custom Fluent light and dark themes from one green brand ramp.
- Use semantic tokens rather than direct color values in components.
- Maintain WCAG AA contrast of at least 4.5:1 for normal text and 3:1 for large text, icons, focus indicators, and control boundaries.
- Use one theme per window. Do not invert individual sections.
- Use solid surfaces and restrained neutral elevation. Remove the current decorative top-bar gradient.
- Add `forced-colors` support and do not rely on color alone for status.

### Typography

- Use the Fluent system stack led by `Segoe UI Variable` and `Segoe UI`, with macOS and system fallbacks.
- Establish a small type scale: app title, section title, body, label, caption, and monospace only where numeric diagnostics benefit.
- Minimum 13px for persistent UI text and 14px for form/body copy at normal desktop scale.
- Use weight and spacing for hierarchy rather than serif switches, all-caps labels, or oversized headings.

### Shape, Spacing, and Elevation

- Use Fluent's medium radius consistently: 8px controls and 12px major surfaces, with pills only for compact statuses.
- Base layout spacing on a 4px scale, with common gaps of 8, 12, 16, 24, and 32px.
- Use borders for structure and shadows only for actual elevation such as menus, drawers, dialogs, and floating controls.
- Keep primary controls at least 40px high in the main app and at least 36px in compact overlays.

### Icons and Motion

- Use only Fluent icons after migration and pair icon-only actions with accessible names and tooltips.
- Standardize icon sizes at 16, 20, and 24px.
- Animate only opacity and transform for drawers, menus, transcript insertion, and status transitions.
- Disable non-essential motion under `prefers-reduced-motion`.
- Do not add decorative loops, scroll effects, glows, or animated backgrounds.

## Component Architecture

Keep state and commands in a container layer while extracting presentational surfaces:

```text
src/
  App.tsx                         route selection only
  app/
    MainApp.tsx                   existing state, effects, and handlers
    TransparentOverlayWindow.tsx  existing overlay state and handlers
    LookHelpOverlayWindow.tsx     existing overlay state and handlers
  components/
    shell/
      AppShell.tsx
      AppBar.tsx
      AppNavigation.tsx
      ResponsiveSettingsPanel.tsx
    session/
      SessionCommandBar.tsx
      SessionSettings.tsx
      LiveStatusRail.tsx
    audio/
      AudioRoutingPanel.tsx
      AudioLineMonitor.tsx
    translation/
      TranslationProviderPanel.tsx
      ConversationFeed.tsx
      UtteranceCard.tsx
    summary/
      SummaryAgentPanel.tsx
      MeetingNotes.tsx
    overlays/
      OverlayTitleBar.tsx
      OverlayStatusBar.tsx
      OverlayPanel.tsx
  ui/
    theme.ts
    ThemeProvider.tsx
    layout.ts
  styles/
    app.css
    overlays.css
```

Extraction rule: move existing hooks, handlers, and derived values without rewriting their logic. Components receive typed props and callback references. Logic helpers remain in `src/transcript.ts`, `src/languages.ts`, and `src/api.ts`.

## Phases

1. Fluent foundation and UI contracts
   - Add the design-system dependencies, custom theme, semantic tokens, global typography, and test harness.
   - Inventory every current control, state, handler, and accessible name before moving markup.
   - See `phase-01-fluent-foundation.md`.

2. Application shell, navigation, and responsive settings
   - Build the live-first shell, primary command bar, navigation rail, and responsive context panel or drawer.
   - Remove the settings-first vertical stack below 1,040px.
   - See `phase-02-shell-navigation-responsive.md`.

3. Workflows, forms, and live translation surface
   - Migrate session, audio, provider, summary, notes, and transcript components without changing their state or command behavior.
   - Complete empty, loading, partial, final, warning, and error presentation.
   - See `phase-03-workflows-live-surface.md`.

4. Overlay modernization
   - Apply the same theme and component language to Look Through and Look & Help while preserving transparent-window behavior.
   - See `phase-04-overlay-modernization.md`.

5. Accessibility, responsive, and regression hardening
   - Add component accessibility tests, keyboard checks, responsive visual verification, and platform smoke tests.
   - See `phase-05-accessibility-regression.md`.

## File Impact Map

| File or area | Planned change |
| --- | --- |
| `package.json`, `package-lock.json` | Add Fluent UI, Fluent icons, and focused UI-test dependencies; remove Lucide after migration |
| `src/main.tsx` | Mount the shared theme provider without changing route selection behavior |
| `src/App.tsx` | Reduce to route selection and import the extracted app windows |
| `src/app/*` | Hold existing state, effects, handlers, and command wiring by window |
| `src/components/*` | Add typed presentational sections and responsive navigation |
| `src/ui/*` | Define Fluent theme, brand ramp, theme selection, and layout constants |
| `src/styles.css` | Retire after styles are split; preserve only a temporary compatibility layer during migration |
| `src/styles/app.css` | Main-window layout and limited non-Fluent composition styles |
| `src/styles/overlays.css` | Transparent overlay layout and Tauri drag-region styles |
| `src/transcript.ts` | No logic rewrite; only type-safe display props if required |
| `src/types.ts` | No backend contract changes; UI-only types may be added separately |
| `vite.config.ts` | Add jsdom or equivalent test environment only if component tests require it |
| `src/**/*.test.tsx` | Add UI contract, keyboard, state, and accessibility coverage |

## Verification Matrix

### Automated

- `npm test`
- `npm run build`
- Component tests for main navigation, settings drawer, command enablement, provider selection, summary configuration, and overlay disclosure behavior.
- Accessibility assertions with Testing Library and axe for the main shell, each settings section, conversation states, and both overlay routes.
- Verify no raw business command is invoked by navigation-only interactions.
- Verify the current handler is called exactly once for Start, Pause, Resume, Stop, Translate now, refresh, export, capture, pause overlay, and close overlay actions.

### Manual Responsive Checks

- Main window at 1,440x1,000, 1,280x800, 1,024x768, 960x640, and 720x900.
- Look Through at its configured minimum, typical, and wide dimensions.
- Look & Help at its configured minimum, typical, and wide dimensions.
- Confirm live translation remains visible without first scrolling through settings at every main-window width.
- Confirm drawers and menus remain within the viewport and return focus to their trigger.
- Confirm labels and primary buttons do not wrap at desktop sizes.

### Accessibility Checks

- Keyboard-only traversal follows visual order and reaches all controls.
- Escape closes menus, drawers, and dialogs without stopping a session.
- Focus is visible in light, dark, and forced-colors modes.
- Screen-reader names remain stable for icon-only actions.
- Status changes include text or icons in addition to color.
- Final transcript updates use polite announcements; partial tokens do not flood the live region.
- Content remains usable at 200% zoom and with text spacing overrides.
- Reduced-motion mode removes drawer and transcript transitions.

### Platform Smoke Checks

- `npm run tauri -- dev` on Windows and macOS.
- Start, Pause, Resume, Stop, Translate now, device refresh, test tone, local monitor, profile CRUD, summary run, exports, overlay launch, overlay capture, overlay pause, and overlay close.
- Light and dark theme follow the OS without losing entered form values or live transcript state.

## Acceptance Criteria

- All current user-facing workflows remain available and call the same backend functions with the same data.
- The main window uses one coherent Fluent 2 visual system across navigation, controls, forms, transcript, notes, states, and menus.
- At 1,024px and 720px, live translation is visible in the first viewport or directly beneath the compact command bar, not after the full settings form.
- Primary meeting actions are visually dominant and remain accessible from every settings section.
- Advanced settings are grouped into clear Audio, Translation, and Summary destinations without losing unsaved values when navigation changes.
- Empty, loading, partial, final, warning, error, disabled, and success states are visually consistent and do not shift layout unexpectedly.
- Both overlay windows share the design system while preserving transparency, dragging, capture, pause, resizing, and close behavior.
- Light, dark, forced-colors, reduced-motion, keyboard, and 200% zoom checks pass.
- Normal text meets WCAG AA contrast and interactive controls have visible focus indicators.
- No new horizontal overflow occurs at the defined widths.
- `npm test` and `npm run build` pass.

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| UI extraction changes hook timing or command wiring | Keep state and effects in window containers first; move only JSX and CSS; add callback contract tests before visual migration |
| Fluent migration increases bundle size | Import components and icons by package exports, measure production bundle, and avoid duplicate component systems |
| Navigation hides critical controls | Keep session commands in a persistent command bar and show readiness issues in both the app bar and relevant settings destination |
| Settings drawer loses focus or values | Keep form state in `MainApp`, use controlled props, trap focus while open, and restore focus on close |
| Theme changes regress overlay transparency | Keep overlay material values in dedicated semantic tokens and test both OS themes at real Tauri opacity |
| Responsive redesign creates a mobile-looking desktop app | Use desktop navigation and density above 1,040px; reserve drawers and menu condensation for constrained windows |
| Status simplification hides diagnostics | Keep the four live status categories and expose detailed audio diagnostics in the Audio destination |
| Existing phase-16 plan duplicates transcript work | Treat the current conversation feed and status rail as the functional baseline; this plan owns broader visual integration and accessibility hardening |

## Non-Goals

- Changing translation, audio, OCR, summary, or LLM behavior.
- Adding new providers, languages, session states, transcript fields, or summary capabilities.
- Rewriting application copy beyond small clarity and consistency edits.
- Changing URL query parameters or Tauri window labels.
- Adding animations for decoration.
- Redesigning the app icon or brand mark.

## Implementation Handoff

Implement in phase order. Phase 1 creates the compatibility and regression foundation. Phases 2-4 may then migrate one surface at a time while the existing handlers remain in place. Phase 5 is required before the redesign is considered complete.

The separate Local LLM destination and its provider-specific form are owned by `plans/260716-2033-local-llm-audio-translation`. Implement that surface using this plan's Fluent shell, responsive settings, focus, and accessibility contracts; do not fold its Whisper/Ollama fields into the Summary profile panel.
