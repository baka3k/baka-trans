# Phase 12: Gemini Live Translation Pipeline

Status: planned
Depends on: phase 11
Primary files: `src-tauri/src/ai.rs`, `src-tauri/src/ai/google_live.rs`, `src-tauri/src/audio.rs`, `src-tauri/src/models.rs`, `src/transcript.ts`

## Objective

Implement Google Gemini Live Translation as a first-class realtime translation backend while reusing the existing capture, playback, session, transcript, and UI contracts.

## Official API Mapping

- WebSocket endpoint: `wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent?key=...`
- Model: `models/gemini-3.5-live-translate-preview`
- Setup payload:

```json
{
  "setup": {
    "model": "models/gemini-3.5-live-translate-preview",
    "generationConfig": {
      "responseModalities": ["AUDIO"],
      "inputAudioTranscription": {},
      "outputAudioTranscription": {},
      "translationConfig": {
        "targetLanguageCode": "vi",
        "echoTargetLanguage": false
      }
    }
  }
}
```

- Audio input payload:

```json
{
  "realtimeInput": {
    "audio": {
      "data": "<base64-pcm16>",
      "mimeType": "audio/pcm;rate=16000"
    }
  }
}
```

- Output events:
  - `serverContent.inputTranscription.text`
  - `serverContent.outputTranscription.text`
  - `serverContent.modelTurn.parts[].inlineData.data`

## Implementation Steps

1. Add Google audio rates.
   - Replace the single `REALTIME_SAMPLE_RATE = 24_000` assumption with provider-specific input/output rates.
   - Capture pipeline should produce 16 kHz PCM16 for Google input.
   - Playback should continue to accept Google 24 kHz PCM16 output.

2. Implement Google WebSocket client.
   - Use `tokio_tungstenite` as with OpenAI.
   - Send setup immediately after open.
   - Send audio chunks as `realtimeInput.audio`.
   - Parse Google server messages into existing transcript/audio events.

3. Normalize transcript handling.
   - Map Google input transcription to `sourceText`.
   - Map Google output transcription to `translatedText`.
   - Preserve the existing `TranscriptItem` contract unless Google exposes stable IDs that justify adding provider metadata.

4. Normalize audio output.
   - Decode base64 `inlineData.data`.
   - Convert little-endian PCM16 bytes to `Vec<i16>`.
   - Emit `translated-audio-level` using the same level calculation.

5. Error handling.
   - Add Google-specific error extraction.
   - Emit provider-specific error codes such as `google_live_connect_error`, `google_live_api_error`, and `google_live_audio_format_error`.
   - Keep UI `app-error` payload shape unchanged.

## Manual Boundary Decision

Do not carry over OpenAI's `session.close` manual-boundary behavior blindly. Google Live Translation is designed for continuous stream processing, and the docs do not describe an equivalent "commit current buffer" operation for translation. Phase 12 should disable or soft-hide the manual boundary button for Google until phase 13 validates an equivalent strategy.

## Acceptance Criteria

- A short 16 kHz PCM fixture can be sent to Google and produce translated audio or transcript output.
- Live microphone/BlackHole capture routes through Google without OpenAI code paths.
- Input and output transcripts appear in the existing transcript UI.
- Translated audio plays through the selected output device.
- OpenAI remains available as a fallback provider.

## Verification

- Unit tests for setup payload and audio message construction.
- Unit tests for Google transcript/audio event parsing.
- Manual smoke test with target `vi` and `en`.
- Manual test that unsupported Google language codes fail before WebSocket setup.
