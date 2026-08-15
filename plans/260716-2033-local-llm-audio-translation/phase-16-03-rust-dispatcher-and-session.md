# Phase 16.3: Rust Dispatcher and Session Integration

Replace the worker's `OllamaClient` field with one engine-neutral client.
`HyMtManager` owns the bundled sidecar child, verified ready identity, NDJSON
pipes, one-flight lock, timeout/cancel/restart/unload, and app shutdown.
`OpenAiCompatibleClient` owns its bounded request. Neither engine retries into
the other.

Preserve existing Whisper segmentation, transcript IDs/revisions, bounded TTS
queue, playback routing, stop/pause generation invalidation, and cloud paths.

## Tests

- Fake-sidecar ready mismatch, malformed frame, timeout, crash, cancellation,
  stale ID, and restart fuse.
- Mock API success/error/empty response, queue pressure, and no late TTS.
