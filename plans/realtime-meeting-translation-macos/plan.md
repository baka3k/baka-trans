# Real-Time Meeting Translation for macOS

Status: planned
Created: 2026-07-08
Source spec: `/Users/hieplq1.rpm/AI/baka-trans/note.md`
Mode: `hi-plan --full`
Blocked by: none
Blocks: `plans/transparent-ocr-overlay-macos`, `plans/windows-support`, `plans/260712-2234-application-ui-modernization`

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
- 2026-07-10 Google migration update: `mind_mcp`, `graph_mcp`, and `serena` were not exposed in this session, so the search chain fast-failed to local `rg`/file reads. Local source now has a concrete OpenAI Realtime implementation in `src-tauri/src/ai.rs`, OpenAI-specific translation secrets in `src-tauri/src/security.rs`, OpenAI target-language assumptions in `src/languages.ts` and `src-tauri/src/models.rs`, and OpenAI-compatible summary profiles in `src-tauri/src/llm.rs`. The Google migration should update this active plan, not create a separate feature plan.
- 2026-07-10 Conversation UI redesign update: `mind_mcp`, `graph_mcp`, and `serena` were not exposed in this session, so the search chain fast-failed to local `rg`/file reads. Local source shows `src/App.tsx` already renders status, routing, audio levels, summary controls, and a two-column transcript table. The new scope should redesign that transcript area into a chat-style conversation feed while reusing existing transcript, audio-level, and session events.
- 2026-07-10 Transparent OCR overlay update: user requested a "xuyen thau" mode that opens a semi-transparent movable/resizable window and translates text under it. `graph_mcp` found the current Tauri/React bridge and transcript UI symbols, while local source confirms translation provider settings, Tauri commands, and app state are already centralized. This should be tracked as `plans/transparent-ocr-overlay-macos` and implemented as a separate screen-capture/OCR/text-translation runtime that depends on this app foundation rather than extending the audio session runtime.
- 2026-07-12 Application UI modernization update: the Phase 16 conversation feed, live status rail, source-signal derivation, and scrolled-away translation affordance are present in current code even though the phase document still says planned. The broader Fluent 2 shell, responsive navigation, form hierarchy, accessibility, and overlay modernization work is tracked in `plans/260712-2234-application-ui-modernization`. That plan treats Phase 16 as a satisfied functional baseline and does not change translation or session behavior.

## Official API Notes

