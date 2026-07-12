# Design

## Product Direction

Baka Trans uses Fluent 2 as a calm, professional desktop utility for live meeting translation. The interface prioritizes fast state recognition, persistent session controls, and a live-first workspace over decorative presentation.

Design dials:

- Design variance: 3. Predictable placement and alignment.
- Motion intensity: 3. Motion only for disclosure and state feedback.
- Visual density: 6. Compact setup controls with generous live-output space.

## Theme

`src/ui/theme.ts` creates light and dark Fluent themes from one green brand ramp. `ApplicationThemeProvider` follows `prefers-color-scheme` without remounting window content. The app uses Fluent semantic tokens for surfaces, foregrounds, strokes, brand controls, status colors, and elevation.

- Green communicates brand, readiness, and primary actions.
- Blue communicates information and in-progress activity.
- Amber communicates warnings and required setup.
- Red communicates errors and destructive actions.
- Status always includes text or an icon and never relies on color alone.

Forced-colors mode preserves visible focus and control boundaries. Reduced-motion mode removes non-essential animation and smooth scrolling.

## Typography and Shape

- Font stack: Segoe UI Variable, Segoe UI, Apple system fonts, then system UI.
- Persistent interface text is at least 13px; normal form and body content targets 14px.
- Controls use an 8px radius; major workspace surfaces use 12px.
- Main controls are at least 40px high; compact overlay controls are at least 36px high.
- Spacing follows a 4px base with common gaps of 8, 12, 16, 24, and 32px.

## Application Layout

The top app bar holds identity, health, overlay launchers, refresh, and export. A persistent command bar holds source and target languages plus Start, Pause or Resume, Stop, and Translate now.

The workspace contains Live, Audio, Translation, and Summary destinations:

- At 1280px and wider, navigation, live output, and a 360-400px context panel are visible together.
- From 1041px to 1279px, settings use a right overlay panel.
- At 1040px and below, navigation becomes horizontal and settings use a modal drawer.
- At 720px and below, the drawer is full width and session actions remain in one horizontally scrollable command row.

Live translation remains the first content workspace at every width. Document-level horizontal overflow is not permitted.

## Overlays

Look Through and Look & Help use the same tokens, type scale, state colors, radii, and Fluent icon family as the main app. Transparent native window behavior, Tauri drag regions, capture cadence, pause behavior, geometry, and close commands remain unchanged.

The configured minimum window sizes are 360x420 for Look Through and 380x500 for Look & Help. Pause or Capture remains the visually dominant action, while result content keeps priority over settings and metadata.
