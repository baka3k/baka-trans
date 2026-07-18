---
title: "Mode Gateway and Local Spoken Translation"
status: in_progress
created: 2026-07-16
updated: 2026-07-18
mode: hi-plan --full
---

# Mode Gateway and Local Spoken Translation

## Overview

Extend the implemented local Japanese-to-Vietnamese pipeline into a spoken translation mode while preserving the current cloud workspace and every existing cloud behavior.

```text
Main application route
  -> choose Cloud API or Local Whisper
     -> Cloud API: current MainApp and Google/OpenAI pipelines
     -> Local Whisper: dedicated local workspace
        -> existing CPAL capture and 16 kHz normalization
        -> existing whisper-rs transcription
        -> existing native Ollama /api/chat client using Gemma
        -> new local platform TTS adapter
        -> existing CPAL output-device and channel-routed playback
```

The cloud branch remains the regression baseline. The local branch replaces only the cloud translation and speech-generation stage. Device discovery, meeting-audio capture, original-audio monitoring, headset selection, left/right routing, meters, lifecycle controls, transcript storage, and export continue to use the current shared runtime.

## Baseline Already Implemented

The original phases 01-04 are present in current source even though this plan remained marked pending:

- `whisper-rs` is installed and local Whisper inference runs through `spawn_blocking`.
- The local segmenter and bounded ordered worker already produce Japanese source text.
- The native Ollama client already posts to `/api/chat` and returns Vietnamese text.
- Stable transcript IDs, revisions, snapshots, local config persistence, and a Local LLM settings panel already exist.
- The session runtime already reuses CPAL capture at 16 kHz and skips cloud credentials for local mode.

The extension begins at Phase 05. It reverses the old text-only non-goal and adds local TTS plus translated-audio routing.

## Scope Challenge Decisions

Three scope questions were resolved so the plan is directly implementable:

1. **When is mode selected?** Show the chooser on every main-window launch. Do not show it for the transparent or Look & Help overlay routes. Do not remember a choice in the first release. A Change mode action returns to the chooser only after the active session has stopped.
2. **What does Cloud API contain?** It opens the current cloud workspace and preserves Google Live, OpenAI Realtime, summaries, overlays, exports, credential handling, audio routing, and current controls. The Google implementation and event contract are not rewritten.
3. **What local TTS is used?** Use platform-native installed voices behind a Rust `LocalTtsEngine` boundary: Windows `Windows.Media.SpeechSynthesis` and macOS `AVSpeechSynthesizer` buffer output. Normalize generated audio to PCM16 mono at 24 kHz and feed the existing CPAL playback runtime. This preserves selected headset and channel routing without adding a second model server. A Piper-style engine can be added later behind the same boundary.

## Design Read

Reading this as: a preserve-first redesign of a desktop realtime translation utility, with a calm operational Fluent 2 language and a focused local workspace rather than a second copy of the cloud dashboard.

- `DESIGN_VARIANCE: 3`: predictable placement and obvious mode boundaries.
- `MOTION_INTENSITY: 2`: only focus, disclosure, and state transitions; honor reduced motion.
- `VISUAL_DENSITY: 6`: compact operational controls with more space reserved for transcript and pipeline state.
- Keep the installed Fluent UI system. Do not introduce a second design system.

### Mode chooser

- One short heading and two clearly differentiated actions: Cloud API and Local Whisper.
- Cloud describes that it opens the existing Google/OpenAI workspace and requires cloud credentials.
- Local describes Whisper -> Gemma -> local voice and links readiness to installed models/voices.
- Full keyboard access, visible focus, no automatic selection, no decorative animation, and a single-column fallback below 768 px.

### Local workspace

- Keep the existing top-level session commands and transcript feed behavior.
- Show a compact semantic pipeline rail for Listening, Transcribing, Translating, Synthesizing, and Speaking. These are real state indicators, not decoration.
- Reuse current audio controls for meeting source, translated output device, output channel, original monitor, test tone, and source/output meters.
- Keep advanced Whisper, Gemma, segmentation, and TTS voice settings in a settings surface, not in the live transcript area.
- Provide Change mode without hiding the stop requirement or silently abandoning an active session.

