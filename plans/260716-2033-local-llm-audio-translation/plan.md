---
title: "Local Whisper and Ollama Translation"
status: pending
created: 2026-07-16
---

# Local Whisper and Ollama Translation

## Overview

Add a third realtime translation provider whose hot path is:

```text
Microphone / system audio
  -> existing Rust CPAL capture and resample (PCM16 mono, 16 kHz)
  -> local Whisper speech-to-text (Japanese)
  -> Rust native Ollama request (POST http://localhost:11434/api/chat)
  -> Vietnamese text
  -> transcript-update event
  -> existing React conversation UI
```

The capture, device selection, input level, original-audio monitor, pause/stop lifecycle, and routing-profile persistence remain on the current audio runtime. Only the provider-specific translation stage is added. Google Live Translation and OpenAI Realtime Translation must remain behaviorally unchanged.

### Scope interpretation

- “Audio config for LLM” is treated as configuration for the local audio-to-text boundary: fixed 16 kHz PCM input plus tunable utterance segmentation, Whisper model/runtime settings, and validation. Ollama `/api/chat` receives Japanese text and returns Vietnamese text; it does not receive audio.
- The local provider is text-only in this phase because the requested pipeline ends at Vietnamese text. Local TTS and translated-audio playback are explicit non-goals. Existing cloud providers continue to use the translated-audio output and meter.
- The first local-provider contract is intentionally Japanese (`ja`) to Vietnamese (`vi`). General language-pair support can be added after this path is stable.

## Current-State Findings

- `src-tauri/src/session.rs` already selects a provider-specific capture sample rate and starts a shared CPAL capture channel, but it always loads a cloud API key and always creates translated-audio playback before dispatching to Google or OpenAI.
- Both existing providers use 16 kHz input or can reuse the same capture/resample code. No local Whisper dependency or model loader exists in `src-tauri/Cargo.toml`.
- `src-tauri/src/llm.rs` supports reusable summary/Look & Help profiles through OpenAI-compatible `/chat/completions`. Its Ollama default is `http://localhost:11434/v1`; this is not the requested native Ollama `/api/chat` contract.
- LLM profile controls currently live under Summary settings. Reusing those controls directly would couple a latency-sensitive translation runtime to summary-agent tuning and would not hold Whisper/audio segmentation settings.
- `src/transcript.ts::mergeTranscriptDelta` merges a single-sided update into the last unfinished item rather than finding an item by stable `id`. A delayed local LLM result can therefore attach to the wrong utterance or duplicate a card.
- The app navigation has `live`, `audio`, `translation`, and `summary`; a dedicated Local LLM destination can be added without mixing it into the cloud credential panel.

## Architecture Decision

### Provider and configuration boundary

Add `local_whisper_ollama` to `TranslationProvider`, but keep local runtime settings in a dedicated persisted `LocalTranslationConfig` rather than reusing `LlmProviderProfile`.

Recommended configuration contract:

| Group | Field | Default / rule |
| --- | --- | --- |
| Ollama | `baseUrl` | `http://localhost:11434`; normalize to exactly `/api/chat` |
| Ollama | `model` | required, user-selected installed model |
| Ollama | `timeoutSeconds` | 30, clamped to a safe range |
| Ollama | `temperature` | 0.0 for deterministic translation |
| Ollama | `maxOutputTokens` | bounded for short utterances |
| Ollama | `keepAlive` | optional native Ollama keep-alive value |
| Whisper | `modelPath` | required readable local GGML model path; do not bundle a model in this phase |
| Whisper | `language` | `ja`, fixed for the first implementation |
| Whisper | `threads` | bounded to available CPU capacity |
| Whisper | `useGpu` | capability-dependent toggle with a safe CPU fallback |
| Audio/STT | `sampleRateHz` | fixed/read-only `16000`; reject incompatible persisted values |
| Audio/STT | `minimumSpeechMs` | ignore very short noise bursts |
| Audio/STT | `silenceToCommitMs` | close an utterance after trailing silence |
| Audio/STT | `maximumUtteranceMs` | force a boundary to cap latency and memory |
| Audio/STT | `preRollMs` | retain a small buffer before speech begins |

