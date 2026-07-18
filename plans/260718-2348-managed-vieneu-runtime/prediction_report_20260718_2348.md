# Managed VieNeu Runtime Prediction — 2026-07-18

## Summary

- Depth: deep
- Verdict: **CAUTION**
- Proposal: bundle a one-folder VieNeu runtime, manage revision-pinned model installation in app-local data, and own the authenticated loopback process lifecycle from Tauri.

The design fits the existing TTS adapter and local-model setup flow, but only if runtime/model integrity, offline behavior, process ownership, and installation states are first-class contracts. System TTS remains the safe default and fallback.

## Agreements

- Use a lazy, long-lived one-folder sidecar; do not use PyInstaller one-file extraction.
- Download only an exact allowlist at pinned Hugging Face revisions, verify size and SHA-256, then atomically activate a versioned install.
- Pass explicit backbone and codec directories and force offline mode during inference.
- Bind an ephemeral loopback port inside the child, require a per-process token on every endpoint, and verify a startup nonce.
- Keep the child alive through an inherited stdin pipe so parent death terminates the sidecar.
- Replace the bridge URL field with a backend-owned setup/runtime state machine.
- Lazy-start only when VieNeu is selected, tested, previewed, or needed by a session.

## Conflicts

| Topic | Architect | Security | Performance | UX | Devil's Advocate | Resolution |
| --- | --- | --- | --- | --- | --- | --- |
| Model delivery | App-local versioned install | Hash every artifact and pin revisions | Avoid full snapshots and double disk | Show size, phases, pause/resume | Bundling model would be simpler offline | Download exact model payload separately; keep installer smaller and preserve offline use after setup. |
| RPC discovery | Manager-owned connection | Ephemeral port, token, nonce, pipe | Reuse one HTTP client | Hide infrastructure from users | Fixed port is simpler | Child binds port 0 and reports readiness over stdout; complexity is justified by spoofing and collision risks. |
| Startup | Managed state object | Verify runtime before execution | Lazy start and bounded restart | Explicit Loading/Recovering states | Starting at app boot is simpler | Lazy start prevents RAM/CPU regression for System TTS users. |
| Synthesis transport | Preserve current TTS boundary initially | Bound requests and redact errors | Streaming is needed for best latency/cancel | Avoid stale queued speech | Streaming expands scope substantially | Keep full-WAV transport in this phase; retain bounded queue/error behavior and record streaming as follow-up technical debt. |

## Risk Summary

| Risk | Severity | Persona | Mitigation |
| --- | --- | --- | --- |
| Tampered runtime/native DLL | Critical | Security | One-folder runtime manifest; build-time manifest digest; installer signing; never execute from model cache. |
| Hidden network fallback | High | Security/Performance | Explicit graph+codec directories, pinned files, `HF_HUB_OFFLINE=1`, offline integration test. |
| Local RPC spoofing | High | Security | Token and nonce delivered over inherited pipes; bearer auth on health, voices, and synthesis. |
| CPU/RAM contention with Whisper/Ollama | High | Performance | Default 2–4 ONNX threads, lazy load, bounded restart, benchmark before release. |
| Setup deadlock before voices exist | High | UX | Installation state is independent of translation config validation; voice selection becomes available only after Ready. |
| Partial/corrupt download | Medium | Security/UX | Persistent staging directory, resume through HF client, static hash/size verification, atomic rename, Repair state. |
| Orphan/crash-loop sidecar | Medium | Security/UX | Parent pipe watchdog, retained child handle, exponential bounded restart, explicit Recovering/Error state. |
| Installer/AV footprint | Medium | Performance/Devil | One-folder build per platform; release script builds and validates runtime before bundling. |

## Persona Details

### Architect

- Keep lifecycle/download code in a dedicated `vieneu` backend module; do not expand `tts.rs` into a process manager.
- Pass `AppHandle` through TTS readiness and synthesis paths so the managed state is explicit rather than global.
- Reuse the existing local model progress-event pattern, but introduce a richer VieNeu runtime status contract.
- Preserve System TTS and the 24 kHz playback boundary to constrain this phase.

### Security

- Treat the runtime and ONNX artifacts as executable supply-chain inputs.
- Require pinned repository commits, exact paths, sizes, SHA-256 values, atomic activation, and no remote-code execution.
- Do not pass secrets on argv or expose them to the frontend; authenticate every endpoint including health.
- Redact Python exception details from primary errors and logs.

### Performance

- Current Python environment is roughly 305 MiB; required preset model+codec payload is roughly 256 MiB without cloning artifacts.
- Avoid eager app startup and cap ONNX threads to prevent oversubscribing Whisper and Ollama.
- Full-response synthesis remains a latency limitation; chunked 48 kHz streaming is a future phase.

### UX

- Use visible phases: `not_installed`, `downloading`, `verifying`, `starting`, `loading`, `ready`, `recovering`, `repair_needed`, and `error`.
- Present aggregate bytes and phase text, not infrastructure paths or ports.
- Preserve partial downloads when paused/interrupted; keep setup independent of Save/Test validation.
- Announce milestones rather than every progress update in live regions.

### Devil's Advocate

- The riskiest assumption is that PyInstaller one-folder output behaves consistently across Windows and macOS antivirus/signing environments.
- A simpler managed-`uv` bootstrap would reduce packaging work, but still depends on an external Python toolchain and does not meet the user's autonomy goal.
- Worst case: runtime is quarantined, model consumes substantial disk, and the machine pages under Whisper+Ollama+TTS. System TTS fallback and capability/error states limit the blast radius.

## Recommendations

1. Implement pinned preset-only model installation first; omit voice cloning/denoiser artifacts.
2. Build an authenticated, pipe-owned child lifecycle before connecting it to UI actions.
3. Make the runtime status backend-authoritative and test state transitions separately from the settings form.
4. Add one-folder packaging scripts and wire release builds to fail if the sidecar artifact/manifest is absent.
5. Keep streaming and GPU support out of this phase; record both as explicit follow-ups.

## Next Step

Proceed under CAUTION with the mitigations above. Stop and redesign if the sidecar cannot initialize with all network access disabled after managed installation.