## Current-State Evidence

- `src/App.tsx` currently routes only overlay query strings and otherwise renders `MainApp`, so the chooser belongs after overlay routing and before the main workspace.
- `src/app/MainApp.tsx` currently owns cloud and local state together, defaults to Google, and contains the internal provider selector.
- `src-tauri/src/session.rs` dispatches Google, OpenAI, and local providers. Google and OpenAI already receive the shared playback sender; local currently does not.
- `src-tauri/src/audio.rs` already owns device discovery, Windows loopback enumeration, PCM capture, resampling, selected-output playback, left/right routing, test tone, and translated-audio metering.
- `src-tauri/src/ai/local_whisper_ollama.rs` has the insertion seam immediately after `ollama.translate` succeeds.
- Local mode currently bypasses output validation and playback creation. Frontend controls and docs explicitly label it text-only; those special cases must be removed.
- The configured Ollama model is currently arbitrary and empty by default. The extension keeps the serialized field compatible but uses `gemma3:4b` as the new default and labels it as the Gemma model.

Detailed evidence is recorded in [research/current-state-and-options.md](research/current-state-and-options.md).

## Architecture

### Workspace boundary

Add an app-level `ApplicationMode = "cloud" | "local"` selection for the main route only.

- Keep overlay route resolution first and unchanged.
- Add `ModeChooser` and a small mode host in `App.tsx`.
- Preserve the current cloud render path as the behavioral baseline.
- Extract only the shared session view-model/actions required by the new local surface. Do not duplicate Tauri listeners, routing persistence, or session lifecycle logic.
- In the cloud workspace, retain Google and OpenAI controls and behavior. Local configuration moves to the dedicated local workspace.
- Returning to the chooser is rejected while a session is active unless the user stops it first.

### Local configuration and migration

Extend `LocalTranslationConfig` without breaking existing persisted JSON:

| Group | Field | Rule |
| --- | --- | --- |
| Gemma | `model` | Preserve existing value; default empty values to `gemma3:4b` |
| TTS | `voiceId` | Required installed voice matching target language `vi` |
| TTS | `rate` | Default 1.0, clamped to a conservative platform-neutral range |
| TTS | `volume` | Default 1.0, clamped to 0.0-1.0 |
| TTS | `outputSampleRateHz` | Read-only 24000 normalized PCM contract |

Add commands to list installed local voices and synthesize a short routed test phrase. A local provider is ready only when Whisper loads, Gemma responds, a Vietnamese-capable voice is available, an output device is selected, and the test state is not stale after a runtime-critical edit.

### TTS engine boundary

Add `src-tauri/src/tts.rs` with a platform-neutral contract:

```text
LocalTtsEngine
  list_voices() -> LocalVoice[]
  synthesize(text, voice, rate, volume, cancellation)
    -> SynthesizedAudio { pcm16_mono, sample_rate_hz }
```

- Windows uses `SpeechSynthesizer.SynthesizeTextToStreamAsync`, reads the returned stream, validates/decodes its audio container, and exposes installed `AllVoices`.
- macOS uses `AVSpeechSynthesizer.writeUtterance(...toBufferCallback:)`, converts callback buffers to mono PCM16, and exposes installed voices.
- Normalize both platform outputs to 24 kHz PCM16 mono before playback.
- Add focused tests around decoding, channel downmix, sample-rate conversion, empty buffers, missing voices, cancellation, and unsupported formats.
- Do not let platform TTS play directly to the OS default speaker. All audio must pass through the existing selected CPAL output.

The platform-buffer approach is supported by the official Windows stream API and Apple buffer callback API. See the research report for source links.

### Runtime topology

