# Phase 15: HY-MT Windows Packaging and Release Validation

## Context

PyInstaller is target-platform specific and the macOS bundle cannot be reused on Windows. This phase builds the frozen HY-MT runtime on Windows, establishes the supported device policy, integrates it with NSIS release checks, and validates antivirus/installer/hardware behavior.

## Requirements

- Build on supported Windows x64 using the frozen dependency/model/protocol versions.
- CPU is the compatibility baseline. CUDA is enabled only after capability and bundled-runtime tests; never assume GPU from `useGpu` or `device_map="auto"`.
- Bundle runtime but not model weights; reuse the same verified app-managed model lifecycle.
- Windows release builds fail if the sidecar is missing, wrong architecture, stale, or fails an offline smoke test.
- Validate Windows Defender/SmartScreen/NSIS install, app-private model write/update/repair, process cleanup, and selected playback hardware.
- Preserve existing VieNeu sidecar build and resources.
- Run a local production-rate readiness benchmark; do not report HY-MT Ready on hardware that misses the Phase 09-approved envelope.

## Related Files

- New `scripts/build-hy-mt-sidecar.ps1`
- `sidecars/hy-mt/bridge.spec`, bundle/manifest output
- `scripts/release-windows.ps1`, `src-tauri/tauri.conf.json`, `package.json`
- `docs/WINDOWS_RELEASE_GUIDE.md`, `docs/WINDOWS_TEAMS_USER_GUIDE.md`, README and validation report.

## Implementation Steps

1. Recreate the locked environment and build one-folder output on clean Windows x64 CI/hardware.
2. Validate CPU load/inference first; probe CUDA availability/version/dtype explicitly and decide whether CUDA libraries are bundled, externally required, or deferred.
3. Add HY-MT bundle resources alongside VieNeu and make the Windows release script build/manifest/smoke both runtimes independently.
4. Run the bundled executable offline before Tauri/NSIS build; verify architecture, runtime/model/protocol versions, and clean process exit.
5. Build/install/uninstall NSIS under Defender, record quarantine/SmartScreen findings, and ensure repair/restart errors remain actionable.
6. Validate model install/resume/repair, CPU and approved CUDA modes, cancellation, 30-minute session, app shutdown, WASAPI loopback input, and selected TTS/output device on real Windows hardware.
7. Record installer/runtime/model sizes, cold/warm latency, memory, CPU/GPU utilization, and minimum supported hardware guidance.
8. Generate and bundle an SBOM or complete third-party notice set for the Python, PyTorch, Transformers, tokenizer, model, and native runtime dependencies.

## Todo

- [ ] Clean Windows one-folder bundle is reproducible.
- [ ] CPU baseline and CUDA decision are documented with measurements.
- [ ] Release script validates both HY-MT and VieNeu sidecars.
- [ ] NSIS/Defender/SmartScreen evidence is recorded.
- [ ] Real Windows audio and sustained-session validation passes.
- [ ] Runtime benchmark gates readiness and licenses/notices are bundled.

## Risks

- Bundling CUDA libraries can make the runtime extremely large; CPU-first support may be preferable for the internal installer.
- Windows PyTorch/Transformers/PyInstaller dependencies can trigger antivirus false positives. Use signed artifacts, stable manifests, and pre-release Defender checks.
- CPU inference may not keep pace on lower-end machines. Gate session start from a local benchmark and document the minimum tier.
- Resource paths can collide with the existing VieNeu bundle. Use independent source/destination roots and release assertions.

## Success Criteria

- The NSIS-installed app runs HY-MT without Python/Ollama and translates offline after managed model installation.
- Supported CPU and any approved CUDA configurations are explicit, benchmarked, and capability-gated; hardware below the production-rate envelope never reports Ready and receives an actionable switch-to-Ollama path.
- Defender/installer checks, 30-minute realtime session, cancellation/shutdown, WASAPI capture, TTS, and selected-output routing pass on documented hardware.
- Existing VieNeu build/runtime and all Windows release checks remain intact.
