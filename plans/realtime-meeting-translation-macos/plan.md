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
- 2026-07-08 Phase 07 update: `mind_mcp` and `graph_mcp` returned unrelated indexed corpus results for the manual sentence-boundary fallback request. Local plan files, `src-tauri/src/ai.rs`, `src-tauri/src/session.rs`, and `src/App.tsx` are the implementation source of truth.
- 2026-07-08 Source signal update: `mind_mcp` had no matching project collection and `graph_mcp` returned unrelated indexed data. Local code shows `src-tauri/src/audio.rs` already emits `audio-level` with `rms` and `peak`, while `src/App.tsx` renders a basic meter. The new scope should refine this into an explicit live source-signal indicator rather than duplicating capture logic.
- 2026-07-09 LLM summary-agent update: `mind_mcp` has no project paragraph sources. `graph_mcp` found current `bakatrans` API-key and settings UI symbols, plus unrelated historical examples for OpenAI-compatible/Ollama calls. Local source shows translation currently has a single OpenAI key path (`src-tauri/src/security.rs`, `src-tauri/src/ai.rs`, `src/api.ts`) and settings UI in `src/App.tsx`; the summary feature should split translation credentials from reusable LLM provider profiles instead of overloading the existing key panel.
- 2026-07-09 OpenAI realtime language-selector update: `mind_mcp` default collection was missing and the project collection returned empty passages. `graph_mcp` found the current selector in `src/App.tsx` and `targetLanguages = languages.filter(...)`; local source confirms both the React `Language` union in `src/types.ts` and Rust `Language` enum in `src-tauri/src/models.rs` are still limited to `auto`, `en`, `ja`, and `vi`. The new scope should update the existing realtime translation plan rather than create a separate plan.

## Official API Notes

- OpenAI Realtime Translation supports a dedicated `/v1/realtime/translations` endpoint and `gpt-realtime-translate` for interpreter-style live speech translation.
- OpenAI's Realtime Translation cookbook says `gpt-realtime-translate` dynamically detects more than 70 input languages and supports 13 target output languages. Target output languages are Spanish, Portuguese, French, Japanese, Russian, Chinese, German, Korean, Hindi, Indonesian, Vietnamese, Italian, and English.
- OpenAI Realtime WebSocket audio uses `input_audio_buffer.append` for base64 audio chunks. `input_audio_buffer.commit` forces the current input buffer into a conversation item and clears it; with server VAD the server normally commits automatically, while disabling VAD requires manual commit and response creation.
- OpenAI Realtime Transcription recommends `gpt-realtime-whisper` for low-latency live transcript deltas, while standard Audio API transcription models are better for non-streaming file or chunk workflows.
- OpenAI Text-to-Speech docs recommend `gpt-4o-mini-tts` for intelligent realtime TTS, with `tts-1` and `tts-1-hd` as older alternatives.
- Current OpenAI model docs list GPT-5.5 as the flagship model and GPT-5.4 mini/nano as lower-latency/lower-cost choices. For this app, the main path should avoid a text model in the hot loop when the dedicated translation session can produce translated audio directly.
- Google ADK's Ollama model guide says ADK integrates with Ollama-hosted models through LiteLLM, recommends the `ollama_chat` provider for Ollama agents, and notes the alternate OpenAI-provider path with `OPENAI_API_BASE=http://localhost:11434/v1` plus any non-empty API key. This matters for the summary agent because provider config must support both direct OpenAI-compatible chat completion and ADK/LiteLLM-style model IDs.

References:

- https://developers.openai.com/api/docs/guides/realtime-translation
- https://developers.openai.com/api/docs/guides/realtime-transcription
- https://developers.openai.com/api/reference/resources/realtime/client-events/
- https://developers.openai.com/api/docs/guides/realtime-websocket
- https://developers.openai.com/api/docs/guides/text-to-speech
- https://developers.openai.com/api/docs/models
- https://developers.openai.com/cookbook/examples/voice_solutions/realtime_translation_guide
- https://adk.dev/agents/models/ollama/

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

Meeting summary agent architecture:

```text
Transcript final items
  -> session transcript store
  -> MeetingSummaryAgent trigger (manual, end-of-session, or interval)
  -> transcript chunking + rolling meeting memory
  -> agent steps:
       summarize agenda/context
       extract decisions
       extract action items and owners
       identify risks, blockers, deadlines, and facts to remember
       produce concise meeting notes
  -> summary state + Tauri events
  -> React summary panel and transcript export
```

