# Local Whisper and Ollama Translation — 2026-07-16

## Context

The local translation plan required a third realtime provider that keeps the existing audio runtime while replacing cloud translation with Japanese Whisper transcription and native Ollama Vietnamese translation (`plans/260716-2033-local-llm-audio-translation/plan.md:40`). It also required local operation without cloud credentials or translated-audio playback.

## Change

- Added a dedicated, persisted local translation configuration, Whisper model validation, and an exact native Ollama `/api/chat` client contract (`src-tauri/src/local_translation.rs:116`, `src-tauri/src/local_translation.rs:127`, `src-tauri/src/local_translation.rs:313`).
- Added a bounded PCM segmenter and ordered Whisper-to-Ollama worker with stable transcript IDs, snapshot revisions, overload handling, and cancellation checks (`src-tauri/src/ai/local_whisper_ollama.rs:59`, `src-tauri/src/ai/local_whisper_ollama.rs:241`, `src-tauri/src/ai/local_whisper_ollama.rs:422`).
- Made session startup provider-aware so the local text-only path uses 16 kHz capture but skips translated-audio playback (`src-tauri/src/session.rs:413`).
- Added revision-aware snapshot reconciliation and a dedicated Local LLM settings/readiness flow while retaining the legacy cloud delta path (`src/transcript.ts:95`, `src/app/MainApp.tsx:393`, `src/components/settings/LocalLlmSettings.tsx:65`).
- Documented model prerequisites, native endpoint behavior, troubleshooting, and the revised runtime topology (`README.md:27`, `docs/baka-trans-architecture.mmd:32`).

## Impact

Users can run Japanese-to-Vietnamese meeting translation entirely through a user-supplied Whisper model and local Ollama server, with source and translation kept on one conversation card. Google and OpenAI retain their existing credential, playback, and delta-update behavior. **Risk level: medium** because native model loading, bounded asynchronous work, and pause/stop cancellation extend the session lifecycle; segmentation, payload, snapshot ordering, and late-mutation tests reduce that risk (`src-tauri/src/ai/local_whisper_ollama.rs:715`, `src-tauri/src/ai/local_whisper_ollama.rs:746`, `src-tauri/src/local_translation.rs:589`).

## Decision

Use a separate local-provider configuration and native Ollama client instead of reusing Summary LLM profiles or the OpenAI-compatible endpoint. Keep one ordered translation worker and stable revisioned snapshots so slow responses cannot attach to later utterances, while preserving the existing last-item delta fallback for cloud providers. Keep translated audio out of this phase because the requested local pipeline terminates in Vietnamese text.

## References

- plan: `../../plans/260716-2033-local-llm-audio-translation/plan.md`
- architecture: `../baka-trans-architecture.mmd:32`
- local runtime smoke hook: `../../src-tauri/src/ai/local_whisper_ollama.rs:842`
