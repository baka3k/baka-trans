# Phase 05: Mode Gateway and Workspace Boundary

## Context

The main route currently enters `MainApp` directly and lets users switch Google, OpenAI, and Local inside one large workspace. The requested experience selects Cloud API or Local Whisper before entering the application and gives local translation a focused interface. Overlay windows must remain independent.

## Requirements

- Show the chooser on every main-window launch.
- Resolve transparent and Look & Help overlay routes before the chooser.
- Cloud API must render the current cloud workspace with unchanged behavior.
- Local Whisper must render a dedicated workspace while reusing one session controller and the current audio/transcript features.
- Change mode must not abandon an active session.
- Preserve keyboard, focus, zoom, forced-colors, and responsive behavior.

## Architecture

- Add `ApplicationMode = "cloud" | "local"` in the main route host.
- Add a `ModeChooser` Fluent component with two primary actions and no persisted default.
- Keep Tauri event subscriptions and routing state in one shared session controller or view-model.
- Treat the cloud workspace as the preserve-first baseline. Extract only the seams needed by a new `LocalWhisperWorkspace`.
- Local workspace forces the local provider and fixed `ja -> vi` language contract. It does not show cloud keys.

## Related Files

- `src/App.tsx`
- new `src/app/ModeChooser.tsx`
- `src/app/MainApp.tsx`
- recommended new `src/app/LocalWhisperWorkspace.tsx`
- recommended shared session controller/hook under `src/app/`
- `src/components/session/SessionCommandBar.tsx`
- `src/components/shell/AppNavigation.tsx`
- `src/styles/app.css`
- `src/App.test.tsx` and new component tests

## Implementation Steps

1. Add route tests proving both overlay query strings bypass mode selection.
2. Add `ModeChooser` with Cloud API and Local Whisper actions, focus entry, semantic descriptions, and a one-column narrow layout.
3. Introduce the smallest shared controller/view-model needed to avoid duplicate Tauri listeners and session state.
4. Render the existing cloud workspace for Cloud API without changing its Google/OpenAI command, credential, transcript, or audio paths.
5. Add a local workspace shell that reuses command bar, transcript feed, routing controls, meters, export, and relevant settings.
6. Add Change mode. Disable or block it with an inline message until an active session is stopped.
7. Move local-only configuration entry points out of the cloud provider selector without deleting persisted state.
8. Add chooser and workspace accessibility/responsive tests.

## Todo

- [ ] Main route requires an explicit mode choice.
- [ ] Overlay routes bypass the chooser.
- [ ] Cloud workspace behavior and visible controls remain unchanged.
- [ ] Local workspace has no cloud-key dependency.
- [ ] Event subscriptions are not duplicated.
- [ ] Mode changes are lifecycle-safe.

## Risks

- Extracting state from the large `MainApp.tsx` can cause broad regressions. Extract behavior behind tests and keep the cloud render branch stable.
- A remembered mode would conflict with the explicit launch choice. Do not persist it in this phase.
- A mode switch during active playback can leak audio. Require stop before switching.

## Success Criteria

- `/` opens the chooser, both selections are reachable by keyboard, and no option is preselected.
- Cloud API renders the established cloud workspace and passes its existing frontend tests.
- Local Whisper opens a visibly separate workspace backed by the same session/audio runtime.
- Overlay route tests prove no chooser is inserted into overlay windows.
