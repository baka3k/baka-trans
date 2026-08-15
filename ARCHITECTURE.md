# Architecture

The main route resolves overlay windows first, then presents a per-launch Cloud API or Local Whisper mode chooser. Both workspaces reuse the same React session controller, Tauri event listeners, audio-device profile, transcript store, and lifecycle commands.

Cloud API preserves the Google Live and OpenAI Realtime pipelines. Local Whisper uses a bounded ordered chain:

```text
CPAL capture at 16 kHz
  -> whisper-rs Japanese transcription
  -> selected translation engine (Hy-MT2 offline or OpenAI-compatible API)
  -> final transcript snapshot
  -> bounded platform TTS queue
  -> PCM16 mono at 24 kHz
  -> shared CPAL PlaybackRuntime
  -> selected output device and all/left/right channel
```

Windows synthesis uses `Windows.Media.SpeechSynthesis` stream output. macOS synthesis uses the platform `say` speech service to create a local WAV buffer. Neither path plays directly to the default speaker. Stop drops the shared playback runtime immediately; generation and cancellation checks prevent late local output. A TTS failure emits an app error without replacing the already-final translated text.
