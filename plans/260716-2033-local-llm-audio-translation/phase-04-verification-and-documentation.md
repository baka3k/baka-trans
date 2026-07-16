# Phase 04: Verification and Documentation

## Context

Native Whisper changes the Rust build surface and local Ollama introduces a new runtime prerequisite. Completion requires both automated regression evidence and a reproducible local smoke workflow.

## Requirements

- Verify new contracts without depending on external services in the normal test suite.
- Run a real local end-to-end smoke test with Japanese PCM, Whisper, and Ollama.
- Confirm cloud translation and audio routing regressions did not occur.
- Document setup, model requirements, text-only behavior, troubleshooting, and resource expectations.

## Architecture

Use three verification layers:

1. Pure unit tests for config, segmentation, payloads, parsers, and transcript reconciliation.
2. Mock-server integration tests for exact `/api/chat` behavior, latency, errors, cancellation, and ordering.
3. Opt-in real smoke test gated by environment variables for Whisper model path and Ollama model, excluded from default CI when models are unavailable.

## Related Files

- Rust and frontend test files adjacent to changed modules
- `package.json` scripts only if a clear smoke command is needed
- `README.md`
- `docs/BLACKHOLE_TEAMS_USER_GUIDE.md`
- `docs/WINDOWS_TEAMS_USER_GUIDE.md`
- `docs/baka-trans-architecture.drawio`
- release/check scripts if native dependency setup is required

## Implementation Steps

1. Run formatting, linting, Rust unit/integration tests, frontend tests, TypeScript build, and platform release checks.
2. Add an opt-in Japanese PCM smoke fixture workflow and record expected environment variables without committing model binaries.
3. Verify Start/Stop cycles, long silence, short noise, rapid utterances, manual Translate Now, Ollama offline, missing model, malformed response, slow response, and app shutdown.
4. Verify Google/OpenAI cloud providers still require their keys/output routes and still produce translated audio/text.
5. Verify transcript export contains one source/translation pair per local utterance and no pending placeholder after successful completion.
6. Update user docs with Ollama install/run prerequisites, model pull step, Whisper model selection/path, recommended segmentation defaults, CPU/GPU expectations, and troubleshooting error codes.
7. Update the architecture diagram only after code and event names are final.
8. Record Windows and macOS build results; if one platform cannot be executed locally, leave the phase pending rather than claiming completion.

## Todo

- [ ] Default tests require no Ollama server or Whisper model download.
- [ ] Opt-in end-to-end smoke test passes with a Japanese fixture.
- [ ] Cloud translation/audio regression checks pass.
- [ ] Both platform build results are recorded.
- [ ] Setup and troubleshooting docs are complete.

## Risks

- Model binaries are large and license-sensitive. Never commit them; document supported formats and checksums/source separately.
- A developer machine smoke pass may hide packaging failures. Run release checks on both target operating systems.
- Timing assertions can be flaky on CPU-only machines. Assert ordering, bounds, and cancellation, not an unrealistically strict absolute inference latency.

## Success Criteria

- All required automated checks pass with mock/local fixtures.
- A documented opt-in run demonstrates microphone/PCM16 16 kHz -> Whisper Japanese -> Ollama `/api/chat` -> Vietnamese -> same UI card.
- User docs make the text-only limitation and local prerequisites explicit.
- The plan is marked complete only after Windows and macOS build evidence is available.
