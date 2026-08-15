---
title: "Mode Gateway and Local Spoken Translation"
status: in_progress
created: 2026-07-16
updated: 2026-08-15
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
        -> selected translation engine
           -> OpenAI-compatible Chat Completions API (user-selected), or
           -> managed offline Hy-MT2 1.8B Transformers sidecar
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

### Hy-MT2 and OpenAI-compatible migration decisions (2026-08-15)

**Phase 16 is authoritative for local translation and supersedes every earlier
Ollama/HY-MT1.5 reference in this plan and in Phases 11-13.** Those historical
details describe the baseline only; no new implementation may preserve or
reintroduce an Ollama runtime, endpoint, setting, or fallback.

Three additional scope questions were resolved for the direct Hugging Face extension:

1. **Does “direct Hugging Face” mean hosted inference?** No. The app downloads the pinned `tencent/Hy-MT2-1.8B` files once, then runs them locally through a bundled Python/PyTorch/Transformers sidecar. End users do not install Python and inference does not send meeting text to Hugging Face.
2. **Which local engines remain?** Ollama is removed. The explicit choices are `huggingface_offline` (managed Hy-MT2) and `openai_compatible` (user-supplied Chat Completions endpoint/model). There is no silent fallback between them.
3. **What platforms are in scope?** Run the first gate on the current Apple M5/24 GB machine using explicit MPS selection. Package/sign macOS arm64 next, then build and validate the Windows sidecar on Windows. Windows CPU is the compatibility baseline; CUDA acceleration is capability-tested rather than assumed.

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
- The configured Ollama model remains user-selectable; the current implemented default is `translategemma:4b`. The HY-MT extension preserves this fallback configuration and makes engine-specific labels explicit.

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
| Ollama | `model` | Preserve existing value; default empty values to `translategemma:4b` |
| TTS | `voiceId` | Required installed voice matching target language `vi` |
| TTS | `rate` | Default 1.0, clamped to a conservative platform-neutral range |
| TTS | `volume` | Default 1.0, clamped to 0.0-1.0 |
| TTS | `outputSampleRateHz` | Read-only 24000 normalized PCM contract |

Add commands to list installed local voices and synthesize a short routed test phrase. A local provider is ready only when Whisper loads, Gemma responds, a Vietnamese-capable voice is available, an output device is selected, and the test state is not stale after a runtime-critical edit.

### Translation engine extension

Preserve the serialized session provider value `local_whisper_ollama` so existing settings, tests, and routing remain compatible; relabel it as **Local Whisper** in user-facing copy. Add a backend and frontend `LocalTranslationEngine` enum with `ollama` and `hy_mt` values.

- Increment the local config schema to v2 and migrate v1 documents to `translationEngine: ollama` without losing the Ollama URL/model, Whisper path/tuning, TTS, or audio settings.
- Keep Ollama fields engine-specific and conditionally validated. HY-MT model ID, revision, artifact manifest, prompt, and generation policy are product-owned rather than editable free-text settings.
- Replace the worker's concrete `OllamaClient` with an enum dispatcher behind one asynchronous `translate(text)` contract. Avoid a new trait dependency unless enum dispatch proves inadequate.
- Keep the translation worker single-flight and ordered. HY-MT timeout/failure produces the existing error snapshot and an actionable restart/switch-engine state; it does not duplicate or silently retry the utterance.
- Rename `ai/local_whisper_ollama.rs` only after dispatcher and regression tests protect module behavior.

### Managed HY-MT runtime

Add `sidecars/hy-mt/` and a Rust `HyMtManager` in `src-tauri/src/hy_mt.rs`, using the process-ownership and install-state lessons from the implemented VieNeu manager without coupling the two runtimes.

```text
Rust HyMtManager
  -> installer process (network allowed)
     -> pinned Hugging Face snapshot in staging
     -> exact size/SHA-256 verification
     -> manifest + atomic activation in app-local data
  -> long-lived inference process (network disabled)
     -> load verified local model once
     -> NDJSON stdin/stdout protocol
     -> one in-flight translation + cancellation/deadline
```

Runtime rules:

- Pin the POC-tested HY-MT commit and allowlist only the seven inference files; store `License.txt` and the required notice alongside the install manifest.
- The installer is the only networked mode. Inference sets `HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, disables telemetry, uses `local_files_only=True`, and accepts no Hub token.
- Stdout is protocol-only NDJSON; logs go to sanitized stderr. Every message includes a protocol version and request ID. The ready message declares model ID/revision, device, dtype, PID, and load time.
- Rust lazily starts and prewarms the sidecar before capture, owns stdin/stdout and child lifetime, enforces one request at a time, deadlines, bounded restarts, cancellation, unload, and shutdown.
- Device policy is explicit: probe MPS, CUDA, then CPU; record the actual device/dtype. Do not ship `device_map="auto"` as the product policy.
- Follow the model card prompt exactly for non-Chinese translation: one user message, language name rather than code, no system prompt, `add_generation_prompt=False`, and decode only generated suffix tokens.

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
  -> ordered selected-engine translation
     -> Ollama /api/chat, or
     -> managed offline HY-MT sidecar
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
| 09 | [phase-09-hy-mt-m5-poc-and-gate.md](phase-09-hy-mt-m5-poc-and-gate.md) | pending | Validate exact model, prompt, MPS device policy, quality, latency, memory, and packaging feasibility |
| 10 | [phase-10-hy-mt-sidecar-and-model-lifecycle.md](phase-10-hy-mt-sidecar-and-model-lifecycle.md) | blocked by 09 | Pinned installer, verified model activation, offline NDJSON inference sidecar, and Python tests |
| 11 | [phase-11-translation-engine-and-manager.md](phase-11-translation-engine-and-manager.md) | blocked by 10 | Config-v2 migration, engine dispatcher, and Rust-managed HY-MT lifecycle |
| 12 | [phase-12-hy-mt-settings-and-readiness.md](phase-12-hy-mt-settings-and-readiness.md) | blocked by 11 | Engine-selective settings, install/readiness UX, commands/events, and language validation |
| 13 | [phase-13-hy-mt-pipeline-integration.md](phase-13-hy-mt-pipeline-integration.md) | blocked by 11-12 | Whisper → selected engine → transcript/TTS integration, cancellation, fallback, and regressions |
| 14 | [phase-14-hy-mt-macos-packaging.md](phase-14-hy-mt-macos-packaging.md) | blocked by 13 | macOS arm64 sidecar build, nested signing/notarization, offline and hardware release evidence |
| 15 | [phase-15-hy-mt-windows-packaging.md](phase-15-hy-mt-windows-packaging.md) | blocked by 13 | Windows same-platform build, CPU/CUDA capability matrix, installer/Defender and hardware evidence |
| 16 | [phase-16-hy-mt2-openai-compatible-migration.md](phase-16-hy-mt2-openai-compatible-migration.md) | in progress | Remove Ollama, adopt pinned Hy-MT2 offline runtime, and add explicit OpenAI-compatible translation settings |

## Dependencies and Cross-Plan Coordination

- Coordinates with `plans/realtime-meeting-translation-macos`, which owns shared capture, playback, session lifecycle, and Google/OpenAI behavior.
- Coordinates with `plans/260712-2234-application-ui-modernization`, which owns the Fluent shell and accessibility conventions.
- Uses `plans/260718-2348-managed-vieneu-runtime` as an implemented lifecycle, authenticated/private-process, model-install, and one-folder packaging precedent; HY-MT remains a separate manager and model store.
- Coordinates with `plans/windows-support` for the Windows build, installer, audio hardware, and manual release gate.
- This plan owns the chooser, local workspace, Gemma defaults, TTS adapters, and local spoken runtime.
- No plan is a hard blocker because the required audio and session foundation is implemented.

Runtime prerequisites:

- User-provided compatible multilingual Whisper GGML model.
- One selected translation engine: an installed/verified managed offline Hy-MT2
  runtime, or a configured OpenAI-compatible endpoint whose data-egress warning
  has been acknowledged.
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
| HY-MT process/model manager | new `src-tauri/src/hy_mt.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands.rs`, manager tests |
| HY-MT sidecar | new `sidecars/hy-mt/server.py`, `pyproject.toml`, lockfile, protocol/model/install modules, tests, PyInstaller spec, README |
| Engine/language contracts | `src-tauri/src/models.rs`, `src-tauri/src/local_translation.rs`, `src/types.ts`, `src/languages.ts`, `src/api.ts` and tests |
| HY-MT status/settings | `src/app/MainApp.tsx`, `src/components/settings/LocalLlmSettings.tsx`, related component/app tests |
| Packaging | `src-tauri/tauri.conf.json`, new macOS/Windows HY-MT build scripts, `scripts/release-macos.sh`, `scripts/release-windows.ps1`, `package.json` |
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
| Bundled Python/PyTorch runtime is large or fails signing/AV | Measure the one-folder bundle during Phase 09; build on each target OS; sign nested native libraries; gate macOS notarization and Windows Defender smoke tests |
| 4.09 GB model download is interrupted/corrupt/duplicated | App-private staging, resume, exact allowlist/hash/size validation, atomic activation, repair state, and no global HF cache duplication |
| HY-MT competes with Whisper/TTS for unified memory | Lazy prewarm, one request, bounded queues, explicit device/dtype, M5 memory-pressure gate, and CPU/GPU capability reporting |
| Model or runtime accesses the network while translating | Inference-only offline environment, local path plus `local_files_only=True`, no HTTP listener, and no-network integration test |
| HY-MT output contains explanation/wrong language | Exact model-card prompt, suffix-only decode, output validation, JA→VI quality corpus, and human comparison against current Ollama baseline |
| Stop leaves generation running or produces a late transcript | Request IDs, stopping criteria/cancel message, generation-token check, forced child termination on deadline, and restart before next request |
| Automatic fallback duplicates an utterance | Never fallback per utterance; expose explicit engine switch/restart and preserve one final/error snapshot per ID |

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
- Existing v1 local configs migrate to v2 with `translationEngine: ollama` and preserve all current values; the session provider value remains backward-compatible.
- On the Apple M5/24 GB POC machine, HY-MT loads on the declared device/dtype, produces accepted JA→VI output without explanations, survives a 30-minute warm run with zero queue drops/crashes/late mutations, and keeps combined memory below 80% without macOS memory-pressure escalation.
- The M5 report records cold load, warm p50/p95 translation latency, peak sidecar and combined RSS, quality comparison to TranslateGemma, and exact bundle size; Phase 10 does not start unless the gate is GO.
- HY-MT inference succeeds with network access disabled after installation, and an interrupted/corrupt install is resumable or repairable without activating unverified files.
- Stop/pause/session-generation invalidation cancels or kills in-flight HY generation so no translation or speech arrives late; no utterance is silently rerun through Ollama.
- macOS arm64 and Windows bundles include the managed runtime without requiring system Python. Each bundled executable passes its target-platform offline smoke test; macOS nested libraries are signed/notarized and Windows installer/Defender validation is recorded.
- Internal-use documentation includes the Tencent license/notice, pinned model revision, supported territory assumption, and a manual update procedure; no automatic model update changes reviewed weights.
- Internal deployment documentation states that Vietnam use does not authorize operation or redistribution in license-excluded territories and includes the runtime dependency inventory/notices.

## Review and Validation

- Adversarial review: [reports/red-team.md](reports/red-team.md)
- Critical validation: [reports/validation.md](reports/validation.md)
- HY-MT prediction: [../../plans/reports/prediction_report_20260815_1230.md](../../plans/reports/prediction_report_20260815_1230.md)
- HY-MT research: [research/hy-mt-current-state-and-feasibility.md](research/hy-mt-current-state-and-feasibility.md)
- HY-MT adversarial review: [reports/hy-mt-red-team.md](reports/hy-mt-red-team.md)
- HY-MT critical validation: [reports/hy-mt-validation.md](reports/hy-mt-validation.md)
- The plan is GO with platform voice availability and real-device playback retained as release gates.

## Implementation Handoff

Implement Phases 05-08 in order. Keep the existing Google implementation untouched unless a regression test exposes a shared-runtime defect. Do not mark Phase 08 complete without real Windows and macOS output-device evidence.

For the HY-MT extension, start with Phase 09 only. If its decision is GO, continue Phases 10-13, then package macOS and Windows independently in Phases 14-15. A CAUTION result must name and schedule mitigations; a STOP result leaves Ollama as the active local translation engine and closes Phases 10-15 without partial product integration.

Suggested command:

```text
/hi-craft plans/260716-2033-local-llm-audio-translation/plan.md --full
```