Persist this as `local-translation-config.json` beside the existing `llm-profiles.json`. Add Tauri commands to get, save, validate, and test the config. The test command must validate the Whisper model path and issue a non-streaming native Ollama `/api/chat` probe; it must not require or store an API key.

### Runtime boundary

Add `src-tauri/src/ai/local_whisper_ollama.rs` and keep `session.rs` as the lifecycle orchestrator.

1. `session.rs` starts the existing capture runtime at 16 kHz and the existing optional original monitor.
2. It does not start translated-audio playback for the local text-only provider and does not require `outputDeviceId` for this provider.
3. A bounded segmenter receives `Vec<i16>`, applies the configured speech/silence boundaries, and flushes on manual Translate Now, stop, or maximum utterance length.
4. Whisper runs through `spawn_blocking` so native inference never blocks the Tokio/Tauri event loop. Load one model context per session and release it at session end.
5. Each committed utterance receives a stable ID and monotonically increasing sequence/revision. Emit the Japanese source text immediately as a pending translation.
6. Send one ordered Ollama request at a time with `stream: false`, a translation-only system prompt, the selected model, and native `options`. Bound queues so a slow model cannot grow memory without limit.
7. Upsert the Vietnamese result into the same transcript item and emit a snapshot update. Errors update the same item to `error` without ending unrelated completed items.
8. Stop/pause/manual-boundary control uses the existing session control channel. Cancellation must prevent late Ollama responses from mutating a stopped or newer session.

### Native Ollama request contract

```json
{
  "model": "<configured model>",
  "stream": false,
  "messages": [
    {
      "role": "system",
      "content": "Translate Japanese to Vietnamese. Return only the translation. Preserve names, numbers, and technical terms. Do not explain."
    },
    { "role": "user", "content": "<Japanese Whisper text>" }
  ],
  "options": {
    "temperature": 0,
    "num_predict": "<configured limit>"
  }
}
```

Parse `message.content`, reject an empty result, surface native Ollama `error` values, and include bounded latency metadata. URL normalization must accept either the server origin or a full `/api/chat` endpoint and must never append `/v1/chat/completions` for this runtime.

### Transcript event and UI reconciliation

Extend the transcript update contract with explicit semantics instead of inferring behavior only from which text field is empty:

- stable `id` per utterance;
- monotonic `revision` per item;
- `updateMode: "delta" | "snapshot"` (existing providers use delta, local provider uses snapshots);
- source-only snapshot means “translation pending,” not a new conversation card;
- translated snapshot with the same ID replaces the pending state;
- stale or duplicate revisions are ignored.

Update both backend storage and `src/transcript.ts` to reconcile by `id` first. Preserve the current last-item delta fallback for Google/OpenAI until those providers are migrated to stable snapshot events. The conversation view must keep source and Vietnamese text on one card, preserve ordering when LLM responses are delayed, and scroll only under the existing at-bottom rules.

## Phases

| Phase | Document | Outcome |
| --- | --- | --- |
| 01 | [phase-01-contracts-and-config.md](phase-01-contracts-and-config.md) | Provider types, persisted local configuration, native Ollama client contract, and Tauri commands |
| 02 | [phase-02-local-runtime.md](phase-02-local-runtime.md) | Existing 16 kHz capture connected to segmentation, Whisper, ordered Ollama translation, and lifecycle control |
| 03 | [phase-03-settings-and-transcript-ui.md](phase-03-settings-and-transcript-ui.md) | Separate Local LLM settings UI, provider-aware readiness, and stable text rendering |
| 04 | [phase-04-verification-and-documentation.md](phase-04-verification-and-documentation.md) | Regression suite, local smoke test, docs, packaging notes, and acceptance evidence |

## Dependencies

### Cross-plan coordination

- Coordinates with `plans/realtime-meeting-translation-macos`: that plan owns the session foundation; this plan owns the new local provider path and transcript reconciliation required by it.
- Coordinates with `plans/260712-2234-application-ui-modernization`: that plan owns Fluent shell/layout; this plan adds one destination and provider-specific form while following the existing component and accessibility conventions.
- Neither plan is a hard blocker because the necessary capture, event bus, conversation feed, and responsive settings shell already exist in current source.

