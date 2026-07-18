---
type: red-team review
date: 2026-07-18
verdict: GO_WITH_GATES
---

# Red-Team Review: Mode Gateway and Local Spoken Translation

## Verdict

GO with two release gates: real Vietnamese voice synthesis on both supported operating systems and playback through a selected non-default output device.

## Critical Challenges

### 1. A new local UI could duplicate the session controller

If the dedicated screen creates its own Tauri listeners or lifecycle state, events can be applied twice and stop can leave orphaned workers.

Mitigation: keep one shared controller/view-model and render two workspace surfaces from it. Add listener-count and repeated mount/unmount tests.

### 2. Native TTS can bypass application routing

The easiest platform APIs speak directly to the default device, which would ignore headset and channel selection.

Mitigation: use only synthesis-to-stream/buffer APIs, normalize to PCM, and send through `PlaybackRuntime`. Treat any direct speak call as an architecture violation.

### 3. Stop can leave audible queued speech

Generation checks around transcript mutation do not automatically clear CPAL audio already queued.

Mitigation: make stop invalidate generation, cancel synthesis, clear TTS queue, and drop local playback immediately. Test stop during synthesis and playback separately.

### 4. TTS failure could corrupt a valid translation

If transcript finality is tied to speech success, a missing voice or playback error could turn correct Vietnamese text into an error or duplicate a card on retry.

Mitigation: final text and speech state are separate. Retry targets audio for the same utterance ID only.

### 5. `MainApp.tsx` extraction could change cloud behavior

The large component combines many features. A broad refactor risks credentials, summaries, overlays, export, scroll behavior, and routing.

Mitigation: preserve the cloud render path, extract incrementally behind current tests, and add explicit Google/OpenAI regression fixtures.

## High-Risk Scenarios

- Mode change requested while listening or speaking.
- Output device unplugged after readiness test.
- Gemma returns faster than TTS can speak sustained dialogue.
- Voice disappears after OS update or user removal.
- TTS callback fires after stop.
- Same physical headset selected for translated audio and original monitor with conflicting channels.
- Existing local config contains a non-Gemma model.

## Required Mitigations Before Completion

- Bounded queues at utterance and TTS stages.
- Explicit queue-saturation errors.
- Config migration tests with old JSON fixtures.
- Overlay route isolation tests.
- Selected-output hardware matrix on Windows and macOS.
- No edits to Google request/event semantics unless a shared-runtime defect is demonstrated.
