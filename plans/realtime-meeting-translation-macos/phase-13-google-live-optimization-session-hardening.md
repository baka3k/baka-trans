# Phase 13: Google Live Optimization and Session Hardening

Status: planned
Depends on: phase 12
Primary files: `src-tauri/src/ai/google_live.rs`, `src-tauri/src/session.rs`, `src-tauri/src/models.rs`, `src/App.tsx`, `src/types.ts`, `src/styles.css`

## Objective

Optimize the Google backend for real meetings: stable long sessions, predictable latency, lower avoidable cost, and visible controls for Google-specific behavior.

## Optimization Items

1. Chunking and backpressure.
   - Send Google audio in 100 ms chunks.
   - Track dropped chunks, queue depth, and send latency.
   - Surface a warning when capture produces audio faster than the WebSocket can send.

2. Session duration handling.
   - Handle `goAway.timeLeft` messages.
   - Store and refresh session-resumption handles when Google sends resumable updates.
   - Rotate connections before the connection limit ends.
   - Decide whether buffered audio should be replayed across reconnects or dropped with a visible gap marker.

3. Context and billing controls.
   - Add `contextWindowCompression` settings once validated against the Live Translation setup schema.
   - Make input/output transcription toggles configurable because transcriptions improve transcript UI but add text-token costs.
   - Add a "meeting mode" default that enables transcripts and a "lowest cost" mode that disables optional transcript output when the user only needs audio.

4. `echoTargetLanguage`.
   - Expose a setting for same-language speech handling.
   - Default to false for cleaner meetings unless the user wants target-language speech echoed to headphones.
   - Warn that background audio may create artifacts when echo is enabled.

5. Manual boundary replacement.
   - Validate whether Google has an equivalent endpoint or stream-control pattern.
   - If no equivalent exists, keep the button disabled for Google and replace it with a "signal/latency diagnostic" action.
   - If a safe equivalent exists, route it through `RealtimeControl` without using OpenAI event names.

6. Diagnostics.
   - Track provider, model, input sample rate, output sample rate, chunk duration, transcript toggles, reconnect count, last GoAway time, and last API error.
   - Keep diagnostics behind an existing settings/details area rather than cluttering the live workflow.

## Acceptance Criteria

- Google sessions can run beyond a single connection limit without a hard stop in normal network conditions.
- The UI clearly shows when Google is reconnecting/resuming.
- The user can select `echoTargetLanguage` and understand its tradeoff.
- Optional transcript toggles are available and reflected in setup payloads.
- Manual boundary UI is provider-aware and not misleading when Google is selected.
- Cost-related settings are documented in-app through concise labels and defaults.

## Verification

- Unit tests for GoAway/session-resumption event handling.
- Unit tests for chunk sizing and provider-specific rate configuration.
- Manual 30-minute synthetic-audio run with Google selected.
- Manual network interruption test.
- Compare latency and transcript completeness across default and low-cost modes.