```text
CPAL capture 16 kHz
  -> bounded utterance segmenter
  -> Whisper worker
  -> pending source transcript snapshot
  -> ordered Gemma /api/chat request
  -> final translated transcript snapshot
  -> bounded TTS request queue
  -> platform TTS synthesis
  -> PCM16 mono 24 kHz
  -> existing PlaybackRuntime
  -> selected output device and selected all/left/right channel
```

- Create local playback at session start and require a translated output device, just like cloud modes.
- Pass a playback sender into the local runtime without changing the Google/OpenAI function contracts.
- Keep translation text final even if TTS later fails. Attach speech failure state separately and emit one actionable app error.
- Serialize translated utterances into the TTS queue to preserve speech order. Bound the queue and never grow memory indefinitely.
- Set `speaking` while local audio is actively being delivered and keep the existing translated-audio-level event/meter.
- Pause stops accepting new local utterances and lets the current spoken sentence finish. Stop, mode change, shutdown, and generation invalidation cancel synthesis immediately, clear queued speech, and drop playback so late audio cannot play.
- A slow TTS engine must not block capture, Whisper, the Tokio runtime, or the React event loop.

### State and event compatibility

- Retain `session-status`, `transcript-update`, `audio-level`, `translated-audio-level`, and `app-error`.
- Add a local pipeline-stage event only if the existing session status cannot distinguish transcribing, translating, and synthesizing without changing cloud semantics.
- Keep transcript snapshots keyed by stable ID and revision. Audio state must reference the same utterance ID for diagnostics but must not mutate already-final translation text.
- Existing exports remain text-based and unchanged.

## Phases

| Phase | Document | State | Outcome |
| --- | --- | --- | --- |
| 01 | [phase-01-contracts-and-config.md](phase-01-contracts-and-config.md) | implemented baseline | Local provider/config/client contracts |
| 02 | [phase-02-local-runtime.md](phase-02-local-runtime.md) | implemented baseline | Whisper -> Ollama text runtime |
| 03 | [phase-03-settings-and-transcript-ui.md](phase-03-settings-and-transcript-ui.md) | implemented baseline | Local settings and snapshot UI |
| 04 | [phase-04-verification-and-documentation.md](phase-04-verification-and-documentation.md) | partial baseline | Existing text-only tests/docs |
| 05 | [phase-05-mode-gateway-and-workspaces.md](phase-05-mode-gateway-and-workspaces.md) | implemented | Main-route mode chooser and dedicated local workspace boundary |
| 06 | [phase-06-local-tts-contracts.md](phase-06-local-tts-contracts.md) | implemented; macOS hardware pending | Cross-platform local voice/config/synthesis contracts |
| 07 | [phase-07-spoken-runtime-and-routing.md](phase-07-spoken-runtime-and-routing.md) | implemented | Gemma result -> TTS -> selected CPAL output lifecycle |
| 08 | [phase-08-regression-and-release-validation.md](phase-08-regression-and-release-validation.md) | automated checks complete; hardware pending | Cloud regression, hardware tests, docs, and release evidence |

## Dependencies and Cross-Plan Coordination

- Coordinates with `plans/realtime-meeting-translation-macos`, which owns shared capture, playback, session lifecycle, and Google/OpenAI behavior.
- Coordinates with `plans/260712-2234-application-ui-modernization`, which owns the Fluent shell and accessibility conventions.
- This plan owns the chooser, local workspace, Gemma defaults, TTS adapters, and local spoken runtime.
- No plan is a hard blocker because the required audio and session foundation is implemented.

Runtime prerequisites:

- User-provided compatible multilingual Whisper GGML model.
- Running local Ollama with the configured Gemma model, default `gemma3:4b`.
- An installed Vietnamese-capable system voice.
- A selected translated-audio output device.

## File Impact Map

