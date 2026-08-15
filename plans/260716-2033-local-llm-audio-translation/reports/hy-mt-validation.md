---
type: validation
date: 2026-08-15
verdict: go-phase-09
---

# HY-MT Managed Runtime Critical Validation

## Result

**GO to execute Phase 09; CAUTION for Phases 10-15.** The plan has enough information to run the feasibility gate without further product decisions. Unknown PyTorch/MPS behavior, bundle size, and Windows production rate are intentionally evidence produced by the planned gates, not assumptions hidden in implementation.

## Critical Questions

| # | Question | Validated Answer | Evidence / Plan Effect | Status |
| --- | --- | --- | --- | --- |
| 1 | What user outcome is required? | Internal users select managed HY-MT and translate locally without installing Python or Ollama; existing Ollama remains available. | User clarification plus Phase 09-13 scope. | Resolved |
| 2 | Is Hugging Face a hosted service in this design? | No. Hugging Face is model distribution during managed install; inference is local and offline. | Research/model card; Phase 10 network boundary. | Resolved |
| 3 | Must Ollama be removed immediately? | No. Existing configs migrate to `ollama`; HY-MT is opt-in until gates pass, with explicit switching only. | Scope decision and red-team RT-05/RT-10. | Resolved |
| 4 | Does the app depend on system Python? | No. Development uses an isolated locked environment; release uses target-native one-folder sidecars. | Phases 09, 14, 15. | Resolved |
| 5 | What is the initial language scope? | Japanese → Vietnamese, matching the current local product flow. Broader HY language exposure is deferred until Rust/TS validation tables agree. | Phase 12 and existing local workflow. | Resolved |
| 6 | Which generation settings ship? | Neither sample is assumed. Phase 09 compares greedy decoding and model defaults, then pins the evidence-backed policy. | Prediction/research and Phase 09. | Deliberately gated |
| 7 | Which macOS device/dtype ships? | Explicit MPS probe on Apple M5; dtype and fallback are pinned only after actual operation/quality/memory evidence. | Phase 09; red-team RT-01. | Deliberately gated |
| 8 | Which Windows acceleration ships? | CPU is compatibility baseline; CUDA is capability-tested and may be deferred if bundle/driver cost is unacceptable. Hardware below production rate is not Ready. | Phase 15; red-team RT-07. | Deliberately gated |
| 9 | How are weights trusted and updated? | Immutable revision, exact allowlist/size/hash, safetensors, no remote code, staged verification, atomic activation, manual reviewed updates. | Phases 10-11; red-team RT-02/RT-08. | Resolved |
| 10 | How is transcript privacy enforced? | No listening port; private inherited NDJSON; offline flags plus local-only loading and egress-blocked tests; no text logging. | Phases 10-13; red-team RT-03/RT-11. | Resolved |
| 11 | What happens on timeout/stop? | Cooperative stop criterion first; bounded grace; child termination and clean restart; reject late IDs/process/session generations. | Phases 10, 11, 13; red-team RT-04. | Resolved |
| 12 | What licenses apply for internal Vietnam use? | Keep Tencent license/notice and runtime dependency inventory; document that internal Vietnam scope does not authorize excluded-territory use. | Plan success criteria; Phases 14-15; red-team RT-09/RT-12. | Resolved for current scope |
| 13 | What is the implementation order? | Phase 09 hard gate → 10 sidecar → 11 Rust manager → 12 UI → 13 pipeline → independent macOS/Windows packaging. | Main phase table. | Resolved |
| 14 | What aborts the project? | Failed quality/production-rate/offline/memory/packaging feasibility in Phase 09 records STOP and leaves Ollama active without partial integration. | Phase 09 success criteria and plan handoff. | Resolved |

## Contract Validation

- **Architecture boundary:** Valid. Whisper and TTS remain Rust-owned; Python owns only model installation/inference.
- **Migration:** Valid. Serialized session provider remains stable; engine is a config-v2 addition with v1 fixtures.
- **Concurrency:** Valid with implementation tests. One translation worker and one sidecar generation request preserve order; cancellation still requires the planned reader-thread/stopping-criterion proof.
- **Security/privacy:** Valid with amendments. Safetensors-only, no remote code, managed paths, no port, offline mode, and egress tests cover the principal new surface.
- **Performance:** Unknown by design. Phase 09 and platform readiness gates prevent an unsupported claim.
- **UX:** Valid. Engine-specific settings avoid asking users for Python/cache/device-map details and retain explicit Ollama recovery.
- **Release:** Valid as phased work. Same-platform sidecar builds and platform-specific signing/AV checks are correctly separated.

## Remaining Evidence, Not Clarification Blockers

These values must be measured before later phases and must not be guessed during implementation:

1. Exact PyTorch version and approved MPS dtype/loading call.
2. M5 cold load, warm p50/p95, peak RSS, memory pressure, and 30-minute production rate.
3. One-folder runtime size and nested macOS signing/notarization behavior.
4. Minimum Windows CPU tier and whether CUDA runtime packaging is justified.
5. Final generation policy and human JA→VI quality comparison.
6. Exact hashes for all seven inference files and runtime dependency inventory.

## Validation Decision

The plan is implementable and internally consistent after red-team amendments. Start only [Phase 09](../phase-09-hy-mt-m5-poc-and-gate.md). Do not hydrate or begin Phase 10 implementation until the Phase 09 report records GO and updates the exact dependency/model pins in the plan.
