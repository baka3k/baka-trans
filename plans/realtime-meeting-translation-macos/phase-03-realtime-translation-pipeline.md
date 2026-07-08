# Phase 03 - Realtime Translation Pipeline

Status: planned
Depends on: phase 02

## Goal

Connect captured audio to OpenAI realtime translation, receive translated audio and transcripts, and provide a debug fallback pipeline.

Manual user-controlled utterance boundaries for nonstop speech are planned separately in `phase-07-manual-utterance-boundary.md`, because they require live control messages into the realtime WebSocket loop after the baseline pipeline exists.

## Primary Path

Use the OpenAI Realtime Translation endpoint:

- endpoint: `/v1/realtime/translations`
- model: `gpt-realtime-translate`
- transport: WebSocket from the Rust backend
- input: PCM16 mono chunks from the audio capture module
- output: translated audio stream plus transcript deltas

## Implementation Tasks

- Implement secure API key lookup:
  - Keychain first
  - `OPENAI_API_KEY` as development fallback
  - never expose standard API key to frontend
- Implement realtime translation client:
  - connect/reconnect lifecycle
  - session configuration from UI settings
  - source and target language handling
  - style prompt/config where supported by the API
  - audio append loop
  - graceful `session.close`
- Implement event parser:
  - source transcript deltas
  - translated transcript deltas
  - translated audio chunks
  - session lifecycle events
  - API errors/rate-limit errors
- Feed translated audio into playback queue.
- Emit transcript and status events to React.
- Track latency metrics:
  - capture timestamp
  - first transcript delta
  - first translated audio
  - playback queued
- Add fallback mode:
  - chunk audio every short utterance or fixed window
  - transcribe with `gpt-4o-transcribe` or `gpt-4o-mini-transcribe`
  - translate with a low-latency text model such as `gpt-5.4-mini`
  - synthesize with `gpt-4o-mini-tts`
  - play generated audio
- Normalize user-visible errors:
  - missing API key
  - invalid key
  - quota/rate limit
  - network unavailable
  - unsupported audio format

## Verification

- A short fixture or microphone sample can be sent to the realtime path.
- Transcript deltas render in the UI.
- Translated audio reaches the playback queue.
- Stop closes WebSocket and audio streams.
- Fallback mode works with a recorded short sample.
- API/auth/quota errors appear as actionable UI messages.

## Exit Criteria

- End-to-end translation works with a controlled audio source.
- The backend can recover from transient network failures with bounded retry/backoff.
