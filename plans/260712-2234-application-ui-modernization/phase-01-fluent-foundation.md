# Phase 1: Fluent Foundation and UI Contracts

Status: planned
Depends on: none
Primary files: `package.json`, `package-lock.json`, `src/main.tsx`, `src/styles.css`, `src/ui/theme.ts`, `src/ui/ThemeProvider.tsx`, `vite.config.ts`

## Objective

Introduce the official Fluent 2 foundation and a regression contract before moving any functional markup.

## Work

1. Record the current UI contract.
   - Inventory every button, input, select, checkbox, progress bar, live region, and overlay control in `src/App.tsx`.
   - Map each control to its current handler, disabled rule, value source, validation text, and accessible name.
   - Record the current route conditions for the main app, Look Through, and Look & Help.

2. Add dependencies.
   - Add `@fluentui/react-components` and `@fluentui/react-icons`.
   - Add Testing Library, user-event, jsdom, and an axe integration suitable for Vitest.
   - Keep `lucide-react` during migration; remove it only when no imports remain.

3. Create one shared theme.
   - Build a brand ramp from the current green accent.
   - Define light and dark Fluent themes with semantic success, information, warning, and danger tokens.
   - Follow `prefers-color-scheme` without adding persistence or a manual theme setting.
   - Add forced-colors and reduced-motion global behavior.

4. Establish typography and layout constants.
   - Use `Segoe UI Variable`, `Segoe UI`, and platform system fallbacks.
   - Define app bar, navigation, context panel, control-height, spacing, radius, and z-index constants.
   - Keep Tauri overlay transparency and drag-region rules outside Fluent component internals.

5. Add a minimal test harness.
   - Render the three routes without a Tauri runtime by mocking bridge functions.
   - Assert that route selection and existing accessible names remain available.
   - Add a command-spy helper so later phases can prove UI actions still call existing handlers exactly once.

## Verification

- `npm test`
- `npm run build`
- Render the main app and both overlay query routes in the browser.
- Confirm system light/dark switching updates theme tokens without remounting forms.
- Confirm no production command or event contract changed.

## Acceptance Criteria

- FluentProvider and the custom theme can wrap every route.
- Existing controls still render through the compatibility layer.
- Tests can render each route without native Tauri services.
- The control-handler inventory is complete enough to validate later extraction.
- No user-facing workflow has moved yet.
