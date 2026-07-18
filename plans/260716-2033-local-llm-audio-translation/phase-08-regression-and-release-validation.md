# Phase 08: Regression and Release Validation

## Context

This change touches application entry, a large shared React surface, platform speech frameworks, and session playback rules. Completion requires automated evidence plus real Windows and macOS device tests.

## Requirements

- Prove the cloud path is behaviorally unchanged.
- Validate chooser/local workspace accessibility and responsiveness.
- Validate local TTS with real installed Vietnamese voices on both supported operating systems.
- Validate selected non-default output devices and left/right routing.
- Document model, voice, privacy, performance, and troubleshooting requirements.

## Verification Layers

1. Pure tests for route resolution, mode state, config migration, Gemma defaults, buffer decoding, normalization, and queue ordering.
2. Mock integration tests for Whisper/Gemma/TTS success, timeout, error, cancellation, and retry.
3. Existing cloud provider regression tests and payload fixtures.
4. Opt-in local end-to-end fixture test.
5. Real hardware validation on Windows and macOS.

## Related Files

- frontend tests adjacent to changed components
- Rust tests adjacent to TTS, local worker, audio, session, and config modules
- `README.md`
- `docs/WINDOWS_TEAMS_USER_GUIDE.md`
- `docs/BLACKHOLE_TEAMS_USER_GUIDE.md`
- `docs/WINDOWS_RELEASE_GUIDE.md`
- `docs/RELEASE_GUIDE.md`
- `docs/baka-trans-architecture.mmd`
- `docs/baka-trans-architecture.drawio`

## Implementation Steps

1. Run the full frontend and Rust default suites with no model, Ollama, or external service required.
2. Add cloud regression assertions for credentials, provider payloads, output playback, transcript events, summaries, overlays, and exports.
3. Test chooser main/overlay routing, focus order, 200% zoom, forced colors, and reduced motion.
4. Test local readiness and error recovery for missing Whisper, Ollama offline, Gemma missing, voice missing, output missing, and device unplug.
5. Extend the opt-in smoke test through TTS with environment-selected model, voice, and Japanese PCM fixture.
6. On Windows, validate WASAPI loopback input, installed Vietnamese voice, selected headset, test tone, both ears, left, and right.
7. On macOS, validate virtual/system input profile, installed Vietnamese voice, selected headset, test tone, both ears, left, and right.
8. Verify pause, stop, mode change, repeated start/stop, long silence, rapid speech, slow Gemma, and slow TTS.
9. Update setup and troubleshooting docs. Remove every text-only claim.
10. Update architecture diagrams after event/command names are final.

## Todo

- [ ] Default CI remains external-service free.
- [ ] Cloud regression suite passes unchanged.
- [ ] Opt-in local spoken smoke test passes.
- [ ] Windows hardware evidence recorded.
- [ ] macOS hardware evidence recorded.
- [ ] User guides and architecture match runtime behavior.

## Risks

- CI cannot prove installed system voice or physical routing. Keep hardware gates explicit.
- Timing assertions can be flaky on CPU-only systems. Assert bounds, order, cancellation, and liveness rather than one strict latency number.
- Model and voice availability differ by region. Document discovery and install steps without bundling restricted assets.

## Success Criteria

- `npm test`, `npm run build`, `cargo test`, `cargo fmt --check`, and strict Clippy pass.
- Current cloud behavior has automated regression evidence.
- Both supported platforms speak Vietnamese through a user-selected non-default output.
- No queued speech survives stop, mode change, or shutdown.
- Documentation contains no text-only local-mode claims and accurately describes Whisper -> Gemma -> TTS.
