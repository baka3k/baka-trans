# Phase 16.4: Settings UX and Ollama Cutover

Replace the local settings' Ollama card with an accessible Translation Engine
selector. Offline mode shows fixed Hy-MT2 identity/install/readiness; API mode
shows URL, model, optional masked key, generation controls, and a remote-text
egress acknowledgement. Preserve inactive mode values without exposing secrets.

Remove executable/user-facing Ollama code, labels, command paths, tests, and
docs. Update provider serialization with a legacy alias/migration only. Run
frontend accessibility tests, Rust tests, offline package smoke, and cloud/
Whisper/TTS/audio regression checks.

## Cutover criteria

- No active Ollama code/path remains.
- Switching engines invalidates readiness and never changes an active session.
- Hy-MT2 live selection remains disabled until Phase 16.1 records GO or an
  explicit owner CAUTION accepts that limited exposure.
