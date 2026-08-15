# Phase 16.1: Hy-MT2 Gate and Sidecar

Pin `tencent/Hy-MT2-1.8B` at `9a341cd1b679d3efd23b46e847b01745a71ed792`.
Upgrade only to an exact Transformers release that loads the model with
`trust_remote_code=False`; otherwise stop and vendor/audit the required code
before proceeding. Update the managed manifest, protocol identity, prompt
template, PyInstaller build, and tests.

Record: hash verification, MPS device/dtype, network-denied inference,
corpus/bilingual quality, API baseline, cancellation, soak, combined
Whisper/TTS memory, and bundled offline smoke. A CAUTION may develop isolated
infrastructure but cannot enable live-session selection.

## Exit criteria

- Exact model/runtime inputs and all loaded files are verified.
- A report records GO/CAUTION/STOP with raw evidence.
- The sidecar rejects wrong identity, symlinks, tokens, and network access.
