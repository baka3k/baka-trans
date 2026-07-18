---
type: research
date: 2026-07-18
---

# Research: Current State and Local TTS Options

## Summary

The requested local speech-to-speech flow is an extension, not a greenfield provider. Whisper, segmentation, ordered Ollama translation, transcript reconciliation, capture, device discovery, and selected-output playback already exist. The missing link is a local TTS engine that returns PCM into the existing playback runtime.

## Coverage

- Repository source and plans: used
- Project-specific indexed knowledge tools: unavailable in this session; local source is the authority
- Official technical documentation: used for Whisper, Gemma/Ollama, Windows TTS, and macOS TTS
- External writes or implementation: none

## Findings

### Entry and workspaces

- `src/App.tsx` routes the two overlay query strings and otherwise renders `MainApp` directly.
- The new chooser must sit only on the main route. Inserting it before route resolution would break overlay windows.
- `src/app/MainApp.tsx` owns cloud and local state together and defaults to Google.

### Cloud path

- `src-tauri/src/session.rs` has distinct Google, OpenAI, and local dispatch arms.
- Google Live owns its websocket setup, input upload, translated transcript, PCM decode, playback, translated meter, and speaking status in `src-tauri/src/ai/google_live.rs`.
- The plan can preserve the Google branch and test it as a regression baseline.

### Reusable audio foundation

- `src-tauri/src/audio.rs` already lists CPAL devices, includes Windows loopback sources, captures and resamples input, plays PCM to a selected device, routes all/left/right, emits levels, and supports test tone/local monitoring.
- `PlaybackRuntime` accepts bounded `Vec<i16>` chunks at a declared source sample rate and resamples them to the output device.
- Local TTS should return PCM16 mono and use this sink. It must not call a platform API that speaks directly to the default speaker.

### Existing local runtime

- `src-tauri/src/ai/local_whisper_ollama.rs` already implements bounded segmentation, Whisper in `spawn_blocking`, stable transcript snapshots, and ordered Ollama translation.
- The insertion seam is directly after successful `ollama.translate`.
- `src-tauri/src/local_translation.rs` already calls native Ollama `/api/chat`, but its model is arbitrary and empty by default.
- Local mode is intentionally text-only in current session/frontend/docs. Output validation, playback creation, UI controls, readiness, copy, and guides all contain special cases that must be reversed.

### TTS choice

The selected baseline is installed platform voices because it avoids another server/model while supporting buffer extraction:

- Windows `SpeechSynthesizer.SynthesizeTextToStreamAsync` returns a `SpeechSynthesisStream`, and `AllVoices` exposes installed signed voices.
- Apple `AVSpeechSynthesizer.writeUtterance(...toBufferCallback:)` generates audio buffers for storage or further processing.
- A Rust adapter per platform can normalize both results and then reuse CPAL routing.

Piper remains a valid future adapter but would add model download, licensing, packaging, and another readiness surface that the user did not request.

## Relationships

```text
App main route -> mode chooser -> cloud workspace or local workspace
local session -> current capture -> current local worker -> new TTS -> current playback
cloud session -> current capture -> unchanged Google/OpenAI worker -> current playback
```

## Official References

- [whisper-rs 0.16 WhisperContext](https://docs.rs/whisper-rs/latest/whisper_rs/struct.WhisperContext.html)
- [Ollama Gemma 3 model library](https://ollama.com/library/gemma3)
- [Windows SpeechSynthesizer stream API](https://learn.microsoft.com/en-us/uwp/api/windows.media.speechsynthesis.speechsynthesizer.synthesizetexttostreamasync)
- [Windows SpeechSynthesizer voices and behavior](https://learn.microsoft.com/en-us/uwp/api/windows.media.speechsynthesis.speechsynthesizer)
- [Apple AVSpeechSynthesizer buffer callback](https://developer.apple.com/documentation/avfaudio/avspeechsynthesizer/write(_:tobuffercallback:))

## Gaps and Release Gates

- Exact audio formats returned by installed voices must be tested on supported Windows and macOS versions.
- A Vietnamese voice may not be installed. Local readiness must detect this and explain installation.
- Hardware routing cannot be proven by unit tests. Both platforms require selected non-default headset validation.
