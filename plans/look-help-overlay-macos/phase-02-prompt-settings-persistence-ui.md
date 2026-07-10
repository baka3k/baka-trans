# Phase 02: Prompt Settings, Persistence, and Overlay UI

## Goal

Add the user-facing Look & Help overlay experience: answer area, source preview, settings button, system prompt editor, profile selection, and persistent helper configuration.

## Tasks

1. Add `LookHelpConfig` and related frontend/Rust DTOs:
   - provider profile ID.
   - system prompt.
   - prompt panel visibility preference.
   - opacity, capture interval, minimum confidence, max OCR input chars.
2. Add config persistence in Rust:
   - store non-secret Look & Help config in app config JSON.
   - reuse existing LLM profile secrets; do not copy API keys.
3. Add commands:
   - `look_help_status`
   - `open_look_help_window`
   - `close_look_help_window`
   - `update_look_help_geometry`
   - `set_look_help_paused`
   - `get_look_help_config`
   - `save_look_help_config`
4. Build helper overlay UI route:
   - title: `Look & Help`.
   - draggable titlebar.
   - settings icon button.
   - collapsible prompt/settings panel.
   - system prompt textarea.
   - LLM profile selector using existing profile list.
   - answer display surface.
   - source OCR preview.
   - opacity control.
5. Add empty/error states:
   - no LLM profile selected.
   - selected profile disabled/missing key.
   - screen recording permission needed.
   - no readable text.
6. Keep compact overlay ergonomics:
   - settings panel should scroll inside the overlay.
   - answer text must not overlap controls.
   - prompt editor should hide by default after configuration.

## Files

- `src-tauri/src/models.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/overlay.rs` or `src-tauri/src/overlay/help.rs`
- `src/App.tsx`
- `src/api.ts`
- `src/types.ts`
- `src/styles.css`

## Acceptance

- User can show/hide the system prompt from inside the overlay.
- Prompt edits are saved and restored after app restart.
- User can select an existing LLM profile for Look & Help.
- Missing profile/key states are visible and actionable.
- No raw OCR text or prompt is written to meeting transcript history.

## Validation

- `npm run build`
- Manual: select profile, edit prompt, close/reopen overlay, confirm settings persist.
- Manual: small window sizes still show usable controls without overlap.