- OpenAI Realtime Translation supports a dedicated `/v1/realtime/translations` endpoint and `gpt-realtime-translate` for interpreter-style live speech translation.
- OpenAI's Realtime Translation cookbook says `gpt-realtime-translate` dynamically detects more than 70 input languages and supports 13 target output languages. Target output languages are Spanish, Portuguese, French, Japanese, Russian, Chinese, German, Korean, Hindi, Indonesian, Vietnamese, Italian, and English.
- OpenAI Realtime WebSocket audio uses `input_audio_buffer.append` for base64 audio chunks. `input_audio_buffer.commit` forces the current input buffer into a conversation item and clears it; with server VAD the server normally commits automatically, while disabling VAD requires manual commit and response creation.
- OpenAI Realtime Transcription recommends `gpt-realtime-whisper` for low-latency live transcript deltas, while standard Audio API transcription models are better for non-streaming file or chunk workflows.
- OpenAI Text-to-Speech docs recommend `gpt-4o-mini-tts` for intelligent realtime TTS, with `tts-1` and `tts-1-hd` as older alternatives.
- Current OpenAI model docs list GPT-5.5 as the flagship model and GPT-5.4 mini/nano as lower-latency/lower-cost choices. For this app, the main path should avoid a text model in the hot loop when the dedicated translation session can produce translated audio directly.
- Google ADK's Ollama model guide says ADK integrates with Ollama-hosted models through LiteLLM, recommends the `ollama_chat` provider for Ollama agents, and notes the alternate OpenAI-provider path with `OPENAI_API_BASE=http://localhost:11434/v1` plus any non-empty API key. This matters for the summary agent because provider config must support both direct OpenAI-compatible chat completion and ADK/LiteLLM-style model IDs.
- Google Gemini Live API is currently Preview and supports low-latency voice/vision sessions over stateful WebSockets. Audio input is raw 16-bit PCM, 16 kHz, little-endian; audio output is raw 16-bit PCM, 24 kHz, little-endian.
- Google Live Translation uses `gemini-3.5-live-translate-preview` and behaves as an interpreter pipeline rather than a tool-capable Live Agent. It supports audio-only input, `responseModalities: ["AUDIO"]`, `inputAudioTranscription`, `outputAudioTranscription`, and `translationConfig.targetLanguageCode`.
- Google Live Translation sends audio via `realtimeInput.audio.data` as base64 plus `mimeType: "audio/pcm;rate=16000"`. Google recommends 100 ms chunks for translation latency.
- Google Live Translation output is carried in `serverContent.modelTurn.parts[].inlineData.data`, with optional input/output transcript objects under `serverContent.inputTranscription` and `serverContent.outputTranscription`.
- `translationConfig.echoTargetLanguage` controls whether speech already in the target language is echoed or suppressed. Default is false; this should be exposed because meeting participants may switch to the target language.
- Google Live Translation supports 70+ languages with BCP-47 codes. This is wider than the current OpenAI target-language list, but includes regional variants such as `pt-BR`, `pt-PT`, `zh-Hans`, and `zh-Hant`; the shared language model must stop assuming one canonical `zh`/`pt` target set.
- Raw WebSocket authentication can use `?key=GEMINI_API_KEY` on the v1beta endpoint. Client-to-server production connections should use v1alpha ephemeral tokens instead of standard API keys; ephemeral tokens can be constrained to a model/config and are short-lived.
- Live API audio-only sessions are limited to 15 minutes without session-management techniques, and individual connections are around 10 minutes. Use GoAway handling, session resumption handles, and context window compression for long meetings.
- Live API cost compounds with session context. Transcriptions add text-output token costs; context-window compression and short retained windows are important optimization controls for meeting-length sessions.
- Gemini audio understanding supports speaker diarization for uploaded/batched audio through structured outputs, including speaker labels and timestamps, but this is not the same as the low-latency Live Translation stream.
- Gemini Live API documentation lists audio transcriptions, Live Translation, and Voice Activity Detection. Treat VAD as "someone is speaking" turn detection, not speaker identity or speaker diarization.
- Google Cloud Speech-to-Text supports speaker diarization through `diarizationConfig` with expected speaker counts and returns speaker-tagged words/segments. This is the more explicit diarization API when reliable speaker labels matter.
- The diarization path should run as a sidecar over bounded audio windows or finalized transcript/audio segments. It must not block the hot Live Translation audio path, and it should reconcile speaker labels into transcript state asynchronously.

References:

- https://developers.openai.com/api/docs/guides/realtime-translation
- https://developers.openai.com/api/docs/guides/realtime-transcription
- https://developers.openai.com/api/reference/resources/realtime/client-events/
- https://developers.openai.com/api/docs/guides/realtime-websocket
- https://developers.openai.com/api/docs/guides/text-to-speech
- https://developers.openai.com/api/docs/models
- https://developers.openai.com/cookbook/examples/voice_solutions/realtime_translation_guide
- https://adk.dev/agents/models/ollama/
- https://ai.google.dev/gemini-api/docs/live-api
- https://ai.google.dev/gemini-api/docs/live-api/live-translate
- https://ai.google.dev/gemini-api/docs/live-api/get-started-websocket
- https://ai.google.dev/gemini-api/docs/live-api/session-management
- https://ai.google.dev/gemini-api/docs/live-api/ephemeral-tokens
- https://ai.google.dev/gemini-api/docs/live-api/best-practices
- https://ai.google.dev/gemini-api/docs/audio
- https://docs.cloud.google.com/speech-to-text/docs/multiple-voices

## Scope Challenge

Question 1: Should the MVP use dedicated Realtime Translation or a manual STT -> text translation -> TTS chain?
Decision: Use dedicated Realtime Translation as the primary path. Keep a chunked fallback path because it is easier to debug and can unblock MVP demos if streaming translation needs extra work.

Question 2: Should the app attempt to capture Teams process audio directly?
Decision: No for MVP. Use BlackHole 2ch or another virtual audio device. Direct per-app capture on macOS is more invasive and would expand permissions, driver, and support burden.