Module boundaries:

- Frontend: React TypeScript UI, session controls, device selectors, language/style settings, live transcript panel, export actions.
- Tauri commands/events: typed bridge between React and Rust.
- Audio: device enumeration, capture stream, resampling, chunking, output playback.
- AI pipeline: realtime translation client, fallback STT/translation/TTS client, summary-agent model client, retry/backoff, API error normalization.
- Session state: lifecycle, status transitions, transcript history, pause/resume, cleanup.
- Security/config: Keychain-backed secret storage for translation and LLM profiles, non-secret provider profile persistence, no raw audio persistence by default.

## Proposed Stack

- Tauri v2, React, TypeScript, Vite.
- Rust async runtime: `tokio`.
- Audio capture/output: `cpal`; add `rubato` or `dasp` for resampling if needed.
- Realtime connection: `tokio-tungstenite` with TLS.
- Summary LLM client: OpenAI-compatible chat-completions HTTP client in Rust first; keep an ADK sidecar adapter optional if richer ADK workflows are required after the profile/config work lands.
- Secure key storage: `keyring`.
- Serialization/errors: `serde`, `thiserror`, `tracing`.
- Frontend state: React state or a small store after UI complexity is visible; avoid large state frameworks in the first scaffold.

## Data Model

Core session config:

- source language: `auto` plus OpenAI Realtime Translation input-language codes for user labeling, fallback paths, and same-language warnings. Dedicated realtime translation should continue relying on model-side source-language detection.
- target language: one of OpenAI Realtime Translation's 13 output-language codes: `es`, `pt`, `fr`, `ja`, `ru`, `zh`, `de`, `ko`, `hi`, `id`, `vi`, `it`, `en`
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

LLM provider profile:

- stable provider ID
- display name
- provider kind: `openai`, `openai_compatible`, `ollama`, `adk_litellm`
- model ID
- base URL
- optional API key secret reference
- timeout and max output tokens
- temperature
- enabled flag
- last test result and fingerprint

Meeting summary agent config:

- selected LLM provider profile ID
- summary trigger: `manual`, `session_end`, optional interval
- transcript scope: source text, translated text, or both
- output language
- enabled sections: summary, decisions, action items, blockers, follow-ups, facts to remember
- maximum transcript chars per run
- rolling memory enabled flag

Meeting summary result:

- stable ID
- session timestamp range
- short summary
- decisions
- action items with optional owner and due date
- blockers/risks
- important points to remember
- source transcript item IDs used
- model/provider metadata
- status: `pending`, `complete`, `error`

Source audio signal state:

- selected input device ID
- latest peak and RMS
- last signal event timestamp
- stream state: `waiting`, `receiving`, `silent`, `stale`, `error`
- optional chunk counter or sequence number for diagnostics

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

7. Manual utterance boundary fallback
   - Add a visible session button and keyboard shortcut that lets the user force the current audio buffer to be translated when the remote speaker talks continuously and automatic turn detection does not close the sentence.
   - Route the control through the Rust session state to the realtime WebSocket writer as a manual boundary/commit command.
   - Track committed/ignored/error states so the fallback is usable under meeting pressure.
   - See `phase-07-manual-utterance-boundary.md`.

8. Source audio signal indicator
   - Promote the existing `audio-level` event into a clear "source audio is arriving" signal in the Audio routing area.
   - Track event freshness so the UI can distinguish no data, silent data, active signal, stale stream, and capture error.
   - Keep the backend change limited to metadata only if the current `rms` and `peak` event is insufficient.
   - See `phase-08-source-audio-signal-indicator.md`.

9. LLM configuration and meeting-summary agent
   - Redesign the current OpenAI key panel into clear translation and summary-agent configuration areas.
   - Add reusable OpenAI-compatible LLM provider profiles, including Ollama/ADK-friendly settings.
   - Implement a meeting summary agent that operates over transcript state with explicit steps for summaries, decisions, action items, blockers, and points to remember.
   - See `phase-09-llm-config-summary-agent.md`.

10. OpenAI Realtime supported language selector
   - Replace the current `auto/en/ja/vi` language option set with OpenAI Realtime Translation-supported language metadata.
   - Keep `Auto` available only for source language. Limit target language to the 13 supported output languages.
   - Update the React options, shared TypeScript types, Rust deserialization/validation, and tests so unsupported target codes cannot reach `/v1/realtime/translations`.
   - See `phase-10-openai-realtime-language-selector.md`.

## Acceptance Criteria

