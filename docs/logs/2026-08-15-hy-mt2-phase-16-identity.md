# Hy-MT2 Phase 16.1: Runtime Identity and Sidecar Verification — 2026-08-15

## Context

Phase 16 supersedes all Ollama assumptions in Phases 11–13. The product owner
removed Ollama from the product direction; local translation must use either
the managed offline `tencent/Hy-MT2-1.8B` runtime or a user-configured
OpenAI-compatible Chat Completions API
(`plans/260716-2033-local-llm-audio-translation/phase-16-hy-mt2-openai-compatible-migration.md:1`).
Phase 16.1 is the first sub-phase: pin the Hy-MT2 model, upgrade to an exact
Transformers release with `trust_remote_code=False`, update the managed
manifest, protocol identity, prompt template, PyInstaller build, and tests.

## Change

The sidecar now declares a complete `RUNTIME_IDENTITY` including
`trustRemoteCode: false` as an explicit constant
(`sidecars/hy-mt/hy_mt_poc/constants.py:28`). The install manifest records
`trustRemoteCode` alongside model ID, revision, and runtime version
(`sidecars/hy-mt/hy_mt_poc/lifecycle.py:121`). The ready message carries the
full identity declaration so the Rust parent can verify exact sidecar identity
before sending any translation request
(`sidecars/hy-mt/hy_mt_poc/server.py:62`). Runtime version bumped from
`0.1.0` to `0.2.0` to signal the protocol-level identity addition.

Seven new identity tests verify: ready message carries exact pinned identity,
runtime identity constant matches pinned model, manifest declares
`trustRemoteCode: false`, validate rejects wrong model ID, validate rejects
wrong revision, runner source code never enables `trust_remote_code`, and
hardened environment blocks all three Hub token names
(`sidecars/hy-mt/tests/test_identity.py:1`).

PyInstaller spec now lists all eleven `hy_mt_poc` submodules explicitly
instead of only four, preventing silent import failures in the packaged binary
(`sidecars/hy-mt/hy-mt-poc.spec:14`).

## Impact

**Risk level: low.** This is a verification and hardening pass over the
existing Phase 10 sidecar. No behavioral changes to translation, cancellation,
or network denial. All 38 tests pass (31 existing + 7 new). The identity
declaration is additive — existing Rust-side consumers can ignore the new
fields until the Phase 16.3 dispatcher verifies them.

## Decision

Declare identity explicitly in constants rather than deriving it from scattered
references. This makes the identity trivially auditable by both tests and the
Rust parent. Bumped runtime version to `0.2.0` to give the parent a clear
signal that the ready message shape changed. Chose `TRUST_REMOTE_CODE = False`
as a named constant rather than an inline literal so the audit test can verify
its value without parsing the runner source heuristically.

## References

- [Phase 16.1 plan](../../plans/260716-2033-local-llm-audio-translation/phase-16-01-hy-mt2-gate-and-sidecar.md)
- [Phase 16 umbrella](../../plans/260716-2033-local-llm-audio-translation/phase-16-hy-mt2-openai-compatible-migration.md)
- commit: `d44a1bb`
