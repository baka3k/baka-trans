# Google Migration Red Team

Status: completed
Date: 2026-07-10

## Findings

1. High risk: treating Google as a schema-compatible OpenAI replacement will break audio.
   - Current capture is hard-wired around 24 kHz realtime audio.
   - Google Live Translation requires 16 kHz PCM16 input and returns 24 kHz PCM16 output.
   - Mitigation: phase 12 makes input/output rates provider-specific before sending live audio.

2. High risk: manual boundary behavior is OpenAI-specific.
   - Current fallback sends `session.close` and reconnects.
   - Google docs describe continuous translation but not an equivalent commit command.
   - Mitigation: disable or replace manual boundary for Google until a validated Google control exists.

3. High risk: long meetings can be interrupted by Google session and connection limits.
   - Audio-only sessions have a documented 15-minute limit without session management, and connections end around 10 minutes.
   - Mitigation: phase 13 includes GoAway, resumption handles, rotation, and context compression.

4. Medium risk: wider Google language support can corrupt existing language assumptions.
   - Current code uses OpenAI-oriented target support and simplified `zh`/`pt` codes.
   - Google uses BCP-47 variants such as `zh-Hans`, `zh-Hant`, `pt-BR`, and `pt-PT`.
   - Mitigation: phase 11 introduces provider-specific language metadata.

5. Medium risk: transcript parity can increase cost.
   - Input/output transcripts are useful for the UI but add text-token billing.
   - Mitigation: phase 13 adds transcript toggles and cost-aware defaults.

6. Medium risk: "translation style" may not map to Google Live Translation.
   - Google Live Translation supports simplified translation config, not arbitrary style instructions.
   - Mitigation: preserve the UI field only where it has a real provider mapping, or move style control to fallback/non-live paths.

7. Low risk: direct frontend WebSocket looks attractive but mismatches current architecture.
   - Audio capture is Rust/cpal and secrets already live in Rust.
   - Mitigation: keep Rust server-to-server first; evaluate ephemeral-token frontend direct only after profiling.

## Verdict

GO with caution. Use provider abstraction first, ship Google behind a selectable provider, and only retire OpenAI after Google passes fixture, smoke, long-session, and live meeting validation.
