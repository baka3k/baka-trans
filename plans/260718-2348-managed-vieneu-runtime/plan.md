# Managed VieNeu Runtime Plan

## Objective

Make VieNeu-TTS self-managed by Baka Trans: no manual Python install, terminal command, fixed bridge URL, or implicit model download at synthesis time.

## Scope

1. Add a pinned model manifest and bridge install/check modes with progress, resume, SHA-256 validation, atomic manifest creation, and explicit local graph/codec paths.
2. Harden bridge serving with offline-only inference, ephemeral port binding, bearer authentication, startup nonce, stable errors, and parent-pipe lifetime.
3. Add a Rust `VieNeuManager` for app-local paths, install status, lazy child startup, health checks, bounded recovery, cancellation, and shutdown.
4. Route VieNeu voice discovery/readiness/preview/live synthesis through the manager while preserving System TTS.
5. Replace manual URL setup with an accessible install/progress/runtime card and auto-refresh voices after Ready.
6. Add a PyInstaller one-folder spec/build script, bundle resources per platform, and make release scripts fail when the runtime is absent.
7. Verify Python install/check flows, Rust lifecycle/adapters, frontend state rendering, build, Clippy, and regression tests.

## Deferred

- GPU/PyTorch runtime.
- Voice cloning and denoiser artifacts.
- Native Rust VieNeu inference.
- Streaming PCM transport and cooperative mid-utterance cancellation.
- Automatic model updates or model deletion UI.
- macOS sidecar packaging (the PyInstaller spec remains target-platform compatible, but only the Windows release script is wired in this change).

## Acceptance Criteria

- A user can install VieNeu from the settings UI and see aggregate progress and verification phases.
- After installation, the bridge starts without internet access and voices load without a manual command.
- Every model artifact is revision-pinned and size/SHA-256 verified before activation.
- Every RPC endpoint rejects missing/incorrect tokens; the manager validates the startup nonce.
- Closing/crashing the parent closes the child through the inherited pipe.
- System TTS users incur no VieNeu startup or model-load cost.
- Missing runtime, interrupted download, corrupt model, startup timeout, and crash-loop exhaustion produce actionable states.
- Windows release packaging builds and bundles a one-folder sidecar; the same-platform PyInstaller build contract is documented for future targets.

## Review

Reviewed against `prediction_report_20260718_2348.md`. Verdict: CAUTION, approved to implement with its security, performance, and UX mitigations.
