# Phase 12: HY-MT Settings, Readiness, and Language Rules

## Context

Backend engine/runtime contracts exist, but Local LLM settings still require Ollama URL/model fields and present TranslateGemma-only copy. This phase exposes an accessible engine choice and HY-MT installation/readiness without changing the live worker yet.

## Requirements

- User-facing provider remains **Local Whisper**; translation engine is an advanced local setting.
- Engine selection is explicit and saved; switching invalidates the last readiness test.
- Ollama fields appear and validate only for `ollama`.
- HY-MT shows fixed model/revision, download/runtime size, device capability, progress, install/cancel/retry/repair/restart/unload actions, and offline-ready state.
- Users never edit ports, cache paths, Python paths, model IDs, or `device_map`.
- Local test result becomes engine-neutral while retaining detailed Ollama/HY health fields as needed.
- Language options and backend validation respect HY-MT's supported list; the first approved path remains JA→VI.
- Progress/live-region behavior is accessible and avoids noisy per-chunk announcements.

## Related Files

- `src/types.ts`, `src/api.ts`, `src/languages.ts`, `src/languages.test.ts`
- `src/app/MainApp.tsx`, `src/App.test.tsx`
- `src/components/settings/LocalLlmSettings.tsx` and tests
- `src-tauri/src/models.rs`, `src-tauri/src/local_translation.rs`, `src-tauri/src/commands.rs`
- `src/styles/app.css` using existing Fluent tokens.

## Implementation Steps

1. Mirror engine/runtime status/progress/config contracts in TypeScript and add command wrappers/listeners.
2. Replace unconditional Ollama draft validation with engine-specific rules; keep Whisper/TTS/audio validation shared.
3. Add an accessible Translation engine selector with concise Ollama and managed HY-MT descriptions.
4. Render Ollama server/model/generation controls only for Ollama; render an HY-MT managed runtime card only for HY-MT.
5. Model installation UI follows backend state and supports progress, cancel/resume, repair, restart/unload, and explicit switch-to-Ollama recovery.
6. Refresh runtime status on entry, relevant actions, progress milestones, and window focus without polling aggressively.
7. Update Local Whisper pipeline copy from TranslateGemma-specific wording to selected-engine wording.
8. Make source/target option filtering and backend validation engine-aware; assert JA→VI remains selectable for both engines.
9. Add component/app/accessibility tests for every runtime phase, dirty/test invalidation, keyboard flow, live-region behavior, and narrow layout.

## Todo

- [ ] Engine selection persists and invalidates stale readiness.
- [ ] Ollama and HY-MT validation/UI are fully conditional.
- [ ] All HY runtime phases have actionable copy.
- [ ] Language filtering agrees between Rust and TypeScript.
- [ ] Keyboard, screen-reader, zoom, forced-color, and responsive tests pass.

## Risks

- Hiding inactive fields can accidentally discard their saved values. Preserve values across engine switches.
- Frontend-derived readiness can drift from the manager. Treat backend status/test as authoritative.
- Large download progress can flood React/events. Throttle byte updates and announce only state milestones.
- Renaming the serialized provider breaks tests/config. Change labels, not the provider value.

## Success Criteria

- A user can choose HY-MT, install/repair it, see actual device/runtime readiness, save, and run the local pipeline test without system Python or Ollama.
- Switching back to Ollama restores prior URL/model values and current Ollama behavior.
- Invalid engine/language/runtime combinations are rejected consistently in UI and Rust before session start.
- Automated UI and accessibility checks cover success, progress, cancellation, error, repair, and fallback states.
