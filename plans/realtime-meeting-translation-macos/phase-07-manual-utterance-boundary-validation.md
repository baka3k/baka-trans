# Phase 07 Validation Pass - Manual Utterance Boundary

Status: completed
Reviewed plan: `phase-07-manual-utterance-boundary.md`

## Critical Questions

1. What proves the feature is useful?
   - A continuous-speech audio fixture delays automatic translation, but pressing `Translate now` produces translated text/audio while the session keeps capturing.

2. What must remain true after pressing the button?
   - Capture, playback, monitor playback, transcript updates, and stop/pause behavior continue to work.

3. What is the riskiest unknown?
   - Whether the dedicated realtime translation endpoint accepts the same manual buffer commit semantics and response-trigger behavior as the generic Realtime WebSocket events.

4. What should the first version avoid?
   - Avoid exposing VAD threshold tuning and avoid switching the app into manual-only segmentation by default.

5. What should be measured?
   - Button press to commit acknowledgement, button press to first translated text, button press to first translated audio, ignored empty-buffer count, and duplicate transcript count.

## Validation Verdict

The phase is implementation-ready after a small protocol spike confirms the exact commit and response events for the translation session. Build the control channel first, then UI, then optional manual-turn-detection mode only if live validation requires it.
