# Phase 03 — Regression Matrix, Docs, and Cutover Evidence

Parent: [plan.md](plan.md)

Depends on: Phases 01-02.

## Objective

Prove the switch works both directions with no regression to Hy-MT2, cloud
modes, or persisted configs, and document the provider setup for users.

## Verification Matrix

| Scenario | Expected |
| --- | --- |
| Fresh config → DeepSeek preset + key → save, test, run session | Translations live; key only in keychain |
| Engine switch openai_compatible → huggingface_offline | Hy-MT2 path unchanged; stored key untouched; no keychain reads on the Hy-MT2 path |
| Engine switch back to openai_compatible | Key still present (status: Keychain); test passes without re-entering |
| Loopback endpoint, no key | Session runs (Ollama regression) |
| Non-loopback, no key | Start + test fail with `local_openai_api_key_missing` |
| `BAKA_TRANS_LOCAL_API_KEY` set | Status shows "Environment variable"; in-app save still updates keychain but env wins at load (documented) |
| Legacy v3→v4 config migration tests | Still green; no schema field changed |
| Cloud sessions (Google Live / OpenAI Realtime) | Untouched; existing cloud-key tests green |
| Stop mid-utterance with in-flight API request | Existing cancellation/error snapshot behavior unchanged |

Gates: `npm test`, `npm run build`, `cargo test`, `cargo fmt --check`,
`cargo clippy --all-targets -- -D warnings`.

## Documentation

- `README.md`
  - Local Whisper section: add "OpenAI-compatible providers (e.g. DeepSeek)"
    setup — preset, key field → OS keychain, `BAKA_TRANS_LOCAL_API_KEY`
    override, `deepseek-v4-flash` example model.
  - Error-code table: `local_openai_api_key_missing`, `credential_input_invalid`.
- `ARCHITECTURE.md`: local pipeline line already says "Hy-MT2 offline or
  OpenAI-compatible API" — extend with "API key in OS keychain" and the
  start-time gate.
- New `docs/logs/2026-09-06-openai-compatible-provider-completion.md` following
  the existing phase-log format: what existed, the gap found (save/delete
  commands absent despite the 16.2 log), what this plan added.

## Acceptance

- Full matrix executed with recorded results (this file or the phase log).
- No diff in `src-tauri/src/hy_mt.rs`, `src-tauri/src/ai/*` cloud modules, or
  `src-tauri/src/llm.rs`.
- A reviewer can set up DeepSeek from README alone in under two minutes.
