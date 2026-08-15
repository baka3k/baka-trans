# Managed HY-MT Sidecar Under Limited CAUTION — 2026-08-15

## Context

The owner reopened Phase 10 only under **CAUTION** so the failed-quality HY-MT candidate can be installed and process-isolated for further evaluation. This is not a quality GO, does not alter Ollama or live routing, and leaves Phases 11-15 closed (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:11`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:25`, `plans/260716-2033-local-llm-audio-translation/phase-10-hy-mt-sidecar-and-model-lifecycle.md:5`).

## Change

The managed sidecar now pins the model revision, artifact sizes, and SHA-256 values (`sidecars/hy-mt/hy_mt_poc/constants.py:28`, `sidecars/hy-mt/hy_mt_poc/constants.py:39`). Its lifecycle stages resumable downloads, rejects symlinks and invalid hashes, verifies the complete allowlist, and atomically activates only a trusted model (`sidecars/hy-mt/hy_mt_poc/lifecycle.py:87`, `sidecars/hy-mt/hy_mt_poc/lifecycle.py:131`). A versioned, size-bounded NDJSON protocol validates Japanese-to-Vietnamese translate/cancel requests (`sidecars/hy-mt/hy_mt_poc/protocol.py:30`, `sidecars/hy-mt/hy_mt_poc/protocol.py:58`), while serve mode rejects Hub credentials, blocks socket use, keeps one model resident, supports cooperative cancellation, emits sanitized errors, and exits on parent-pipe EOF (`sidecars/hy-mt/hy_mt_poc/server.py:20`, `sidecars/hy-mt/hy_mt_poc/server.py:72`, `sidecars/hy-mt/hy_mt_poc/server.py:122`). The PyInstaller specification packages the managed server entry point (`sidecars/hy-mt/hy-mt-poc.spec:8`, `sidecars/hy-mt/hy-mt-poc.spec:29`).

## Impact

**Risk level: high.** Phase 10 establishes an app-manageable, offline sidecar boundary without exposing the candidate to users or production sessions. The model still failed the quality gate, and active-model network-disabled translation remains pending on a target host (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:40`, `plans/260716-2033-local-llm-audio-translation/phase-10-hy-mt-sidecar-and-model-lifecycle.md:64`). Large downloads, native runtime packaging, and cancellation supervision remain operational risks.

## Decision

Continue only the isolated Phase 10 evaluation surface. Keep Ollama as the unchanged production/default path, and keep Phases 11-15 closed until a fresh gate supplies bilingual human acceptance, an installed Ollama baseline, and combined Whisper/HY-MT/TTS memory evidence (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:315`).

## References

- [Phase 10 plan](../../plans/260716-2033-local-llm-audio-translation/phase-10-hy-mt-sidecar-and-model-lifecycle.md)
- [HY-MT M5 gate report](../../plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md)
- commit: `e3bb31dc5bd5ae0b35c3a1a59afacd83fcfb9bc4`
