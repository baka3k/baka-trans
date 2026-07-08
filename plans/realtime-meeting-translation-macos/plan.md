# Real-Time Meeting Translation for macOS

Status: planned
Created: 2026-07-08
Source spec: `/Users/hieplq1.rpm/AI/baka-trans/note.md`
Mode: `hi-plan --full`
Blocked by: none
Blocks: none

## Objective

Build a lightweight macOS desktop app that captures Microsoft Teams meeting audio, translates speech in near real time, plays the translated audio privately to the user's headphones, and shows a live source/translated transcript.

## Context Scan

- `plans/` did not exist before this plan, so there are no active cross-plan dependencies.
- `docs/development-rules.md` is absent, so no additional project-local development rules were found.
- The repository currently contains only `note.md`, so the first implementation phase must scaffold the app.
- `mind_mcp` was reachable, but its graph-RAG query failed with an internal backend error. `graph_mcp` semantic results pointed to an unrelated indexed project, so local `note.md` is the project source of truth.

## Official API Notes

- OpenAI Realtime Translation supports a dedicated `/v1/realtime/translations` endpoint and `gpt-realtime-translate` for interpreter-style live speech translation.
- OpenAI Realtime Transcription recommends `gpt-realtime-whisper` for low-latency live transcript deltas, while standard Audio API transcription models are better for non-streaming file or chunk workflows.
- OpenAI Text-to-Speech docs recommend `gpt-4o-mini-tts` for intelligent realtime TTS, with `tts-1` and `tts-1-hd` as older alternatives.
- Current OpenAI model docs list GPT-5.5 as the flagship model and GPT-5.4 mini/nano as lower-latency/lower-cost choices. For this app, the main path should avoid a text model in the hot loop when the dedicated translation session can produce translated audio directly.

References:

- https://developers.openai.com/api/docs/guides/realtime-translation
- https://developers.openai.com/api/docs/guides/realtime-transcription
- https://developers.openai.com/api/docs/guides/realtime-websocket
- https://developers.openai.com/api/docs/guides/text-to-speech
- https://developers.openai.com/api/docs/models

## Scope Challenge

Question 1: Should the MVP use dedicated Realtime Translation or a manual STT -> text translation -> TTS chain?
Decision: Use dedicated Realtime Translation as the primary path. Keep a chunked fallback path because it is easier to debug and can unblock MVP demos if streaming translation needs extra work.

Question 2: Should the app attempt to capture Teams process audio directly?
Decision: No for MVP. Use BlackHole 2ch or another virtual audio device. Direct per-app capture on macOS is more invasive and would expand permissions, driver, and support burden.

Question 3: Should the API key live in frontend state, environment variables, or secure storage?
Decision: Use macOS Keychain through the Rust backend. Environment variables can be accepted for development only. The frontend must never persist or directly expose a standard API key.

## Architecture

Recommended MVP architecture:

```text
Teams audio
  -> BlackHole 2ch / selected input device
  -> Rust audio capture service
  -> PCM16 mono 24 kHz stream
  -> OpenAI Realtime Translation WebSocket session
  -> translated audio + source/target transcript events
  -> Rust playback queue
  -> selected headphones/output device
  -> Tauri events to React transcript UI
```

Fallback architecture:

```text
Captured audio chunks
  -> Audio API transcription
  -> Responses API text translation
  -> Audio API speech generation
  -> playback queue
```

Module boundaries:

- Frontend: React TypeScript UI, session controls, device selectors, language/style settings, live transcript panel, export actions.
- Tauri commands/events: typed bridge between React and Rust.
- Audio: device enumeration, capture stream, resampling, chunking, output playback.
- AI pipeline: realtime translation client, fallback STT/translation/TTS client, retry/backoff, API error normalization.
- Session state: lifecycle, status transitions, transcript history, pause/resume, cleanup.
- Security: Keychain-backed API key storage, no raw audio persistence by default.

## Proposed Stack

- Tauri v2, React, TypeScript, Vite.
- Rust async runtime: `tokio`.
- Audio capture/output: `cpal`; add `rubato` or `dasp` for resampling if needed.
- Realtime connection: `tokio-tungstenite` with TLS.
- Secure key storage: `keyring`.
- Serialization/errors: `serde`, `thiserror`, `tracing`.
- Frontend state: React state or a small store after UI complexity is visible; avoid large state frameworks in the first scaffold.

