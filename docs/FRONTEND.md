# Frontend Guidelines

## Architecture

`src/App.tsx` resolves the main, Look Through, and Look & Help routes. `src/app/MainApp.tsx` owns existing state, effects, derived values, event subscriptions, and backend command handlers. Route-specific files expose the two overlay containers without changing their native contracts.

Presentational shell components live under `src/components/`:

- `shell/AppNavigation.tsx` owns setup destination semantics.
- `shell/ResponsiveSettingsPanel.tsx` owns drawer focus trapping, Escape close, and focus return.
- `session/SessionCommandBar.tsx` owns persistent meeting commands.
- `ui/theme.ts` and `ui/ThemeProvider.tsx` own the shared Fluent theme.
- `styles/app.css` owns the new application composition.
- `styles/overlays.css` owns overlay refinements and native-window-safe composition.
- `styles/legacy.css` is the compatibility layer for existing workflow forms and feed internals.

## Business-Logic Boundary

Visual work must not change Tauri command names, arguments, event names, payloads, state transitions, persistence shapes, validation, transcript derivation, summary configuration, or overlay capture behavior.

Keep these helpers unchanged unless a separate behavior plan requires it:

- `src/api.ts` for native commands.
- `src/transcript.ts` for transcript merging, display derivation, prompt validation, and exports.
- `src/languages.ts` for provider language rules.
- `src/types.ts` for backend data contracts.

Navigation, disclosure, focus return, responsive layout, and theme selection may use UI-only state.

## Component Rules

- Use `@fluentui/react-components` for new controls and `@fluentui/react-icons` for all icons.
- Use one `FluentProvider` per window route through the shared application provider.
- Prefer semantic Fluent tokens over direct color values.
- Preserve stable accessible names when replacing an existing control.
- Keep labels above fields, descriptions adjacent to controls, and validation below fields.
- Final transcript items may use polite live announcements; partial items remain silent.
- Native overlay drag exclusions must continue to include buttons, inputs, selects, textareas, and links.

## Testing

Vitest uses jsdom for component tests. Testing Library covers keyboard and exact-once interaction contracts. `vitest-axe` scans the main route, both overlay routes, navigation, the session command bar, and the responsive settings dialog.

Required checks for frontend changes:

```text
npm test
npm run build
```

Responsive work must also inspect 1440x1000, 1280x800, 1024x768, 960x640, and 720x900. Confirm `document.documentElement.scrollWidth` does not exceed the viewport and live translation appears before any full settings form.
