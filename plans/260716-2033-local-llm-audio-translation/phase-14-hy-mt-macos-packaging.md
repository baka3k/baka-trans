# Phase 14: HY-MT macOS Packaging and Release Validation

## Context

The source/development sidecar works on Apple M5, but internal users must not install Python or recreate a virtual environment. This phase adds a reproducible macOS arm64 one-folder runtime to the Tauri app and validates signing, notarization, offline inference, and hardware behavior.

## Requirements

- Build the sidecar on macOS arm64 from the frozen dependency lock.
- Bundle the runtime, not the 4.09 GB model; model installation remains app-managed.
- Make release checks fail when the runtime or its manifest is absent/stale.
- Sign every nested executable/dylib/framework in the correct inside-out order before signing the app.
- Notarization/stapling and Gatekeeper checks cover the final DMG.
- A clean macOS user account can install the model, translate offline, cancel, restart, and use selected TTS/output routing without Python/Ollama.

## Related Files

- New `scripts/build-hy-mt-sidecar-macos.sh`
- `sidecars/hy-mt/bridge.spec`, bundle manifest and build docs
- `src-tauri/tauri.conf.json`, macOS bundle configuration/entitlements if required
- `scripts/release-macos.sh`, `package.json`
- `docs/RELEASE_GUIDE.md`, README/user guide and implementation validation report.

## Implementation Steps

1. Produce a clean same-platform one-folder build and a deterministic manifest of expected executable/native-library files and versions.
2. Add bundle resources under an HY-MT-specific destination without colliding with VieNeu resources.
3. Update release check to build the sidecar before Tauri, validate bundle contents/architecture, run the bundled offline smoke test, and fail on stale/missing artifacts.
4. Add nested signing/entitlement handling for Python/PyTorch native content, then run existing app/DMG codesign, notarization, stapler, and `hdiutil` verification.
5. Measure final app/DMG and installed runtime/model sizes, cold start, warm translation, and memory pressure against Phase 09 evidence.
6. Validate install, interrupted/resumed repair, offline translation, cancellation, restart, selected-output playback, and app shutdown on a clean account.
7. Document the internal build/update/rollback procedure and exact artifact hashes.
8. Generate and bundle an SBOM or complete third-party notice set for the Python, PyTorch, Transformers, tokenizer, model, and native runtime dependencies.

## Todo

- [ ] macOS arm64 sidecar bundle is reproducible and manifested.
- [ ] Release checks build/smoke/verify it before Tauri packaging.
- [ ] Nested signing, notarization, stapling, and Gatekeeper checks pass.
- [ ] Clean-account model install and offline session pass.
- [ ] Size/performance/hardware evidence is recorded.
- [ ] Model/runtime licenses, notice, and dependency inventory are bundled and reviewable.

## Risks

- PyTorch native libraries can invalidate app signing if modified after signing. Freeze bundle contents before inside-out signing.
- Runtime size can make DMG distribution/update impractical. Record the measured size and retain Ollama-only internal builds as a fallback profile if needed.
- MPS entitlements/native loading may differ inside a signed app. Test the packaged executable, not only development mode.

## Success Criteria

- The signed/notarized DMG installs and runs HY-MT on Apple Silicon without system Python, development tools, Ollama, or network access after model installation.
- Release automation fails early for missing/wrong-architecture/stale/unsigned HY-MT runtime content.
- Packaged latency/memory/quality remains within the Phase 09 approved envelope and selected audio routing works on real hardware.