## Data Model

Core session config:

- source language: `auto`, `en`, `ja`, `vi`
- target language: `en`, `ja`, `vi`
- translation style: `literal`, `natural`, `technical_meeting_safe`
- translation input device ID
- translated audio output device ID
- optional original-audio monitor output device ID
- original-audio monitor enabled flag
- voice ID
- fallback mode enabled

Transcript item:

- stable ID
- timestamp
- source text
- translated text
- status: `partial`, `final`, `error`
- latency metrics when available

Session status:

- `idle`
- `starting`
- `listening`
- `translating`
- `speaking`
- `paused`
- `stopping`
- `error`

## Phase Plan

1. Foundation and project scaffold
   - Create the Tauri React TypeScript app.
   - Add Rust module layout, typed command/event contracts, and development config.
   - Verify app boots locally.
   - See `phase-01-project-foundation.md`.

2. Audio devices and routing
   - Enumerate input/output devices.
   - Capture from BlackHole or selected input.
   - Play a local test tone/sample to selected output.
   - Add an in-app setup checklist for Teams + BlackHole routing.
   - See `phase-02-audio-devices-routing.md`.

3. Realtime translation pipeline
   - Implement OpenAI Realtime Translation WebSocket client.
   - Convert captured audio into required PCM chunks.
   - Receive translated audio and transcript deltas.
   - Add fallback chunked STT/translation/TTS path behind a config flag.
   - See `phase-03-realtime-translation-pipeline.md`.

4. Product UI and session experience
   - Build controls, live status, transcript history, export, errors, and pause/resume.
   - Connect all UI actions to backend commands and event streams.
   - See `phase-04-ui-session-transcripts.md`.

5. Hardening, privacy, packaging
   - Add long-session stability testing, reconnect/backoff, device disappearance handling, secure key storage, and macOS packaging.
   - See `phase-05-hardening-packaging.md`.

6. Advanced audio routing profile
   - Add explicit routing controls for meeting source input, translated output, and optional original-audio monitoring.
   - Support the meeting scenario where Teams audio is captured from BlackHole, translated audio is played to headphones, and the original meeting audio is still heard through Mac speakers or another selected monitor output.
   - Persist routing choices and validate feedback-risk combinations before session start.
   - See `phase-06-audio-routing-profile.md`.

## Acceptance Criteria

- The user can select source/target languages, input device, and output device.
- The user can choose the meeting audio input source independently from translated output.
- The user can optionally monitor original meeting audio on a separate output device such as Mac speakers while translated audio plays to headphones.
- The app can capture Teams-routed audio through BlackHole 2ch.
- The app streams audio to the OpenAI realtime translation path and receives translated output.
- The translated audio plays only to the selected headphones/output device.
- The UI shows live source and translated transcripts with session status.
- Start, Stop, Pause, and Resume work without leaving orphaned audio streams or sockets.
- Transcript export works as plain text and Markdown.
- API key is stored in Keychain or loaded from development environment variables only.
- Raw audio is not stored unless a future explicit setting is added.

## Verification Strategy

- Unit tests for config validation, transcript reducer/state transitions, event parsing, and retry policy.
- Rust integration checks for device enumeration and audio format conversion.
- Manual macOS validation with BlackHole 2ch and Teams audio routing.
- Realtime API smoke test with a short local audio fixture before live meeting tests.
- Two-hour soak test with synthetic or looped audio before packaging.
- Frontend verification at small and normal desktop window sizes.

## Risks

- macOS audio routing depends on user setup in Teams and BlackHole.
- Realtime translated audio event format and output codec must be verified during implementation against current API behavior.
- Bluetooth output latency may make the 1-3 second target harder.
- Simultaneous capture/playback can create feedback if the selected input receives output audio.
- Original-audio monitoring can duplicate Teams audio or create echo unless the selected input, translated output, and monitor output are validated together.
- API quota/network errors can interrupt meetings unless retry and fallback paths are explicit.

## Out of Scope for MVP

- Virtual microphone rebroadcast into Teams.
- Speaker diarization.
- Meeting summaries or searchable archives.
- Windows support.
- Native Teams integration.
- Local/offline speech models.

## Cook Command

After reviewing this plan, implementation can start with:

```bash
$hi-brew /Users/hieplq1.rpm/AI/baka-trans/plans/realtime-meeting-translation-macos/plan.md --full
```
