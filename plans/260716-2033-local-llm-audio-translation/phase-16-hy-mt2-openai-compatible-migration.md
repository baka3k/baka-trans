# Phase 16: Hy-MT2 Offline and OpenAI-Compatible Translation Migration

## Execution Breakdown

This umbrella is implemented in dependency order:

1. [16.1 Hy-MT2 Gate and Sidecar](phase-16-01-hy-mt2-gate-and-sidecar.md)
2. [16.2 Engine Config and API Client](phase-16-02-engine-config-and-api-client.md)
3. [16.3 Rust Dispatcher and Session](phase-16-03-rust-dispatcher-and-session.md)
4. [16.4 Settings and Cutover](phase-16-04-settings-and-cutover.md)

## Context

The product owner has removed Ollama from the intended product direction. Local
translation must instead use either the app-managed offline
`tencent/Hy-MT2-1.8B` runtime or a user-configured OpenAI-compatible Chat
Completions API. This phase supersedes the Ollama assumptions in Phases 11-13.

## Verified Inputs

- Model: `tencent/Hy-MT2-1.8B`
- Immutable revision: `9a341cd1b679d3efd23b46e847b01745a71ed792`
- License: Apache-2.0
- Official runtime guidance: `transformers>=5.6.0`, direct model loading, and
  no default system prompt.
- Primary sources: <https://huggingface.co/tencent/Hy-MT2-1.8B> and
  <https://huggingface.co/docs/huggingface_hub/guides/download>.

## Target Contract

```text
Local Whisper -> selected translation engine -> existing transcript/TTS flow

huggingface_offline
  Rust HyMtManager -> bundled Python sidecar -> verified local Hy-MT2 files

openai_compatible
  Rust OpenAiCompatibleClient -> POST {baseUrl}/chat/completions
```

`translationEngine` is required and serializes as `huggingface_offline` or
`openai_compatible`. Config migration must map the retired legacy local value
to `huggingface_offline`, never to a network API. It writes an atomic, one-time
legacy-config backup before removing old Ollama URL/model/keep-alive fields, so
users can recover settings while the migration is being validated.

## Requirements

1. Replace all product Ollama client, config, UI, provider labels, tests, and
   smoke-test assumptions; retain only a read-only migration path for old JSON.
2. Pin Hy-MT2 files by revision, exact size, and SHA-256. Installer mode alone
   may reach Hugging Face; serve mode is local-files-only, credential-free, and
   network-denied.
3. Upgrade the sidecar to an exact, audited Transformers release that supports
   this model. Prefer `trust_remote_code=False`; do not enable remote model code
   unless it is first vendored, reviewed, pinned, and launched under OS-enforced
   process/filesystem/network sandboxing.
4. Implement an engine-neutral Rust dispatch boundary with no silent fallback.
   Sidecar identity must declare Hy-MT2 model ID/revision and be rejected if it
   differs.
5. Add an accessible Translation Engine setting. Offline shows fixed Hy-MT2
   runtime state/actions. API mode shows base URL, model, optional API key, and
   generation controls.
6. Store the API key only in the OS keychain (or a documented environment
   override); never serialize it to the local translation config or display it
   after save.
7. Normalize OpenAI-compatible base URLs to `/v1/chat/completions`, bound
   requests/responses/timeouts, redact server errors, and accept only non-empty
   `choices[0].message.content` text. HTTPS is required outside loopback; URL
   credentials/fragments are forbidden; redirects are disabled; HTTP never
   carries an API key.
8. Before the first non-loopback API translation, show a clear confirmation
   that meeting text is sent to the chosen endpoint and retain the accepted
   endpoint origin only (not text or key) as the acknowledgement record.
8. Connect both engines to the existing ordered Whisper → transcript → TTS
   worker. Stop/pause/config generation changes cancel in-flight work and never
   permit a late result/TTS output.

## Implementation Order

0. Run a dedicated Hy-MT2 gate on the fixed JA→VI corpus: pin/runtime audit,
   bilingual quality review, current API baseline comparison, M5 latency/soak,
   combined Whisper/TTS memory, cancellation, offline inference, and package
   smoke. Do not enable it for live sessions without a recorded GO or an
   explicit owner CAUTION limited to an isolated evaluation path.
1. Update the sidecar pins, prompt policy, manifest verification, protocol
   identity, tests, README, and one-folder build for Hy-MT2.
2. Introduce `LocalTranslationEngine`, schema-v3 migration, keychain API-key
   functions, OpenAI-compatible client/payload/response parsing, and tests.
3. Add the Rust manager/dispatcher and replace the worker's direct Ollama
   dependency; test with fake sidecar and mock HTTP server.
4. Replace the React/OUI config with engine selection and conditional cards;
   test persistence, validation, keyboard/accessibility, and secret masking.
5. Run the sidecar offline smoke test using an active managed Hy-MT2 model,
   then local pipeline regression tests for both engines.

## Acceptance Criteria

- No user-facing or executable Ollama code remains; the migration has a
  recoverable legacy-config backup and never maps a user to a network API.
- A dedicated Hy-MT2 gate records quality/bilingual acceptance, baseline,
  latency, soak, combined-memory, cancellation, offline, and package evidence
  before Hy-MT2 becomes selectable for live sessions.
- A migrated legacy config starts with offline Hy-MT2 selected and no API key.
- Offline sidecar rejects network, token, symlink, tamper, and wrong-identity
  cases; it translates JA→VI only after verified local loading.
- API engine sends OpenAI Chat Completions requests only to a normalized,
  redirect-disabled endpoint; it requires HTTPS outside loopback, never sends a
  key over HTTP, records user acknowledgement before remote transcript egress,
  and surfaces stable errors without transcript text or secret values.
- Switching engines persists independently relevant fields, invalidates stale
  readiness, and never changes an active session's engine.
- Existing cloud, Whisper, transcript, TTS, and audio-routing tests remain
  green.

## Risks and Mitigations

- Hy-MT2's official card currently specifies remote code. Do not execute it
  from the model root: use an audited native Transformers implementation or
  vendor/review it and enforce an OS sandbox before allowing serve mode.
- OpenAI-compatible services vary. Support the standard Chat Completions shape
  only and report unsupported responses explicitly.
- The other Ollama-removal work may arrive in this shared worktree. Rebase the
  migration onto those changes rather than restoring any removed code.
