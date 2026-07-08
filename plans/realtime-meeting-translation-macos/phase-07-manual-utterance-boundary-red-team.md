# Phase 07 Red Team Review - Manual Utterance Boundary

Status: completed
Reviewed plan: `phase-07-manual-utterance-boundary.md`

## Findings

1. Manual commit can race with server VAD.
   - Mitigation: track buffered-audio state, parse commit/error events, and test close-together automatic/manual commits for duplicate transcripts.

2. The current code uses translation-specific event names.
   - Mitigation: verify event names against the active translation endpoint before coding, and keep protocol constants isolated in the AI module.

3. A blocking UI command could disrupt the realtime loop.
   - Mitigation: send a control message to the async task that owns the WebSocket writer; do not let Tauri command handlers write to the socket directly.

4. Too many button presses can generate empty-buffer errors or fragmented translations.
   - Mitigation: debounce requests, keep pending state, and make empty-buffer responses nonfatal.

5. Disabling VAD globally would make the app less realtime for normal meetings.
   - Mitigation: keep automatic VAD as default; add manual-only mode only after validation proves it is needed.

## Verdict

Proceed. This is a practical fallback, but it should be implemented as a narrow control channel and tested with continuous-speech fixtures before exposing deeper VAD settings.
