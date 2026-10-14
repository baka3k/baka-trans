# Architecture

The main route resolves overlay windows first, then presents a per-launch Cloud API or Local Whisper mode chooser. Both workspaces reuse the same React session controller, Tauri event listeners, audio-device profile, transcript store, and lifecycle commands.

Cloud API preserves the Google Live and OpenAI Realtime pipelines. Local Whisper uses a bounded ordered chain:

```text
CPAL capture at 16 kHz
  -> whisper-rs Japanese transcription
  -> selected translation engine (Hy-MT2 / TranslateGemma 4B offline, OpenAI-compatible API, or Gemini)
  -> final transcript snapshot
  -> bounded platform TTS queue
  -> PCM16 mono at 24 kHz
  -> shared CPAL PlaybackRuntime
  -> selected output device and all/left/right channel
```

The OpenAI-compatible engine keeps its API key in the OS keychain (or `BAKA_TRANS_LOCAL_API_KEY`), and a start-time gate fails the session and engine test with `local_openai_api_key_missing` when a non-loopback endpoint has no key; loopback servers may stay keyless. The Gemini engine shares the Cloud panel's Google credential (`google-gemini-api-key` or `GEMINI_API_KEY`) and is always gated: without a key it fails fast with `local_gemini_api_key_missing`. It translates Whisper sentences in sliding-window batches (5 sentences, ≤500 ms wait) with a JSON-array request so the model keeps context across ASR fragments; the offline and OpenAI-compatible engines keep the one-sentence-per-request loop.

Windows synthesis uses `Windows.Media.SpeechSynthesis` stream output. macOS synthesis uses the platform `say` speech service to create a local WAV buffer. Neither path plays directly to the default speaker. Stop drops the shared playback runtime immediately; generation and cancellation checks prevent late local output. A TTS failure emits an app error without replacing the already-final translated text.
