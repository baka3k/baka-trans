# Responsive UI Redesign — 2026-07-18

## Context

The responsive application shell was breaking at constrained window sizes and obscuring the live workspace behind rigid viewport offsets. This follow-up implements the live-first, modern Fluent direction defined in the [Application UI Modernization plan](../../plans/260712-2234-application-ui-modernization/plan.md).

## Change

- Reworked the shell, navigation, workspace, translation, and settings height/overflow chain so the app occupies one viewport without document-level overflow (`src/styles/app.css:196`, `src/styles/app.css:303`, `src/styles/app.css:337`, `src/styles/app.css:355`, `src/styles/app.css:364`). At widths below 1280px, settings now overlays the workspace relative to its container; at 720px it becomes full-width while the compact top bar and command bar remain usable (`src/styles/app.css:622`, `src/styles/app.css:637`, `src/styles/app.css:740`, `src/styles/app.css:765`, `src/styles/app.css:798`, `src/styles/app.css:831`).
- Added a contextual **Open audio settings** action when the empty conversation state detects missing input or output devices, with interaction coverage for opening the Audio dialog (`src/app/MainApp.tsx:2099`, `src/app/MainApp.tsx:2851`, `src/app/MainApp.tsx:2858`, `src/App.test.tsx:79`).
- Refined the teal-green brand ramp and explicit light/dark neutral tokens, then applied accessible on-brand colors to primary empty-state and overlay capture actions (`src/ui/theme.ts:8`, `src/ui/theme.ts:30`, `src/ui/theme.ts:50`, `src/styles/app.css:597`, `src/styles/app.css:605`, `src/styles/overlays.css:42`).

## Impact

**Risk level: medium.** Users retain immediate access to live translation and settings across wide, compact, and narrow layouts, with clearer setup recovery and more consistent light/dark visual hierarchy. The main residual risk is responsive regression across native window sizes because the change touches shared height, overflow, and breakpoint behavior; the focused dialog interaction test protects the new recovery path but visual smoke checks remain important.

## Decision

Keep the existing Fluent component and application-state architecture, and repair layout through a single bounded grid/overflow chain plus container-relative settings overlays. Hard-coded viewport subtraction was rejected because top-bar and command-bar heights vary by breakpoint; rewriting application logic was also unnecessary for a presentation-layer defect. Semantic theme tokens were retained so main and overlay surfaces share one modern palette without changing backend or session contracts.

## References

- plan: [Application UI Modernization](../../plans/260712-2234-application-ui-modernization/plan.md)
- commit: `15cd7ae62598a806e83383c1a9df5655fb31bc75`