### Runtime dependencies

- A Rust Whisper binding backed by whisper.cpp (recommended: `whisper-rs`) and its platform build requirements.
- A user-provided compatible Whisper GGML model file.
- A running local Ollama server with the configured model already installed.
- No cloud translation credential for the local provider.

## File Impact Map

| Area | Expected files |
| --- | --- |
| Rust contracts | `src-tauri/src/models.rs` |
| Local config/native Ollama | new `src-tauri/src/local_translation.rs` |
| Whisper + pipeline | new `src-tauri/src/ai/local_whisper_ollama.rs`, `src-tauri/src/ai.rs` |
| Session lifecycle | `src-tauri/src/session.rs` |
| Tauri commands | `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` |
| Rust dependencies | `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` |
| Frontend bridge/contracts | `src/api.ts`, `src/types.ts` |
| Settings shell/UI | `src/components/shell/AppNavigation.tsx`, `src/components/shell/ResponsiveSettingsPanel.tsx`, `src/app/MainApp.tsx`, `src/styles/app.css` |
| Text reconciliation/tests | `src/transcript.ts`, `src/transcript.test.ts`, relevant component tests |
| Documentation | `README.md`, audio platform guides as applicable, `docs/baka-trans-architecture.drawio` after behavior is verified |

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Whisper inference blocks capture/UI | Run inference in `spawn_blocking`, keep capture channel bounded, and test level events during inference |
| Slow Ollama creates an unbounded backlog | One ordered translator worker, bounded utterance queue, maximum utterance length, visible overload error |
| Translation attaches to the wrong source card | Stable ID + revision + snapshot contract; frontend upsert-by-ID tests with delayed/out-of-order fixtures |
| Local provider still requires translated output | Make routing validation and playback creation provider-aware; preserve cloud-provider requirements |
| Native Whisper dependency complicates Windows/macOS builds | Add CI/release checks in Phase 04; document CPU baseline and optional GPU behavior; do not silently require GPU |
| Invalid or missing model path fails after Start | Validate on save/test and again before session start with actionable error codes |
| Ollama endpoint accidentally uses OpenAI compatibility path | Separate native client and URL normalizer from `llm.rs`; assert exact `/api/chat` requests in tests |
| Prompt leakage or explanatory text reaches UI | Deterministic prompt, low temperature, output cleanup, empty/error validation, and prompt-response unit fixtures |
| Existing Google/OpenAI transcript behavior regresses | Keep legacy delta fallback and run the current provider merge/session tests unchanged |

## Success Criteria

- Selecting Local LLM captures through the current audio runtime as mono PCM16 at exactly 16 kHz.
- A Japanese utterance produces a Japanese source item, then Vietnamese text on the same conversation item after a native `POST /api/chat` request.
- No `/v1/chat/completions` request is used by the local translation runtime.
- Local settings persist across app restart and expose Ollama, Whisper, and audio segmentation validation separately from Summary profiles.
- Local text-only mode can start without a cloud API key or translated-audio output; Google/OpenAI retain their current credential and playback requirements.
- Delayed, duplicate, stale, empty, and failed translation updates do not create duplicate cards or overwrite a later utterance.
- Pause, resume, Translate Now, stop, and app shutdown leave no active Whisper/Ollama worker or late transcript mutation.
- Existing audio capture, input meter, original monitor, cloud providers, summary profiles, transcript export, and settings accessibility tests remain green.
- `npm test`, `npm run build`, `cargo test`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings` pass on the supported development platform; Windows/macOS release checks are recorded before completion.

## Implementation Handoff

Implement in phase order. Do not begin Phase 02 until the configuration and event contracts in Phase 01 are covered by tests. Do not remove the legacy transcript delta path until both existing cloud providers emit stable snapshot events.

Suggested command:

```text
/hi-craft plans/260716-2033-local-llm-audio-translation/plan.md
```
