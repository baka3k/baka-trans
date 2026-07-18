# Phase 06: Local TTS Contracts and Platform Adapters

## Context

The local provider ends at Vietnamese text. No TTS dependency, voice discovery, audio-buffer contract, or readiness check exists. Direct platform speech playback would ignore the selected headset, so each platform must return audio buffers to Rust.

## Requirements

- Discover installed voices and identify language/locale.
- Synthesize Vietnamese text locally on Windows and macOS.
- Return normalized PCM16 mono at 24 kHz to the application.
- Persist selected voice, rate, and volume compatibly with existing local config.
- Test the voice and route the test phrase through the selected CPAL output.
- Support cancellation and actionable error codes.

## Architecture

Add a platform-neutral `LocalTtsEngine` in `src-tauri/src/tts.rs`.

```text
list_voices -> LocalVoice[]
synthesize(request, cancellation) -> SynthesizedAudio
SynthesizedAudio -> PCM16 mono, 24000 Hz
```

- Windows adapter: `Windows.Media.SpeechSynthesis.SpeechSynthesizer`, installed `AllVoices`, and `SynthesizeTextToStreamAsync`.
- macOS adapter: `AVSpeechSynthesizer`, installed voices, and `writeUtterance` buffer callback.
- Convert/downmix platform buffers in adapter code, then use a shared resampling/normalization helper.
- Do not open an output device inside either adapter.

## Related Files

- new `src-tauri/src/tts.rs` and platform modules if separation is useful
- `src-tauri/src/models.rs`
- `src-tauri/src/local_translation.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/src/audio.rs` for narrowly exposed normalization helpers only
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src/types.ts`
- `src/api.ts`
- `src/components/settings/LocalLlmSettings.tsx`

## Implementation Steps

1. Add `LocalVoice`, `LocalTtsConfig`, `LocalTtsTestResult`, `TtsRequest`, and `SynthesizedAudio` contracts.
2. Extend config with serde/default migration. Preserve existing model path, server, and segmentation values.
3. Default an empty Ollama model to `gemma3:4b`; preserve non-empty existing model tags.
4. Implement Windows voice discovery and synthesis-to-stream, including container validation/decoding.
5. Implement macOS voice discovery and synthesis-to-buffer callback.
6. Normalize supported outputs to mono PCM16 at 24 kHz and reject empty or unsupported buffers.
7. Add `list_local_tts_voices` and `test_local_tts` commands plus TypeScript wrappers.
8. Add settings for Gemma model, Vietnamese voice, rate, volume, and per-stage readiness.
9. Clear readiness after any Whisper, Gemma, TTS, or output-critical edit.
10. Add platform-independent fixtures and platform-gated adapter tests.

## Todo

- [ ] Existing local JSON migrates without data loss.
- [ ] Windows returns normalized PCM without direct playback.
- [ ] macOS returns normalized PCM without direct playback.
- [ ] Vietnamese voice absence is an actionable readiness error.
- [ ] TTS test uses the selected application output.

## Risks

- Windows streams and macOS buffers may vary by voice/OS version. Validate formats and keep adapters isolated.
- Some machines have no Vietnamese voice installed. Do not silently substitute an unrelated locale.
- Platform callbacks can outlive a session. Bind them to cancellation and generation tokens.

## Success Criteria

- A deterministic adapter fixture becomes PCM16 mono at exactly 24 kHz.
- Installed voices are listed with stable IDs and locale labels.
- The selected Vietnamese voice persists across restart.
- A routed test phrase reaches the selected headset and selected channel on each supported OS.
