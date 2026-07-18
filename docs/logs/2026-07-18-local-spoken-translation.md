# Local Spoken Translation — 2026-07-18

## Context

The local provider previously stopped after Whisper and Ollama text translation. The extension plan adds a launch-time Cloud API / Local Whisper choice and completes the local path as Whisper -> Gemma -> system TTS -> the user's selected audio output, while explicitly preserving Google/OpenAI behavior (`plans/260716-2033-local-llm-audio-translation/plan.md:18`, `plans/260716-2033-local-llm-audio-translation/plan.md:46`).

## Change

The main route now shows an accessible two-mode chooser before mounting a keyed shared `MainApp`; overlay routes still resolve first (`src/App.tsx:24`, `src/App.tsx:30`, `src/app/ModeChooser.tsx:31`, `src/app/ModeChooser.tsx:42`). Cloud mode continues to default to Google and exposes only the existing Google/OpenAI provider controls, while local mode selects the Whisper/Ollama provider and a focused workspace (`src/app/MainApp.tsx:281`, `src/app/MainApp.tsx:1596`).

Added a platform TTS buffer contract that validates/normalizes WAV output to mono PCM16 at 24 kHz, backed by `Windows.Media.SpeechSynthesis` on Windows and the local `say` service on macOS (`src-tauri/src/tts.rs:6`, `src-tauri/src/tts.rs:23`, `src-tauri/src/tts.rs:48`, `src-tauri/src/tts.rs:183`, `src-tauri/src/tts.rs:430`). Final Gemma results enter a bounded ordered TTS queue; synthesis failures and playback overload preserve the final transcript, and generated PCM is sent through the same selected CPAL playback runtime used by cloud providers (`src-tauri/src/ai/local_whisper_ollama.rs:92`, `src-tauri/src/ai/local_whisper_ollama.rs:339`, `src-tauri/src/ai/local_whisper_ollama.rs:407`, `src-tauri/src/session.rs:425`, `src-tauri/src/session.rs:463`).

## Impact

Impact level: high. Users gain an offline spoken translation workflow with retained device/channel routing, and cloud users keep the existing Google/OpenAI runtime path. Automated evidence covers 53 frontend tests, a production build, 74 Rust tests, an installed Windows voice smoke test, formatting, Clippy, and diff checks (`plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:18`, `plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:24`). Release risk remains open for a full non-default-headset routing run on Windows, macOS voice/synthesis/cancellation/playback hardware validation, and desktop-webview visual inspection (`plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:28`, `plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:31`).

## Decision

Kept one React controller/listener lifecycle and unchanged cloud dispatch signatures instead of forking the application or rewriting Google/OpenAI. Platform speech engines return buffers rather than playing directly, so CPAL remains the sole output-device/channel authority. A separate bounded TTS worker preserves speech order without blocking capture or Gemma translation and drops late work through shared generation/cancellation checks. The macOS implementation currently uses `say` to produce the buffer; direct `AVSpeechSynthesizer` remains a release-policy option pending macOS hardware/format evidence (`plans/260716-2033-local-llm-audio-translation/plan.md:127`, `plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:30`).

## References

- plan: `plans/260716-2033-local-llm-audio-translation/plan.md:3`
- implementation evidence: `plans/260716-2033-local-llm-audio-translation/reports/implementation-validation.md:5`
- route regression tests: `src/App.test.tsx:17`, `src/App.test.tsx:33`, `src/App.test.tsx:52`
- baseline commit: `8b4eb75f8183343ebed86812a96fc0f631307147`