Question 3: Should the API key live in frontend state, environment variables, or secure storage?
Decision: Use macOS Keychain through the Rust backend. Environment variables can be accepted for development only. The frontend must never persist or directly expose a standard API key.

Question 4: Should the OpenAI to Google migration be a direct replacement or side-by-side provider cutover?
Decision: Use side-by-side provider abstraction first, then make Google the default after parity tests pass. Direct replacement is faster but risky because Google has different audio rate, message schema, session duration, language metadata, and manual-boundary behavior.

Question 5: Should the desktop app connect to Google from Rust or directly from the React frontend?
Decision: Keep Rust server-to-server WebSocket as the first production path because the app already captures audio in Rust and stores secrets in Keychain. Add an optional ephemeral-token/client-direct path only if latency measurements prove the Rust path is the bottleneck.

Question 6: Should Google Live Agent replace the translation pipeline?
Decision: No for live meeting translation. Use Live Translation for the hot path because it is optimized for continuous interpreter-style translation. Use Gemini text/structured-output APIs separately for meeting summaries and action items.

Question 7: Should speaker diarization be part of the realtime translation hot path?
Decision: No. Keep Gemini Live Translation focused on low-latency translation and audio playback. Add speaker diarization as an optional sidecar/post-processing pipeline that consumes bounded captured-audio windows or retained transcript-aligned audio segments, then annotates transcript items when speaker labels arrive.

Question 8: Should diarization use Gemini audio understanding or Cloud Speech-to-Text?
Decision: Support both behind a `DiarizationProvider` abstraction. Prefer Cloud Speech-to-Text for explicit diarization controls and streaming/batch recognition semantics. Use Gemini audio structured output for post-meeting enrichment, summaries, emotion/timestamp experiments, or when the user already uses Gemini credentials and accepts less deterministic speaker timing.

## Google Migration Options

Option A: direct OpenAI replacement in `src-tauri/src/ai.rs`.

- Replace the OpenAI URL, token minting, setup payload, audio append events, event parser, and language validation in place.
- Pros: smallest file count and fastest first experiment.
- Cons: high regression risk; no easy fallback if Google Preview behavior differs during a meeting; OpenAI-specific names remain misleading.
- Use only for a throwaway spike.

Option B: provider abstraction with side-by-side OpenAI and Google backends.

- Introduce `TranslationProvider` and provider-specific runtime modules, for example `ai/openai_realtime.rs` and `ai/google_live.rs`.
- Keep the session/audio/playback UI mostly stable while routing through a provider selected in settings.
- Pros: safest migration, measurable parity, easy rollback, cleaner secrets and language metadata.
- Cons: more initial refactor than Option A.
- Recommendation: choose this path.

Option C: frontend direct WebSocket with backend-minted Google ephemeral tokens.

- Rust mints constrained v1alpha ephemeral tokens; React opens the Live API WebSocket directly.
- Pros: lowest network hop if audio capture later moves to browser APIs or web deployment.
- Cons: current audio capture is Rust/cpal, so this would add cross-process audio forwarding and more concurrency surface. Ephemeral-token constraints also add setup complexity.
- Use after Option B only if latency profiling demands it.

Option D: use Google Live Agent instead of Live Translation.

- Use a general Live model with system instructions/tools and ask it to translate.
- Pros: could combine translation with richer assistant behavior.
- Cons: not appropriate for the core interpreter loop; Live Translation explicitly has simpler config and audio-only latency guarantees.
- Do not use for the live translation hot path.

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

Speaker diarization sidecar architecture:

