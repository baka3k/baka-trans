# Red Team Review

Status: completed
Reviewed plan: `plan.md`

## Findings

1. Direct Teams audio capture is not realistic for MVP.
   - Mitigation: The plan explicitly uses BlackHole 2ch and documents direct/native Teams capture as out of scope.

2. The latency target is aggressive if implemented as chunked STT -> translation -> TTS.
   - Mitigation: The plan makes dedicated Realtime Translation the primary path and keeps chunked processing only as fallback/debug.

3. API key handling can become a security issue in Tauri if the frontend owns the key.
   - Mitigation: The plan requires Keychain-backed Rust storage and forbids standard API key exposure to React.

4. Audio format conversion is a major failure point.
   - Mitigation: Phase 02 isolates capture, downmixing, resampling, and PCM encoding before API integration.

5. Bluetooth output latency may make user experience worse than the raw API latency.
   - Mitigation: Phase 03 tracks latency milestones and Phase 05 includes output-device soak testing.

6. Translation session event shapes must be verified against current OpenAI API behavior.
   - Mitigation: Phase 03 includes event parser smoke tests and controlled fixture validation before live meeting tests.

7. A user can accidentally create feedback by routing translated output into the captured input.
   - Mitigation: Phase 02 and Phase 04 add warnings for suspicious input/output combinations.

## Verdict

Proceed with caution. The scope is viable if implementation starts with audio primitives and a controlled realtime smoke test before polishing UI. Do not build advanced transcript features, summaries, diarization, or virtual microphone support until the core capture -> translate -> playback loop is stable.
