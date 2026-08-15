# HY-MT M5 Feasibility Gate Stopped — 2026-08-15

## Context

Phase 09 was a hard gate for proving the pinned HY-MT candidate before any managed-runtime work could proceed; the production Ollama path and default had to remain unchanged meanwhile (`plans/260716-2033-local-llm-audio-translation/phase-09-hy-mt-m5-poc-and-gate.md:5`, `plans/260716-2033-local-llm-audio-translation/phase-09-hy-mt-m5-poc-and-gate.md:7`).

## Change

Commit `81309b51062792d66628517600b194765f62bc2b` added the isolated M5 POC and decision evidence. MPS/BF16 execution, network-denied offline inference, the 30-minute 360-request soak, and the one-folder package spike worked (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:14`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:188`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:201`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:221`). The quality gate failed, while the required current Ollama baseline, bilingual human sign-off, and combined Whisper/HY-MT/TTS memory evidence were unavailable (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:34`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:36`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:39`).

## Impact

**Risk level: high.** Advancing the failed candidate could ship wrong-language output and material changes to technical terms, names, or instructions without baseline, human, or combined-process validation (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:163`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:164`). No production Rust, Tauri, settings, engine-selection, or Ollama-default behavior changed (`plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:23`).

## Decision

Record **STOP**, keep Ollama active and unchanged, and close Phases 10-15. Passing MPS, offline, soak, and packaging checks does not override the failed quality criterion or missing required evidence; any future HY-MT attempt must begin as a newly reviewed gate (`plans/260716-2033-local-llm-audio-translation/phase-09-hy-mt-m5-poc-and-gate.md:73`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:43`, `plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md:304`).

## References

- [Phase 09 plan](../../plans/260716-2033-local-llm-audio-translation/phase-09-hy-mt-m5-poc-and-gate.md)
- [Phase 09 decision report](../../plans/260716-2033-local-llm-audio-translation/reports/hy-mt-m5-poc.md)
- commit: `81309b51062792d66628517600b194765f62bc2b`