```text
Captured source audio windows
  -> bounded in-memory rolling buffer or explicit temporary encrypted segment file
  -> DiarizationService trigger (interval, manual, or session_end)
  -> provider adapter:
       Cloud Speech-to-Text diarization
       or Gemini audio structured-output diarization
  -> speaker segments with time ranges and confidence/provider metadata
  -> transcript alignment by timestamp/window overlap
  -> Tauri speaker-label events
  -> React transcript speaker badges + export labels
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
- optional speaker label and confidence when diarization later enriches the item

Conversation display item:

- stable transcript item ID
- speaker display label, using a neutral fallback until diarization exists
- source speech line
- translated speech line directly below source speech
- display status: listening, translating, final, or error
- timestamp and optional latency

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
- summary prompt preset: `balanced`, `professional`, `gentle`, `detailed`, `timeline`, or `custom`
- custom system prompt text when the `custom` preset is selected
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

Speaker diarization config:

- enabled flag
- provider: `google_cloud_speech_to_text`, `gemini_audio_analysis`
- trigger: `interval`, `manual`, `session_end`
- minimum and maximum expected speaker count
- window duration and overlap duration
- temporary audio retention policy: memory-only, encrypted temp file, delete-on-complete
- transcript alignment mode: source transcript only first, translated text inherited after alignment
- confidence threshold for applying labels

Speaker segment:

- stable ID
- speaker label: `Speaker 1`, `Speaker 2`, or user-renamed display label
- start timestamp and end timestamp relative to session start
- source transcript item IDs matched
- optional word-level ranges when the provider returns them
- confidence/provider metadata
- status: `pending`, `aligned`, `needs_review`, `error`

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

11. Translation provider abstraction and Google credentials
   - Rename OpenAI-specific translation key storage and status surfaces into provider-aware translation credentials.
   - Add a `TranslationProvider` model with `openai_realtime` and `google_live_translate` values.
   - Keep OpenAI available during migration, but make the config capable of selecting Google.
   - See `phase-11-translation-provider-abstraction-google-credentials.md`.

12. Gemini Live Translation pipeline
   - Add a Google Live Translation WebSocket client using `gemini-3.5-live-translate-preview`.
   - Convert captured audio to 16 kHz mono PCM16 for Google input and preserve 24 kHz playback for Google output.
   - Parse Google `serverContent` audio/transcript events into the existing transcript and playback queues.
   - See `phase-12-gemini-live-translation-pipeline.md`.

13. Google Live optimization and meeting-session hardening
   - Add 100 ms chunking, GoAway handling, reconnect/session-resumption strategy, context-window compression settings, and cost-aware transcription toggles.
   - Expose `echoTargetLanguage`, Google session limits, and Google-specific diagnostics in the UI.
   - Replace manual-boundary assumptions that are specific to OpenAI `session.close`.
   - See `phase-13-google-live-optimization-session-hardening.md`.

14. Gemini summary provider and final OpenAI retirement
   - Add a Google Gemini summary provider profile path for meeting notes.
   - Migrate labels, defaults, docs, tests, and error messages from OpenAI-first to Google-first while keeping OpenAI-compatible profiles optional.
   - Remove or deprecate OpenAI-only translation paths after Google parity is verified.
   - See `phase-14-gemini-summary-provider-openai-retirement.md`.

15. Speaker diarization sidecar and transcript attribution
   - Add an optional diarization pipeline that does not block Gemini Live Translation, playback, or transcript updates.
   - Store only bounded audio windows needed for diarization and delete them according to an explicit retention policy.
   - Support Cloud Speech-to-Text diarization first for reliable speaker tagging, with a Gemini audio structured-output adapter for post-meeting enrichment.
   - Align speaker segments back onto existing transcript items and show speaker labels in transcript/export/summary inputs.
   - See `phase-15-speaker-diarization-sidecar.md`.

16. Conversation translation UI redesign
   - Replace the current two-column transcript table with a chat-style conversation feed.
   - Show each spoken line as one utterance card with speaker context, source speech, and translation immediately underneath.
   - Promote source signal, translation progress, and translated playback into a compact live status rail above the feed.
   - Keep speaker attribution neutral until phase 15 supplies real diarization labels.
   - See `phase-16-conversation-translation-ui-redesign.md`.

17. Summary Agent prompt presets and custom instructions
   - Add selectable balanced, professional, gentle, detailed, and timeline-oriented summary styles.
   - Add a custom system-prompt option with a dedicated editor in Summary Agent settings.
   - Compose the selected instructions with immutable structured-output and transcript-grounding rules in Rust.
   - Keep preset/custom selection compatible with every existing LLM provider profile.
   - See `phase-17-summary-agent-prompt-presets-custom.md`.

18. Application-wide UI modernization
   - Adopt one official Fluent 2 component, token, icon, accessibility, and responsive system across the main window and both overlay routes.
   - Replace the settings-first compact layout with a live-first shell and responsive context panel or drawer.
   - Preserve all existing commands, events, data contracts, validation, transcript behavior, overlay behavior, and provider logic.
   - See `../260712-2234-application-ui-modernization/plan.md`.

## Acceptance Criteria

- The user can select source/target languages, input device, and output device.
- The target-language selector includes all 13 OpenAI Realtime Translation output languages and excludes `Auto`.
- The source-language selector includes `Auto` and OpenAI Realtime Translation input-language options without pretending the dedicated realtime endpoint requires a manual source-language parameter.
- When Google is selected, the target-language selector uses Google Live Translation BCP-47 metadata, including regional target codes where Google distinguishes them.
- The app can store and test a Google Gemini translation key separately from legacy OpenAI credentials.
- The app can run a Google Live Translation session and receive translated audio plus input/output transcripts.
- Google input audio is sent as 16 kHz mono PCM16 in 100 ms chunks, and Google output audio is played as 24 kHz mono PCM16.
- Google session handling survives planned connection rotation by reacting to GoAway/session-resumption signals or clearly restarting with minimal dropped audio.
- Google optimization controls cover `echoTargetLanguage`, transcript toggles, context compression, retained-window size, and visible cost/latency tradeoffs.
- The user can choose the meeting audio input source independently from translated output.
- The user can optionally monitor original meeting audio on a separate output device such as Mac speakers while translated audio plays to headphones.
- The app can capture Teams-routed audio through BlackHole 2ch.
- The app streams audio to the OpenAI realtime translation path and receives translated output.
- The translated audio plays only to the selected headphones/output device.
- The UI shows live source and translated transcripts with session status.
- The transcript UI reads as a conversation stream, with each source utterance and its translation grouped together line by line.
- The live translation area clearly indicates incoming audio, translating, translated playback, silence, stale stream, and error states.
- Start, Stop, Pause, and Resume work without leaving orphaned audio streams or sockets.
- During an active session, the user can manually force the current spoken segment to translate when automatic sentence/turn detection stalls.
- The UI clearly indicates when the selected meeting source is actively delivering audio data into the app, even before translation output appears.
- The source signal indicator distinguishes a healthy but silent stream from a stream that has stopped sending events.
- Transcript export works as plain text and Markdown.
- Translation API key is stored in Keychain or loaded from development environment variables only.
- Google production client-direct mode, if added later, uses constrained ephemeral tokens and never exposes a standard Gemini API key in React state.
- The user can add, test, select, edit, and disable LLM provider profiles for meeting summaries without changing the realtime translation key.
- The user can create a Gemini summary provider profile and run the meeting-summary agent without OpenAI.
- LLM profiles support OpenAI and OpenAI-compatible endpoints, including local Ollama through either `http://localhost:11434/v1` OpenAI-provider mode or an ADK/LiteLLM-compatible model naming path.
- The user can run a meeting-summary agent over the current transcript and receive summary, decisions, action items, blockers, and points to remember.
- The user can choose a built-in Summary Agent style for balanced, professional, gentle, detailed, or timeline-oriented meeting notes.
- The user can enter a custom Summary Agent system prompt, preview/edit it in the settings panel, and use it for the next summary run.
- Preset and custom instructions cannot remove the backend's structured JSON contract or transcript-grounding rules.
- The summary agent exposes progress/error state and never blocks live audio capture, translation, playback, or manual boundary controls.
- The user can enable optional speaker diarization without delaying realtime translation or translated audio playback.
- Speaker diarization labels source transcript items asynchronously and clearly marks uncertain labels for review.
- Transcript export and meeting-summary inputs can include speaker labels when diarization is available.
- Diarization audio retention is explicit, bounded, and disabled by default.
- Raw audio is not stored unless the user explicitly enables a bounded feature such as diarization retention, and those buffers are deleted according to policy.

