---
type: plan validation
date: 2026-07-18
status: validated
---

# Plan Validation: Mode Gateway and Local Spoken Translation

## Validation Questions

### Does the plan preserve current functionality?

Yes. Cloud API retains the current Google/OpenAI workspace and all shared functions. The plan explicitly treats cloud behavior as a regression baseline and limits the new runtime branch to local TTS and its shared playback seam.

### Does it reuse current audio capture and output selection?

Yes. CPAL device discovery, Windows loopback, capture, input resampling, selected output, left/right routing, test tone, original monitoring, and meters remain the foundation.

### Is the requested local pipeline complete?

Yes. The target topology is audio -> whisper-rs -> Japanese text -> Gemma through native Ollama `/api/chat` -> Vietnamese text -> local TTS -> selected output.

### Is the startup behavior unambiguous?

Yes. The chooser appears on every main-window launch, does not affect overlays, and does not remember the prior choice in this release.

### Is TTS implementable on both supported platforms?

Yes at the contract level. Both platform APIs expose generated audio rather than requiring direct default-device playback. Actual buffer variants and voice availability remain platform release gates.

### Are lifecycle semantics defined?

Yes. Pause stops new intake and lets the current sentence finish. Stop, mode change, shutdown, and stale generation cancel synthesis and playback immediately.

### Are failure boundaries correct?

Yes. Whisper and Gemma failures remain transcript-stage failures. TTS failure does not invalidate final translated text and can be retried without creating a new transcript item.

## Assumptions Accepted for Implementation

- Local language pair remains Japanese to Vietnamese.
- Gemma is hosted by the existing local Ollama service; `gemma3:4b` is the new empty-config default.
- Platform-installed voices are the baseline TTS engine.
- Windows and macOS remain supported equally; neither is declared complete without hardware evidence.

## Validation Result

The plan is internally consistent, scoped to existing seams, and ready for `hi-craft`. No blocking product decision remains. Platform voice and physical output evidence are completion gates, not planning blockers.
