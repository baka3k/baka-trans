# Google Migration Validation

Status: completed
Date: 2026-07-10

## Critical Questions

1. Can Google be the only service for the hot translation loop?
   - Yes, if Live Translation quality and latency pass real meeting tests.
   - The implementation must respect Google-specific audio rates, message schema, and session limits.

2. Should the migration remove OpenAI immediately?
   - No. Keep OpenAI as a legacy fallback until Google completes smoke tests, a long synthetic run, and at least one realistic Teams/BlackHole workflow.

3. Does Google Live Translation cover the existing "style" setting?
   - Not directly based on current docs. The plan should either hide style for Google or make it fallback-only.

4. Are ephemeral tokens required for this desktop app?
   - Not for the first Rust server-to-server path. They are useful if React opens the WebSocket directly later, and they should be constrained if that path is added.

5. What proves the migration is complete?
   - Google-only capture, translation, playback, transcript, export, and summary all work without an OpenAI key.
   - Remaining OpenAI references are either removed or intentionally marked legacy/optional.

## Selected Plan

Use Option B: provider abstraction with side-by-side OpenAI and Google backends. Add Google Live Translation, then harden session management and cost controls, then migrate the summary agent and documentation to Google-first.

## Must-Not-Skip Checks

- Verify Google receives 16 kHz PCM16 input.
- Verify Google output audio decodes as 24 kHz PCM16.
- Verify target language metadata with BCP-47 variants.
- Verify long-session behavior around GoAway and resumption.
- Verify UI does not advertise OpenAI-only manual boundary behavior for Google.
- Verify the app can run with only `GEMINI_API_KEY` or a saved Google Keychain credential.
