# Prediction Report: Audio Routing, Controls, and Realtime UX

Date: 2026-07-08
Depth: quick
Verdict: CAUTION

## Executive Summary

The current app is viable for realtime translation with one translated output and an optional original monitor output. The main gaps are UX clarity and routing capability: the code downmixes input to mono, writes translated playback to every output channel, mirrors original audio through a separate playback stream, and does not expose left/right channel routing or measured end-to-end latency.

## Agreements

- The existing architecture can support simple channel routing by changing playback fan-out behavior.
- Input signal feedback should show both signal presence and level, not only a silent meter.
- Sound test actions should not disable the whole app UI for the duration of the tone.
- The app must not imply left/right ear split is supported unless controls map to real playback behavior.
- Latency should be described honestly until transcript/audio events carry measurable timing.

## Conflicts

| Topic | Architect | Security | Performance | UX | Devil's Advocate | Resolution |
| --- | --- | --- | --- | --- | --- | --- |
| Add stereo split now | Fits existing playback boundary if implemented as channel fan-out | No new secret or auth risk | Low cost, but two streams to one device can vary by driver | Valuable for meeting use | Might be unreliable on some devices | Implement explicit all/left/right routing, label it as route control, keep separate-device mode intact |
| Measure latency now | Needs timestamp propagation across capture/API/output | No security issue | Accurate measurement is non-trivial | Users want delay visibility | Fake latency is worse than no latency | Do not fake metrics. Show realtime status and leave true latency telemetry for a follow-up |
| Keep global busy flag | Simple | No concern | Blocks unrelated actions | Feels frozen during test tone | Hides whether app is actually stuck | Split test-tone busy state from session busy |

## Risk Summary

| Risk | Severity | Persona | Mitigation |
| --- | --- | --- | --- |
| Left/right split implied but not implemented | High | UX | Add channel controls wired to backend playback behavior |
| Sound test disables unrelated controls | Medium | UX/Performance | Add per-tone busy state in UI |
| Driver behavior with two streams on one output can vary | Medium | Architect/Devil's Advocate | Keep separate monitor output supported and warn when devices match |
| Latency expectation may be misunderstood | Medium | UX | Avoid fake delay numbers; document realtime path limitation in UI/report |
| Audio device names and routing profile may change after refresh | Low | Architect | Continue resolving stored device IDs by exact ID then normalized name |
| API key handling remains sensitive | Low | Security | Keep Keychain storage path unchanged; do not log secrets |

## Per-Persona Details

### Architect

Concerns:
- Current playback assumes mono input and duplicates samples to every output channel.
- Original monitor and translated playback are separate streams, not a single mixer.
- Adding a full mixer would be larger than needed for this pass.

Recommendations:
- Add a small `AudioOutputChannel` mode at the playback boundary.
- Preserve the existing monitor output abstraction.
- Avoid changing session lifecycle or realtime API flow.

Confidence: high

### Security

Threats:
- No new network surface is needed for channel routing.
- API key storage should remain unchanged.

Severity: low

Mitigations:
- Keep new controls local-only and avoid logging audio samples or secrets.

### Performance

Bottlenecks:
- Realtime delay is dominated by capture buffering, WebSocket transport, model processing, and audio playback queueing.
- Test tone currently blocks its command for about 650 ms and UI uses one global busy state.

Metrics impact:
- Channel fan-out is per-frame assignment and should be negligible.
- Per-tone UI busy state removes perceived blocking without changing audio latency.

Alternatives:
- A proper mixer can be added later if same-device dual-stream routing is unreliable on some hardware.

### UX

Issues:
- Button label `TTS` is ambiguous.
- Input meter lacks a numeric or textual signal state.
- Left/right routing is not visible despite being a natural headphone workflow.
- Refresh has no refreshed-state feedback.

Edge cases:
- Same output selected for translated audio and original monitor.
- Mono output device selected while user chooses left or right only.
- No input signal while session is active.

A11y concerns:
- Icon-only buttons have labels, but sound test feedback needs visible state too.

### Devil's Advocate

Assumptions challenged:
- Same-device dual stream channel routing may not work identically on every CoreAudio device.
- Users may expect translation latency under 500 ms, which the realtime model path cannot guarantee.
- A button for every command can clutter the session panel.

Simpler alternatives:
- Only relabel and warn about unsupported split routing.
- Keep one output and require OS-level routing tools.

Worst case:
- User selects split-ear routing on a device/driver that mixes streams unexpectedly, hearing both languages in both ears.

## Recommendations

1. Add explicit translated/original channel routing controls.
   Rationale: users can intentionally choose both ears, left, or right instead of relying on hidden behavior.

2. Update playback and test tone to honor all/left/right channel modes.
   Rationale: UI controls must map to real audio behavior.

3. Split sound-test busy state from global session busy.
   Rationale: test tone should not make refresh/session UI feel frozen.

4. Improve input signal display.
   Rationale: realtime translation users need immediate confidence that audio is actually entering the app.

5. Do not show fabricated latency.
   Rationale: current models contain `latencyMs`, but the realtime path does not populate it.

## Next Steps

Verdict is CAUTION: proceed with the mitigations above, then run frontend build/tests and Rust checks.
