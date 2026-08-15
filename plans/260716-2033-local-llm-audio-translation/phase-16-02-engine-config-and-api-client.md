# Phase 16.2: Engine Config and OpenAI-Compatible Client

Introduce `LocalTranslationEngine::{HuggingfaceOffline, OpenaiCompatible}` and
schema v3. Legacy local JSON is atomically backed up and migrated to offline;
it never selects a remote service. Remove Ollama URL/model/keep-alive from the
persisted v3 contract.

Implement OpenAI Chat Completions only: normalize `/v1/chat/completions`, reject
URL credentials/fragments, require HTTPS outside loopback, disable redirects,
never send a key over HTTP, bound text/body/timeout, and parse only a nonempty
`choices[0].message.content`. Store the optional key in OS keychain or a
documented environment override, never JSON.

## Tests

- v1/v2 migrations, atomic recovery, no secret persistence.
- URL/TLS/redirect validation and request/response/error fixtures.
- Engine-specific config validation without silent fallback.
