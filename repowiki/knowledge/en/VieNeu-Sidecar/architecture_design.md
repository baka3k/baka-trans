# VieNeu sidecar architecture

The bridge has two modes: installation verifies pinned artifact versions and checksums; serving loads the model and accepts authenticated health, voice-list, and synthesis requests. It serializes inference behind a lock and returns WAV PCM audio.

```mermaid
graph LR
  Rust[Rust VieNeu manager] -->|authenticated loopback HTTP| Bridge[Python bridge]
  Bridge --> Validate[Manifest and SHA-256 validation]
  Bridge --> Engine[ONNX VieNeu engine]
  Engine --> Wav[WAV response]
```