| Area | Expected files |
| --- | --- |
| Main route/mode host | `src/App.tsx`, new `src/app/ModeChooser.tsx`, route tests |
| Shared/live workspace | `src/app/MainApp.tsx`, recommended local workspace component and shared session hook/view-model |
| Local settings | `src/components/settings/LocalLlmSettings.tsx`, related tests |
| Frontend contracts/bridge | `src/types.ts`, `src/api.ts` |
| Styles | `src/styles/app.css` using existing Fluent tokens/conventions |
| Rust config/contracts | `src-tauri/src/models.rs`, `src-tauri/src/local_translation.rs` |
| TTS | new `src-tauri/src/tts.rs` with platform modules as needed |
| Local worker | `src-tauri/src/ai/local_whisper_ollama.rs`, `src-tauri/src/ai.rs` |
| Session/playback integration | `src-tauri/src/session.rs`; minimal helper exposure in `src-tauri/src/audio.rs` only if required |
| Commands/registration | `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs` |
| Native dependencies | `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` |
| Docs | `README.md`, Windows/macOS user/release guides, architecture diagrams |

## Risks and Mitigations

| Risk | Mitigation |
| --- | --- |
| Platform TTS emits different formats/sample rates | Normalize behind `LocalTtsEngine`; fixture tests per supported container/buffer format |
| Vietnamese voice is not installed | Voice discovery plus readiness gate and actionable platform setup guidance |
| TTS bypasses selected headset | Reject direct speaker APIs; require PCM through existing CPAL playback |
| Local queues lag behind live speech | Separate bounded TTS queue, serialized ordering, explicit backlog error, no unbounded buffers |
| Stop still plays queued speech | Generation token plus cancellation checks, queue clear, and immediate playback runtime drop |
| Mode chooser breaks overlays | Resolve overlay routes before mode selection and add route isolation tests |
| Refactoring `MainApp` regresses cloud UI | Preserve cloud JSX/event path, extract only shared controller seams, run snapshot/behavior regression tests |
| Existing local config is lost | Add serde/default migration; preserve non-empty model/path/tuning fields |
| Gemma response succeeds but TTS fails | Keep transcript final, show speech-specific error, allow replay after TTS recovery |
| Output/monitor routing conflicts return | Reuse current conflict validation and opposite-channel rules for local audio |

## Success Criteria

- The main window always presents Cloud API and Local Whisper choices before entering a workspace; overlays still open directly.
- Cloud API opens the existing cloud interface and Google/OpenAI capture, credentials, translation, playback, events, summaries, overlays, exports, and routing behave unchanged.
- Local Whisper uses the current capture/device selection and produces `audio -> whisper-rs -> Japanese text -> Gemma -> Vietnamese text -> local TTS -> selected headset`.
- The local translated output device, all/left/right channel, test tone, translated meter, and routing warnings reuse current audio features.
- Local mode cannot start without a readable Whisper model, successful Gemma check, installed compatible voice, input device, and translated output device.
- A successful translation remains visible if speech synthesis fails, and retrying speech does not duplicate the transcript item.
- Speech order matches transcript order under slow Gemma/TTS, queues remain bounded, and no late audio or transcript mutation occurs after stop or mode change.
- Existing persisted local settings migrate without losing user paths or tuning.
- Keyboard navigation, focus return, 200% zoom, forced colors, reduced motion, and sub-768 px layout work for the chooser and local workspace.
- `npm test`, `npm run build`, `cargo test`, `cargo fmt --check`, and `cargo clippy --all-targets -- -D warnings` pass.
- Windows and macOS hardware validation confirms Vietnamese voice synthesis and playback to a non-default selected headset before the plan is complete.

## Review and Validation

- Adversarial review: [reports/red-team.md](reports/red-team.md)
- Critical validation: [reports/validation.md](reports/validation.md)
- The plan is GO with platform voice availability and real-device playback retained as release gates.

## Implementation Handoff

Implement Phases 05-08 in order. Keep the existing Google implementation untouched unless a regression test exposes a shared-runtime defect. Do not mark Phase 08 complete without real Windows and macOS output-device evidence.

Suggested command:

```text
/hi-craft plans/260716-2033-local-llm-audio-translation/plan.md --full
```
