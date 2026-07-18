# App-Managed VieNeu Runtime — 2026-07-18

## Context

The first VieNeu integration still required users to prepare Python, start a bridge manually, and configure its URL. The managed-runtime plan replaces that operational dependency with an application-owned install and process lifecycle while keeping System TTS as the default (`plans/260718-2348-managed-vieneu-runtime/plan.md:1`, `README.md:40`).

## Change

The Python bridge now downloads only an exact revision-pinned ONNX/int8 artifact allowlist, verifies every file by size and SHA-256 before activation, and opens the model with explicit local backbone/codec paths in offline mode (`sidecars/vieneu-tts/server.py:30`, `sidecars/vieneu-tts/server.py:46`, `sidecars/vieneu-tts/server.py:86`, `sidecars/vieneu-tts/server.py:172`, `sidecars/vieneu-tts/server.py:230`). Its loopback API requires a per-process bearer token, echoes a startup nonce, binds an ephemeral port, and exits when the parent stdin pipe closes (`sidecars/vieneu-tts/server.py:328`, `sidecars/vieneu-tts/server.py:403`, `sidecars/vieneu-tts/server.py:436`, `sidecars/vieneu-tts/server.py:470`).

A dedicated Rust manager owns app-local staging/final paths, resumable setup progress, repair-safe directory swaps, lazy process startup, authenticated health checks, and shutdown (`src-tauri/src/vieneu.rs:61`, `src-tauri/src/vieneu.rs:86`, `src-tauri/src/vieneu.rs:156`, `src-tauri/src/vieneu.rs:176`, `src-tauri/src/vieneu.rs:236`, `src-tauri/src/vieneu.rs:338`, `src-tauri/src/vieneu.rs:395`). Voice discovery and synthesis now obtain the private connection from that manager instead of accepting a user-supplied bridge URL (`src-tauri/src/tts.rs:20`, `src-tauri/src/tts.rs:45`, `src-tauri/src/tts.rs:78`, `src-tauri/src/tts.rs:88`).

Settings expose install, pause/resume, verification, start, restart, and repair states through a managed runtime card; the release flow builds and bundles a PyInstaller one-folder runtime (`src/components/settings/LocalLlmSettings.tsx:234`, `src/components/settings/LocalLlmSettings.tsx:491`, `src/app/MainApp.tsx:634`, `scripts/build-vieneu-sidecar.ps1:6`, `scripts/release-windows.ps1:30`, `src-tauri/tauri.conf.json:29`).

## Impact

**Risk level: high.** Users can opt into VieNeu without installing Python, running a terminal command, choosing a port, or downloading models during synthesis. The material risks are executable/native-runtime supply-chain integrity, roughly 244 MiB of model storage, antivirus quarantine of the one-folder bundle, CPU/RAM contention with Whisper and Ollama, and whole-response WAV latency. Pinned revisions plus artifact hashes, offline inference, authenticated loopback RPC, lazy startup, bounded requests, and an unaffected System TTS path reduce but do not eliminate those risks (`src-tauri/src/vieneu.rs:19`, `src-tauri/src/tts.rs:12`, `src-tauri/src/tts.rs:130`, `sidecars/vieneu-tts/server.py:231`, `README.md:40`).

## Decision

Use a target-platform PyInstaller one-folder sidecar with a separately downloaded, versioned model. This meets the autonomy goal while avoiding one-file extraction overhead and keeping the installer smaller than bundling model weights. A managed `uv` bootstrap was rejected because it still depends on an external Python toolchain; direct Rust inference, GPU support, model deletion/updates, macOS release wiring, and streaming PCM remain deferred (`sidecars/vieneu-tts/README.md:20`, `plans/260718-2348-managed-vieneu-runtime/plan.md:14`).

## References

- plan: `plans/260718-2348-managed-vieneu-runtime/plan.md:1`
- prediction: `plans/260718-2348-managed-vieneu-runtime/prediction_report_20260718_2348.md:1`
- bridge runtime: `sidecars/vieneu-tts/server.py:30`
- lifecycle manager: `src-tauri/src/vieneu.rs:61`
- settings integration: `src/components/settings/LocalLlmSettings.tsx:491`
- baseline commit: `91ea5ec45a51ac48dca8daee54c0700d191297ca`
