# Phase 01: Overlay Window Shell and Lifecycle

## Goal

Add the user-visible overlay entry point and secondary Tauri window without screen capture yet.

## Tasks

1. Add frontend state and an icon/text button labeled "Xuyen thau" in the main app toolbar or session control area.
2. Add Tauri commands:
   - `open_overlay_window`
   - `close_overlay_window`
   - `overlay_status`
3. Create a secondary webview window with:
   - label: `transparent-overlay`
   - transparent background
   - always on top
   - decorations disabled
   - resizable enabled
   - minimum dimensions
4. Add overlay UI route/component:
   - drag handle
   - resize-friendly content layout
   - opacity setting
   - scanning/status area
   - translated text area
5. Emit overlay geometry updates from the overlay window to Rust.
6. Ensure opening the button twice focuses the existing overlay instead of creating duplicates.

## Files Likely Touched

- `src/App.tsx`
- `src/api.ts`
- `src/types.ts`
- `src/styles.css`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/models.rs`
- `src-tauri/tauri.conf.json`

## Verification

- `npm run build`
- `cargo test` from `src-tauri`
- Manual: open, move, resize, close, reopen overlay.

