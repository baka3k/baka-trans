# Phase 11: Translation Engine Contract and Rust Manager

## Context

The sidecar and model lifecycle are stable, but production code still constructs `OllamaClient` directly. This phase adds backward-compatible engine configuration, one translation dispatcher, and a Rust owner for the HY-MT process/model state. Live pipeline routing remains disabled until Phase 13.

## Requirements

- Preserve serialized `TranslationProvider::LocalWhisperOllama` and current session selection.
- Add `LocalTranslationEngine::{Ollama, HyMt}` serialized as `ollama` and `hy_mt`.
- Migrate schema-v1 config to v2 with `ollama`; preserve every existing field.
- Keep Ollama client/payload/parser and smoke test intact.
- Expose one enum-dispatched async translate contract returning text and latency with stable engine-specific errors.
- Rust owns install/start/prewarm/translate/cancel/restart/unload/shutdown; it never exposes the child pipes to React.
- The manager must validate protocol version, model ID/revision, request ID, actual device/dtype, and child identity before Ready.

## Architecture

```text
LocalTranslationEngineClient
  Ollama(OllamaClient)
  HyMt(AppHandle -> HyMtManager)

HyMtManager
  model status/install task
  lazy ManagedProcess + stdin/stdout channel
  one-flight async mutex + deadlines + bounded restarts
```

## Related Files

- `src-tauri/src/models.rs`
- `src-tauri/src/local_translation.rs`
- New `src-tauri/src/hy_mt.rs`
- `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `Cargo.lock`
- Rust unit/integration tests near the owning modules.

## Implementation Steps

1. Add engine/config/runtime status/progress/test-result types with serde defaults and frontend-compatible camelCase output.
2. Implement explicit schema-v1 → v2 migration, atomic config rewrite, and tests for missing/default/legacy/invalid engine values.
3. Extract engine-neutral prompt request/result/error concepts while leaving Ollama behavior unchanged.
4. Add enum dispatch and a mockable boundary for unit tests; avoid silent fallback and avoid `async_trait` unless enum dispatch cannot express the call.
5. Implement `HyMtManager` paths, install/cancel/status/repair, command resolution, short-lived installer process, progress forwarding, long-lived serve process, and parent-pipe ownership.
6. Add translate request correlation, one-flight locking, timeout/cancel, child exit detection, bounded restart/fuse, unload, and app shutdown cleanup.
7. Register manager state and Tauri commands; ensure model install/readiness can be queried without starting Whisper or a meeting session.
8. Add fake-sidecar fixtures/tests for protocol mismatch, wrong model/revision, malformed responses, stale IDs, timeout, crash/restart, cancellation, and fuse exhaustion.

## Todo

- [ ] Config v2 migration preserves all v1 values.
- [ ] Ollama tests/behavior remain unchanged.
- [ ] HY manager states and commands are backend-authoritative.
- [ ] Protocol/identity/timeout/restart tests cover failure paths.
- [ ] No silent engine fallback exists.

## Risks

- Adding a config field without true migration can make old files fail or reset values. Test real serialized fixtures.
- Blocking pipe I/O can stall Tokio. Isolate blocking reads/writes in dedicated threads or `spawn_blocking` and bound every wait.
- Killing an in-flight sidecar can race with a late response. Correlate IDs and invalidate process generations.
- Generalizing VieNeu and HY-MT managers prematurely may create a fragile abstraction. Reuse patterns, not a shared god manager.

## Success Criteria

- Existing configs and Ollama sessions behave identically after migration.
- HY status/install/start/translate/cancel/restart/unload commands work against a fake sidecar without a real model.
- Manager shutdown and deadline paths leave no child process, blocked thread, stale response, or active request.
- Rust errors are stable/actionable and retain enough structured state for the settings UI.