## Verification Strategy

- Unit tests for config validation, transcript reducer/state transitions, event parsing, and retry policy.
- Unit tests for manual boundary command routing, empty-buffer handling, debounce behavior, and UI disabled states.
- Unit tests for source signal state transitions: waiting, receiving, silent, stale, and error.
- Unit tests for conversation display helpers, pending translation placeholders, optional speaker-label fallback, and scrolled-away transcript behavior.
- Unit tests for LLM provider config validation, OpenAI-compatible request building, Ollama base URL normalization, summary-agent output parsing, and transcript scope selection.
- Unit tests for Summary Agent preset resolution, custom-prompt validation, invariant prompt composition, and frontend preset/config helpers.
- Unit tests for provider selection, Google credential lookup, Google setup payload construction, Google audio message construction, and Google event parsing.
- Unit tests for Google language metadata, including `zh-Hans`/`zh-Hant` and `pt-BR`/`pt-PT` handling if exposed in the UI.
- Unit tests for session-rotation decisions, GoAway event handling, context-compression config, and `echoTargetLanguage` config.
- Unit tests for diarization config validation, audio-window retention policy, provider request construction, speaker-segment parsing, and transcript alignment.
- Rust integration checks for device enumeration and audio format conversion.
- Manual macOS validation with BlackHole 2ch and Teams audio routing.
- Manual validation that the source indicator changes when Teams/fixture audio is routed into BlackHole, goes silent when audio stops, and turns stale when capture events stop.
- Realtime API smoke test with a short local audio fixture before live meeting tests.
- Google Live API smoke test with a short 16 kHz fixture and target `vi`/`en` before live meeting tests.
- Summary-agent smoke tests with one OpenAI profile and one local/Ollama-compatible profile when available.
- Diarization smoke test with a short two-speaker fixture and expected speaker-count config.
- Manual validation that diarization runs in the background during translation without introducing audio dropouts or delayed playback.
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
- Chat-style UI can imply speaker identity before diarization exists, so speaker labels must remain neutral until explicit speaker attribution is available.
- Overactive motion or status chips can distract during live meetings, so feedback must stay compact and state-driven.
- Provider config can become confusing if translation and summary settings are mixed together; the UI should name them as separate concerns and allow explicit sharing only when selected by the user.
- Ollama/OpenAI-compatible providers vary in JSON output reliability and tool-call behavior; summary-agent parsing must tolerate fenced text, invalid JSON, and retries before surfacing an error.
- Custom summary instructions can conflict with the structured response contract or request unsupported/invented content; compose them after explicit invariant rules, enforce length/non-empty validation, and keep schema validation authoritative.
- Bundling a full ADK runtime inside a desktop app may add packaging complexity. Keep the agent contract independent of ADK and treat an ADK sidecar as an adapter, not the only execution path.
- Google Live API and ephemeral tokens are Preview, so schema or endpoint changes may require quick adapters.
- Google Live Translation does not support text input or tools in translation mode, so translation style prompts cannot be carried over from an OpenAI-style configurable prompt. Style controls must either be removed, mapped to supported Google config, or moved into a fallback non-live path.
- Google 16 kHz input means the current `REALTIME_SAMPLE_RATE = 24_000` capture constant must become provider-specific; otherwise Google will receive invalid audio.
- Session duration and connection duration limits can interrupt long meetings if GoAway/session-resumption handling is not implemented before real use.
- Enabling both input and output transcriptions improves UI parity but increases text-token billing.
- Speaker diarization requires retaining at least short source-audio windows, which changes the privacy profile. It must be opt-in, bounded, and visibly disclosed.
- Speaker labels can be unstable across rolling windows. Use overlap, label reconciliation, confidence thresholds, and "needs review" states instead of pretending labels are exact.
- Cloud Speech-to-Text diarization may require Google Cloud project/OAuth setup that differs from a simple Gemini API key; settings must make this credential boundary clear.

## Out of Scope for MVP

- Virtual microphone rebroadcast into Teams.
- Guaranteed real-time speaker identity during the translation hot path.
- Searchable meeting archives.
- Windows support.
- Native Teams integration.
- Local/offline speech models for realtime translation.

## Cook Command

After reviewing this plan, implementation can start with:

```bash
$hi-brew /Users/hieplq1.rpm/AI/baka-trans/plans/realtime-meeting-translation-macos/plan.md --full
```
