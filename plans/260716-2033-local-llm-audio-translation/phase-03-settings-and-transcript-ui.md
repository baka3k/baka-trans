# Phase 03: Local LLM Settings and Transcript UI

## Context

The settings shell currently separates Audio, Translation, and Summary. Local translation needs a distinct configuration surface, while the conversation feed needs ID-aware reconciliation for asynchronous Whisper and Ollama updates.

## Requirements

- Add a separate Local LLM navigation destination and responsive settings panel state.
- Keep local translation configuration independent from Summary Agent profiles.
- Show actionable readiness for Ollama, Whisper, and audio segmentation.
- Make Start validation provider-aware.
- Render source and translated snapshots on one card without duplicates or cross-utterance pairing.

## Architecture

- Extend `SettingsSection` with `local_llm` and add a dedicated panel/component rather than adding more conditionals to the cloud key panel.
- Keep Audio settings responsible for devices and original monitor. The Local LLM panel owns Whisper/Ollama/segmentation settings only.
- Add Local LLM to the translation provider selector. When selected, replace cloud credential messaging with local readiness plus a shortcut to the Local LLM destination.
- Reconcile transcript events by `id` and `revision`; use `updateMode` to distinguish full snapshots from cloud deltas.

## Related Files

- `src/components/shell/AppNavigation.tsx`
- `src/components/shell/ResponsiveSettingsPanel.tsx`
- recommended new `src/components/settings/LocalLlmSettings.tsx`
- `src/app/MainApp.tsx`
- `src/styles/app.css`
- `src/api.ts`
- `src/types.ts`
- `src/transcript.ts`
- `src/transcript.test.ts`
- relevant navigation/settings component tests

## Implementation Steps

1. Add the Local LLM destination, icon, title, responsive visibility rules, focus handling, and keyboard navigation tests.
2. Hydrate the persisted local config with app startup data; keep a draft, dirty state, validation errors, save state, and test result.
3. Build grouped controls for Ollama server/model/tuning, Whisper model/runtime, and Audio-to-text segmentation. Display 16 kHz as a fixed capability, not an editable arbitrary sample rate.
4. Show separate health states: config saved, Whisper model readable/loaded, Ollama reachable/model accepted. Do not label the provider ready based only on a saved path.
5. Add `Local LLM` to the provider selector and enforce `ja -> vi` for its first version. Explain the text-only output and keep cloud fields intact when switching back.
6. Update `canStart` and routing warnings: local mode requires input plus valid/tested local config; cloud modes retain API key and translated output requirements.
7. Update transcript reconciliation to upsert snapshots by ID, ignore stale revisions, retain legacy delta merging, and preserve the existing scrolled-away/new-translation affordance.
8. Render pending/error states on the same card. Do not show an indefinitely pending spinner after an item-level error.
9. Add tests for config hydration/save/test, provider switching, readiness, keyboard/focus behavior, same-ID replacement, out-of-order revisions, duplicate events, two pending utterances, empty translation, and local error rendering.

## Todo

- [ ] Local LLM has a separate accessible settings destination.
- [ ] Summary profile editing remains unchanged.
- [ ] Provider switching preserves each provider's draft/state.
- [ ] Local Start requirements match the backend.
- [ ] Transcript snapshots cannot duplicate or attach to the wrong card.

## Risks

- `MainApp.tsx` is already large. Extract the new form instead of adding another large inline panel.
- A “tested” flag can become stale after edits. Clear readiness whenever endpoint, model path, model name, or runtime-critical audio settings change.
- Hiding translated audio controls can destroy saved routing state. Leave the saved profile untouched; disable/explain only the controls that are irrelevant while local text-only mode is selected.

## Success Criteria

- A user can configure and test local translation without opening Summary settings or entering an API key.
- Local mode clearly communicates Japanese-to-Vietnamese, 16 kHz input, and text-only output.
- Each utterance appears once; Japanese arrives first and Vietnamese fills the same card later even under delayed/stale event fixtures.
- Navigation, focus trap, 200% zoom, forced colors, and reduced-motion checks remain valid.
