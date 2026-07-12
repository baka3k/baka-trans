# Phase 4: Overlay Modernization

Status: planned
Depends on: phases 01 and 03
Primary files: `src/App.tsx`, `src/app/TransparentOverlayWindow.tsx`, `src/app/LookHelpOverlayWindow.tsx`, `src/components/overlays/*`, `src/styles/overlays.css`

## Objective

Bring Look Through and Look & Help into the same Fluent visual system while preserving transparent-window capture and Tauri window behavior.

## Work

1. Extract overlay containers.
   - Move existing overlay state, event subscriptions, timers, geometry updates, and handlers without rewriting behavior.
   - Share title-bar, status-bar, panel, empty-state, result, copy, and settings components where their contracts genuinely match.

2. Modernize Look Through.
   - Preserve draggable title bar, settings, close, live status, pause/resume, detected screen text, translated text, copy, language metadata, and automatic detection messaging.
   - Keep source and translation panels balanced at typical sizes and stacked at constrained widths.
   - Increase type size and contrast while maintaining transparency.

3. Modernize Look & Help.
   - Preserve draggable title bar, settings, close, capture, OCR text, request editor, result, copy, selected-profile status, and manual-capture messaging.
   - Keep the result as the largest region and prevent the request editor from crowding it at compact sizes.

4. Standardize overlay settings.
   - Use a consistent disclosure pattern and control spacing.
   - Preserve every range, select, checkbox, prompt, provider, cadence, opacity, confidence, and character-limit value.
   - Avoid changing the visible result height when a compact settings disclosure opens unless the window cannot support an overlay panel.

5. Preserve native behavior.
   - Retain drag exclusions for buttons, inputs, selects, textareas, and links.
   - Retain current geometry updates, transparent background behavior, pointer behavior, and window close commands.
   - Verify Windows and macOS window controls remain usable.

## Verification

- Route tests for `?overlay=transparent` and `?overlay=look-help`.
- Handler tests for settings, close, pause/resume, capture, copy, and input updates.
- Visual checks at configured minimum, typical, and wide window sizes.
- Light, dark, opacity, reduced-motion, and forced-colors checks.
- Real Tauri drag, resize, capture, and close smoke tests on Windows and macOS.

## Acceptance Criteria

- Both overlays clearly belong to the same application as the main window.
- Text is readable at typical overlay dimensions without relying on browser zoom.
- Capture or pause remains the dominant action.
- Result content keeps priority over settings and metadata.
- Transparency, dragging, resizing, capture, pause, copy, settings, and close behavior are unchanged.