- The user can select source/target languages, input device, and output device.
- The target-language selector includes all 13 OpenAI Realtime Translation output languages and excludes `Auto`.
- The source-language selector includes `Auto` and OpenAI Realtime Translation input-language options without pretending the dedicated realtime endpoint requires a manual source-language parameter.
- The user can choose the meeting audio input source independently from translated output.
- The user can optionally monitor original meeting audio on a separate output device such as Mac speakers while translated audio plays to headphones.
- The app can capture Teams-routed audio through BlackHole 2ch.
- The app streams audio to the OpenAI realtime translation path and receives translated output.
- The translated audio plays only to the selected headphones/output device.
- The UI shows live source and translated transcripts with session status.
- Start, Stop, Pause, and Resume work without leaving orphaned audio streams or sockets.
- During an active session, the user can manually force the current spoken segment to translate when automatic sentence/turn detection stalls.
- The UI clearly indicates when the selected meeting source is actively delivering audio data into the app, even before translation output appears.
- The source signal indicator distinguishes a healthy but silent stream from a stream that has stopped sending events.
- Transcript export works as plain text and Markdown.
- Translation API key is stored in Keychain or loaded from development environment variables only.
- The user can add, test, select, edit, and disable LLM provider profiles for meeting summaries without changing the realtime translation key.
- LLM profiles support OpenAI and OpenAI-compatible endpoints, including local Ollama through either `http://localhost:11434/v1` OpenAI-provider mode or an ADK/LiteLLM-compatible model naming path.
- The user can run a meeting-summary agent over the current transcript and receive summary, decisions, action items, blockers, and points to remember.
- The summary agent exposes progress/error state and never blocks live audio capture, translation, playback, or manual boundary controls.
- Raw audio is not stored unless a future explicit setting is added.

## Verification Strategy

- Unit tests for config validation, transcript reducer/state transitions, event parsing, and retry policy.
- Unit tests for manual boundary command routing, empty-buffer handling, debounce behavior, and UI disabled states.
- Unit tests for source signal state transitions: waiting, receiving, silent, stale, and error.
- Unit tests for LLM provider config validation, OpenAI-compatible request building, Ollama base URL normalization, summary-agent output parsing, and transcript scope selection.
- Rust integration checks for device enumeration and audio format conversion.
- Manual macOS validation with BlackHole 2ch and Teams audio routing.
- Manual validation that the source indicator changes when Teams/fixture audio is routed into BlackHole, goes silent when audio stops, and turns stale when capture events stop.
- Realtime API smoke test with a short local audio fixture before live meeting tests.
- Summary-agent smoke tests with one OpenAI profile and one local/Ollama-compatible profile when available.
- Two-hour soak test with synthetic or looped audio before packaging.
- Frontend verification at small and normal desktop window sizes.

## Risks

- macOS audio routing depends on user setup in Teams and BlackHole.
- Realtime translated audio event format and output codec must be verified during implementation against current API behavior.
- Bluetooth output latency may make the 1-3 second target harder.
- Simultaneous capture/playback can create feedback if the selected input receives output audio.
- Original-audio monitoring can duplicate Teams audio or create echo unless the selected input, translated output, and monitor output are validated together.
- API quota/network errors can interrupt meetings unless retry and fallback paths are explicit.
- Manual boundary commits can produce duplicate, truncated, or empty translations if they race with server VAD or are pressed too often.
- Realtime translation may require additional response triggering depending on selected turn-detection mode; this must be verified against the active API event contract.
- A low level meter alone can mislead users during silence, so freshness and selected-device matching must be visible enough to prove the audio path is connected.
- Provider config can become confusing if translation and summary settings are mixed together; the UI should name them as separate concerns and allow explicit sharing only when selected by the user.
- Ollama/OpenAI-compatible providers vary in JSON output reliability and tool-call behavior; summary-agent parsing must tolerate fenced text, invalid JSON, and retries before surfacing an error.
- Bundling a full ADK runtime inside a desktop app may add packaging complexity. Keep the agent contract independent of ADK and treat an ADK sidecar as an adapter, not the only execution path.

## Out of Scope for MVP

- Virtual microphone rebroadcast into Teams.
- Speaker diarization.
- Searchable meeting archives.
- Windows support.
- Native Teams integration.
- Local/offline speech models for realtime translation.

## Cook Command

After reviewing this plan, implementation can start with:

```bash
$hi-brew /Users/hieplq1.rpm/AI/baka-trans/plans/realtime-meeting-translation-macos/plan.md --full
```
