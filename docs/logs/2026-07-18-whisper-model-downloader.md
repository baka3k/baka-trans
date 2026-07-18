# Whisper Model Downloader — 2026-07-18

## Context

Local translation previously required users to find a compatible multilingual Whisper GGML binary and type its absolute path manually. The local-workspace plan keeps Whisper configuration in settings and requires a readable model before the pipeline can start ([plan](../../plans/260716-2033-local-llm-audio-translation/plan.md)).

## Change

- Added a curated three-model multilingual catalog and Tauri list/download commands (`src-tauri/src/local_translation.rs:35`, `src-tauri/src/commands.rs:154`, `src-tauri/src/commands.rs:159`). Downloads are single-flight, stream into a `.part` file, reject incomplete responses, atomically install the result, clean up failures, and emit typed progress events (`src-tauri/src/local_translation.rs:62`, `src-tauri/src/local_translation.rs:98`, `src-tauri/src/local_translation.rs:174`, `src-tauri/src/local_translation.rs:210`, `src-tauri/src/local_translation.rs:217`, `src-tauri/src/local_translation.rs:237`, `src-tauri/src/local_translation.rs:273`).
- Wired model discovery, progress listening, error handling, and automatic population of the downloaded model path into the local settings draft (`src/api.ts:107`, `src/app/MainApp.tsx:617`, `src/app/MainApp.tsx:707`, `src/app/MainApp.tsx:889`). The settings UI exposes model size/recommendation, disables conflicting actions during transfer, announces status politely, and retains manual-path entry (`src/components/settings/LocalLlmSettings.tsx:230`, `src/components/settings/LocalLlmSettings.tsx:258`, `src/components/settings/LocalLlmSettings.tsx:271`, `src/components/settings/LocalLlmSettings.tsx:284`).
- Added allowlist/percentage unit coverage and a component interaction test for selection, one-shot download, disabled progress state, and progressbar output (`src-tauri/src/local_translation.rs:896`, `src-tauri/src/local_translation.rs:914`, `src/components/settings/LocalLlmSettings.test.tsx:137`). The follow-up polish gives the model selector an explicit accessible name (`src/components/settings/LocalLlmSettings.tsx:241`).

## Impact

**Risk level: medium.** Local users can install a supported Whisper model without leaving the app and receive visible, accessible progress feedback. The feature adds large external downloads and filesystem writes, so network interruption, disk capacity, and upstream availability remain operational risks; allowlisted IDs, one active download, temporary-file installation, completeness checks, and failure cleanup limit corruption and path-injection exposure.

## Decision

Use an application-owned, curated model catalog instead of accepting arbitrary download URLs, while preserving the existing manual absolute-path field for advanced users. Stream to a temporary file and rename only after validation so an interrupted transfer cannot be mistaken for an installed model. Report progress through a Tauri event rather than polling, keeping transfer ownership in Rust and presentation state in React.

## References

- plan: [Local LLM Audio Translation](../../plans/260716-2033-local-llm-audio-translation/plan.md)
- feature commit: `1cdd7057b6f4bcdfb9d3aee098478f35d3c10d28`
- polish commit: `31fc8613815b13fff97f9d2ea0ced3e0a1684729`
