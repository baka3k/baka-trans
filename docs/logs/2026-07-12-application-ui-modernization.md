# Application UI Modernization — 2026-07-12

## Context

The desktop UI needed a calm, accessible Fluent 2 refresh that kept live translation visible at constrained widths without changing session, translation, summary, OCR, or Tauri command contracts. The implementation follows the phased modernization plan and its preservation rules (`plans/260712-2234-application-ui-modernization/plan.md:35`).

## Change

- Added Fluent UI, Fluent icons, and a jsdom/Vitest UI-test foundation (`package.json:11`, `package.json:20`, `vite.config.ts:14`), then mounted one OS-responsive green brand theme around every application route (`src/main.tsx:8`, `src/ui/theme.ts:8`, `src/ui/ThemeProvider.tsx:7`).
- Reduced the application entry point to explicit main and overlay routing (`src/App.tsx:5`, `src/App.tsx:18`) while retaining state and backend command wiring in the main container. A persistent session command bar and Live/Audio/Translation/Summary navigation now drive a live-first workspace with a focus-managed responsive settings panel (`src/app/MainApp.tsx:1157`, `src/app/MainApp.tsx:1182`, `src/components/shell/ResponsiveSettingsPanel.tsx:33`).
- Added wide, compact, and narrow layout behavior plus reduced-motion and forced-colors handling (`src/styles/app.css:277`, `src/styles/app.css:328`, `src/styles/app.css:396`, `src/styles/app.css:477`, `src/styles/app.css:488`). Look Through and Look & Help now share the same themed title, status, action, disclosure, and result structure while preserving their existing geometry and event flows (`src/app/MainApp.tsx:1900`, `src/app/MainApp.tsx:2174`, `src/styles/overlays.css:19`).
- Added route/accessibility, single-invocation command, unsaved-settings persistence, and drawer focus-return coverage (`src/App.test.tsx:7`, `src/components/session/SessionCommandBar.test.tsx:40`, `src/components/shell/ResponsiveSettingsPanel.test.tsx:7`).

## Impact

**Risk level: medium.** Meeting users now reach live translation and primary session actions before advanced setup at desktop and narrow widths, and both overlays have more consistent, readable controls. The main risk is presentation-layer regression across responsive states and native overlay windows; keeping command wiring in the existing container and adding interaction/accessibility contracts limits that risk.

## Decision

Adopt Fluent UI React v9 with a single semantic green theme and migrate incrementally around the existing application container. A full state or business-logic rewrite was rejected because it would increase the chance of changing hook timing, command enablement, persistence, and native event behavior; the new route shell and typed presentational components instead preserve those boundaries while making future extraction safer.

## References

- plan: `../../plans/260712-2234-application-ui-modernization/plan.md`
- coordinated plan: `../../plans/realtime-meeting-translation-macos/phase-16-conversation-translation-ui-redesign.md`
