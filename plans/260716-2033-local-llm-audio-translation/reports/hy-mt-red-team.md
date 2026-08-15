---
type: red-team
date: 2026-08-15
verdict: caution
---

# HY-MT Managed Runtime Red-Team Review

## Verdict

**CAUTION — proceed with Phase 09 only.** The architecture preserves the existing realtime pipeline and puts the risky runtime behind a hard feasibility gate. Phases 10-15 are acceptable only if Phase 09 records a GO and the required amendments below remain release criteria.

## Attack Summary

The plan was challenged against runtime ownership, model supply chain, data privacy, cancellation races, migration compatibility, resource contention, packaging/signing, Windows viability, and internal-use licensing.

## Findings

| ID | Severity | Finding | Failure Mode | Required Mitigation | Disposition |
| --- | --- | --- | --- | --- | --- |
| RT-01 | Critical | Product integration could start before feasibility is proven. | The repo gains a multi-hundred-MiB runtime and 4.09 GB model flow that cannot keep pace with utterances. | Phase 09 is a hard GO gate; STOP closes Phases 10-15 and leaves Ollama active. | Accepted in plan. |
| RT-02 | High | Mutable/remote model code or unverified paths could execute inside the sidecar. | Compromised Hub content, symlink escape, or implicit remote code runs with user permissions. | Immutable revision, exact allowlist/hash/size, safetensors only, `trust_remote_code=False`, reject symlink/path escape, atomic same-filesystem activation. | Amendment required in Phase 10. |
| RT-03 | High | Offline environment variables alone do not prove transcript privacy. | A dependency or accidental fallback makes a network call during translation. | Use only verified local paths, `local_files_only=True`, no Hub token/listening port, and firewall/no-network integration tests while translating. | Accepted; strengthen test wording. |
| RT-04 | High | Cancellation can race with a late sidecar result. | Stop returns to Idle, then a translation/TTS event mutates the finished session. | Request ID + process generation + session generation checks; cooperative stopping criterion; kill after grace period; reject every late response before store/event/TTS. | Accepted in Phases 10, 11, 13. |
| RT-05 | High | Config migration/provider renaming could break existing internal installs. | Existing JSON rejects schema v2, loses Ollama/TTS settings, or routes to HY unexpectedly. | Keep serialized provider, add engine enum, migrate real v1 fixtures to `ollama`, atomically rewrite, preserve every field. | Accepted in Phase 11. |
| RT-06 | High | PyTorch sidecar packaging can be too large or fail target security controls. | DMG/NSIS grows excessively; macOS signing/notarization or Defender quarantines native libraries. | Package spike in Phase 09; target-native builds; manifest all binaries; nested signing; bundled-executable smoke; Defender/installer gates. | Accepted in Phases 09, 14, 15. |
| RT-07 | High | Windows CPU baseline may not be realtime. | Queue fills on supported Windows hardware despite functional inference. | Benchmark before readiness; document minimum tier; reject realtime start or offer explicit Ollama switch if local benchmark misses the approved envelope. | Amendment required in Phase 15/readiness. |
| RT-08 | Medium | Model/runtime updates can double disk use or activate partial content. | Staging plus active models exhaust disk; repair destroys a working version. | Preflight first-install/update space separately, retain current active version until verified replacement is ready, then atomic swap; cleanup only version-scoped paths. | Accepted; clarify Phase 10. |
| RT-09 | Medium | Bundled dependency licenses/notices are incomplete. | Internal distribution lacks required Tencent or Python/PyTorch/Transformers third-party notices. | Generate/store runtime SBOM or dependency notice set; include model license/notice and internal territory/use instructions. | Amendment required in packaging phases. |
| RT-10 | Medium | Silent cross-engine fallback duplicates or changes an utterance. | HY times out, Ollama returns later under the same transcript, producing stale/duplicate output. | Never auto-fallback per request; expose explicit restart/switch, one terminal snapshot per ID. | Accepted throughout plan. |
| RT-11 | Medium | stdout logs can corrupt NDJSON protocol or leak text. | Parser desynchronizes or transcript appears in diagnostics. | Protocol-only stdout, sanitized stderr, max line length, strict parser, no text logging, malformed-line fuse. | Accepted; add parser bounds test. |
| RT-12 | Medium | Internal Vietnam use is treated as a universal license exemption. | A laptop or internal build is used/distributed in an excluded territory. | Record operational territory restriction and acceptance; do not describe the model license as permissive; revisit before deployment outside Vietnam. | Amendment required in docs/success criteria. |

## Adversarial Scenarios

1. **Stop at token 20:** send cancel, immediately stop the session, force the sidecar to delay its result, then verify no transcript/TTS mutation and no child leak.
2. **Corrupt active update:** keep a valid active model, interrupt a new revision download, replace a small config with a symlink, and verify the old active version remains usable while repair rejects staging.
3. **Protocol pollution:** emit a warning on stdout, oversized JSON, wrong request ID, and wrong protocol version; the manager must fuse/restart without associating output to an utterance.
4. **No-network inference:** block all egress after install, remove Hub credentials, and run warm/cold translation. Any attempted fetch fails the release gate.
5. **Memory contention:** run Whisper, HY-MT, and VieNeu/System TTS for 30 minutes on M5; queue capacity remains four and memory pressure stays below the approved threshold.
6. **Slow Windows CPU:** run the readiness benchmark on minimum hardware; the UI must prevent false “realtime ready” state and offer explicit Ollama recovery.
7. **Migration fixture:** load a real schema-v1 config containing non-default Ollama, Whisper, segmentation, VieNeu, and audio fields; save v2 and compare all values.

## Required Plan Amendments

1. Add `trust_remote_code=False`, safetensors-only loading, symlink/path containment checks, max NDJSON line bounds, and firewall/no-network testing to Phase 10.
2. Add runtime SBOM/third-party notices to macOS and Windows packaging gates.
3. Make the Windows local benchmark part of readiness and prohibit “Ready” if the approved production-rate envelope is missed.
4. State the internal Vietnam territory assumption operationally; internal use does not authorize use in excluded territories.

## Conclusion

No unmitigatable architecture blocker was found. The plan remains CAUTION because MPS performance, one-folder bundle size/signing, and Windows CPU viability are empirical unknowns. The Phase 09 decision gate correctly contains those risks.
