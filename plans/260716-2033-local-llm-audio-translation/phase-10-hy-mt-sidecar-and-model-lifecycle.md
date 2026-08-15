# Phase 10: HY-MT Sidecar and Model Lifecycle

## Context

Phase 09 has approved exact model and runtime pins. This phase turns the POC into a deterministic sidecar with a strict protocol and a safe model installer. It does not yet route live sessions through HY-MT.

## Requirements

- User machines need no Python, terminal, Ollama, or global Hugging Face cache for the HY-MT path.
- Installer mode may access Hugging Face; serve mode must be offline-only.
- Downloads are revision-pinned, allowlisted, resumable, fully verified, and atomically activated.
- Runtime communication uses inherited NDJSON stdin/stdout with protocol version and request IDs; no local listening port.
- Load safetensors only with `trust_remote_code=False`; reject symlinks and any resolved path outside the managed staging/active roots.
- One model instance stays resident and supports one translation at a time.
- Input, output, request size, timeout, and error messages are bounded.
- Cancellation uses a Transformers stopping criterion and exits/restarts cleanly if cooperative cancellation misses its grace period.

## Architecture

```text
hy-mt sidecar
  install/check -> staging -> verify manifest -> atomic active model
  serve         -> verified local model -> NDJSON translate/cancel -> result/error
```

## Protocol Contract

```text
ready     {protocolVersion, modelId, revision, device, dtype, pid, loadMs}
translate {type, id, sourceLanguage, targetLanguage, text, maxNewTokens}
result    {id, text, inputTokens, outputTokens, latencyMs}
cancel    {type, id}
error     {id, code, message, retryable}
```

Unknown message types, duplicate active IDs, unsupported languages, oversized text, invalid token limits, oversized protocol lines, and malformed JSON return stable errors without leaking stack traces.

## Related Files

- New `sidecars/hy-mt/server.py` and focused protocol/install/model modules.
- `sidecars/hy-mt/pyproject.toml`, `uv.lock`, PyInstaller spec, README, and tests.
- New `sidecars/hy-mt/bundle/.gitkeep`; generated runtime artifacts remain ignored except the intended release bundle convention.
- Product model files live under app-local data and are never committed.

## Implementation Steps

1. Refactor Phase 09 prompt/generation/device code into testable production modules without changing approved outputs.
2. Define constants for model ID, immutable revision, protocol version, allowlisted paths, exact sizes/digests, and runtime/model version.
3. Implement install/check modes with app-provided model root, separate first-install/update free-space preflight, version-scoped staging, resume, progress NDJSON, exact validation, symlink/path-containment checks, manifest creation, and atomic activation while retaining the last verified version until swap succeeds.
4. Store `License.txt`, notice, model/runtime metadata, and verification timestamps with the active manifest.
5. Implement offline serve startup, strict environment controls, safetensors-only `trust_remote_code=False` loading, `local_files_only=True`, device/dtype selection, preload/warmup, and the ready event.
6. Add a reader thread, single generation worker, cancellation event/stopping criterion, suffix-only decode, result validation, and bounded structured errors.
7. Ensure stdout is protocol-only and stderr is concise/sanitized; disable telemetry and reject tokens/network configuration in serve mode.
8. Create one-folder builds on the development OS and a test harness that drives the bundled executable, not only source Python; run inference with egress blocked and no Hub credentials.

## Todo

- [ ] Immutable manifest and all file hashes are pinned.
- [ ] Interrupted/corrupt installs never activate.
- [ ] Serve mode translates with network disabled.
- [ ] NDJSON protocol and stable error catalog are tested.
- [ ] Symlink/path escape, oversized line, and stdout-pollution cases are rejected.
- [ ] Cooperative and forced cancellation paths are tested.
- [ ] Bundled executable passes offline smoke tests.

## Risks

- `snapshot_download(local_dir=...)` metadata can create unexpected duplicate storage. Keep one product-owned local directory and measure it.
- Model download recovery can leave stale staging data. Version staging paths and make repair idempotent.
- PyInstaller may miss Transformers dynamic imports/native libraries. Smoke the bundled executable in CI/on target hardware.
- Reading cancel messages while generating requires concurrency discipline. Keep one generation worker and a narrow thread-safe cancellation signal.

## Success Criteria

- Every install state (`not_installed`, `downloading`, `verifying`, `installed`, `paused`, `repair_needed`, `error`) is reproducible and idempotent.
- An interrupted or tampered download is resumed/repaired and cannot be loaded before full verification.
- The bundled sidecar emits a verified ready event, translates the test corpus with firewall/egress disabled, cancels an active request, and exits on parent-pipe EOF.
- No meeting text, Hub token, raw exception, or unrestricted filesystem path appears in primary protocol errors/logs.
