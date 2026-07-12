# Phase 5: Accessibility, Responsive, and Regression Hardening

Status: planned
Depends on: phases 02, 03, and 04
Primary files: `src/**/*.test.tsx`, `src/styles/app.css`, `src/styles/overlays.css`, `vite.config.ts`, `docs/FRONTEND.md`, `docs/DESIGN.md`

## Objective

Prove that the redesign preserves functionality and meets the responsive and WCAG-friendly quality bar before release.

## Work

1. Complete automated coverage.
   - Add axe assertions for the main shell, each settings destination, live translation states, notes, errors, Look Through, and Look & Help.
   - Add keyboard tests for navigation, menus, drawers, segmented controls, disclosures, and form submission.
   - Add exact-once command tests for all user actions in the Phase 1 contract inventory.
   - Keep existing transcript and language tests unchanged and passing.

2. Run responsive verification.
   - Check 1,440x1,000, 1,280x800, 1,024x768, 960x640, and 720x900.
   - Check overlay minimum, typical, and wide sizes.
   - Verify no horizontal document overflow, clipped focus rings, wrapped primary actions, or off-screen menus.
   - Verify live translation remains the first content workspace at compact widths.

3. Run accessibility verification.
   - Verify WCAG AA contrast for text, icons, input boundaries, focus rings, and state treatments.
   - Verify light, dark, and forced-colors modes.
   - Verify 200% zoom and text-spacing overrides.
   - Verify reduced motion and no reliance on color alone.
   - Verify polite live announcements do not repeat partial transcript tokens.

4. Run platform smoke tests.
   - Use `npm run tauri -- dev` on Windows and macOS.
   - Exercise session commands, routing tools, provider credentials, summary profile CRUD, summary run, export, overlay launch, capture, pause, resize, drag, and close.
   - Confirm no focus or theme behavior conflicts with native window controls.

5. Measure performance and bundle impact.
   - Compare production bundle size before and after Fluent migration.
   - Confirm tree-shaken component and icon imports.
   - Confirm transcript rendering remains responsive with a long fixture and no avoidable layout shift.
   - Confirm INP-sensitive actions respond without blocking work on the main thread.

6. Document the final system.
   - Update `docs/DESIGN.md` with theme, spacing, typography, status, radius, icon, and responsive rules.
   - Update `docs/FRONTEND.md` with component boundaries, testing rules, and the business-logic preservation boundary.

## Verification

- `npm test`
- `npm run build`
- Windows Tauri development smoke test.
- macOS Tauri development smoke test.
- Manual accessibility and responsive checklist recorded in the implementation log.

## Acceptance Criteria

- Automated tests cover the complete UI action contract and core accessibility rules.
- No critical or serious axe violations remain in tested states.
- All target widths and overlay sizes are usable without horizontal overflow.
- Keyboard, focus, forced-colors, reduced-motion, light, dark, and 200% zoom checks pass.
- Production build and existing business-logic tests pass.
- Design and frontend documentation describe the implemented system.
