---
type: UI contract audit
date: 2026-07-12
---
# UI Contract Audit: Application Modernization

## Summary

The React container owns all existing state, Tauri subscriptions, derived transcript state, and command handlers. The modernization preserves those functions and initially extracts route selection and presentational layout only.

## Route Contract

| Query | Window | Contract |
| --- | --- | --- |
| no supported `overlay` value | Main app | Session, routing, provider, summary, notes, and conversation workspace |
| `?overlay=transparent` | Look Through | Automatic OCR capture, pause/resume, settings, copy, drag, and close |
| `?overlay=look-help` | Look & Help | Manual capture, request editing, profile settings, copy, drag, and close |

## Main Window Actions

| Surface | Controls | Existing handler or state |
| --- | --- | --- |
| App bar | settings, both overlay launchers, refresh, text export, Markdown export | `setSettingsOpen`, `showTransparentOverlay`, `showLookHelpOverlay`, `refreshAudioDevices`, `doExport` |
| Session | source, target, Start, Pause, Resume, Stop, Translate now, fallback | language state, `startSession`, `pauseSession`, `resumeSession`, `stopSession`, `requestBoundary`, `fallbackEnabled` |
| Audio | auto/manual refresh, device and channel selects, original monitor, signal meter, test tones, local monitor | routing state and update helpers, `refreshAudioDevices`, `testTone`, `toggleLocalMonitor` |
| Translation | provider selection, key input/save/test | provider and key state, `saveKey`, `testStoredKey` |
| Summary | profile selection and CRUD, provider fields, key, test, scope, language, prompt, sections, run | profile and summary state, `saveSummaryProfile`, `testSelectedProfile`, `deleteSelectedProfile`, `runSummary` |
| Conversation | scroll tracking and New translation | `handleConversationScroll`, `jumpToLatestTranslation` |

## Command Enablement Contract

- Start remains gated by idle/error state, routing selections, monitor state, and tone state.
- Pause and Translate now remain tied to the OpenAI realtime active-session rule.
- Resume remains available only while paused; Stop remains unavailable only while idle or stopping.
- Export remains disabled for an empty transcript.
- Summary remains gated by transcript content, a valid selected profile, prompt validation, and non-running state.
- Navigation and settings disclosure are UI-only and must never invoke a backend command.

## Accessibility Contract

- Existing icon-button names, field labels, progressbar values, and polite final-transcript announcements remain stable.
- Partial transcript items remain outside live announcements.
- Overlay drag exclusions continue to cover buttons, inputs, selects, textareas, and links.

## Unresolved Questions

- Real Windows and macOS Tauri smoke checks require each target platform and cannot be completed by browser automation alone.
