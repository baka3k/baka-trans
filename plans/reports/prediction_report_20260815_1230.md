---
type: prediction
date: 2026-08-15
depth: deep
verdict: caution
---

# Prediction: replace Ollama with direct Hugging Face HY-MT1.5 inference

## Proposal

After Whisper produces an utterance, replace the local Ollama `/api/chat` request with a locally managed Hugging Face Transformers runtime that loads `tencent/HY-MT1.5-1.8B` and generates the translation using its chat template.

## Executive Summary

**Verdict: CAUTION.** The change is technically feasible and fits the current ordered Whisper → translation → TTS pipeline. It removes the end-user Ollama dependency and gives the application a translation-specific model that officially supports Japanese and Vietnamese, but it does **not** mean “call Hugging Face directly over the network”: the supplied code downloads a 4.08 GB model and executes it locally through Python, PyTorch, and Transformers.

This needs a managed, long-lived Python sidecar and model installer, not a Rust dependency swap. A POC is suitable on the current Apple M5 / 24 GB development machine using PyTorch MPS; a product rollout requires platform-specific packaging, benchmark gates, pinned artifacts, and a licensing decision before distribution.

### Evidence and Scope

- Code-context MCP tools were unavailable; repository findings are based on direct source inspection and therefore have medium confidence where runtime behaviour is not covered by tests.
- The current flow is already bounded and ordered: `local_whisper_ollama.rs` calls `OllamaClient::translate` after Whisper, emits a final transcript snapshot, then queues TTS. That is the intended replacement seam.
- The official model card identifies HY-MT1.5-1.8B as a BF16, 2B-parameter causal model with Japanese and Vietnamese support, a 4.08 GB weight file, and a Transformers 4.56.0 example. It is not deployed by a Hugging Face Inference Provider, so it cannot be substituted by a hosted-provider URL without choosing a separate hosting product.

## Agreements

- Preserve Whisper segmentation, the single ordered translation worker, transcript revisions, bounded queues, cancellation, TTS, and output routing.
- Replace `OllamaClient` with a narrow provider interface; do not teach the audio worker about Python or model files.
- Start one translation sidecar lazily and keep the model resident for the session. Loading the model for every utterance is incompatible with realtime use.
- Download to app-local storage at a pinned Hugging Face commit; verify every expected file before marking the model usable; force offline mode while translating.
- Use the model card’s user-message prompt, not the current Ollama system-prompt shape. The card says the model has no default system prompt and provides the exact non-Chinese translation wording.
- Keep Ollama as a supported fallback until HY-MT quality, latency, installation, and cancellation gates pass on supported hardware.

## Conflicts and Resolution

| Topic | Architect | Security | Performance | UX | Devil's Advocate | Resolution |
| --- | --- | --- | --- | --- | --- | --- |
| “Direct Hugging Face” meaning | A local Python runtime is a new component boundary. | Do not expose a generic local HTTP server. | Loading once is essential. | Users should not need Python, a terminal, or a Hub token. | Keeping Ollama is much simpler. | Treat this as a managed local runtime, not an API replacement; parent-owned stdio IPC is preferred over an unauthenticated port. |
| Model delivery | Dedicated installer/state machine. | Pin commit, allowlist files, hashes, atomic activation. | 4.08 GB download; avoid duplicate HF caches. | Display size, download/retry/repair/offline states. | User-supplied Ollama models avoid this product work. | Download the six runtime files to one app-private model directory; record source revision and SHA-256 in a manifest. |
| Device selection | Make backend selection a runtime capability. | No cloud fallback without explicit consent. | MPS/CUDA can meet latency; CPU may not. | Explain capability rather than exposing `device_map` jargon. | The guide’s `device_map="auto"` is not a cross-platform performance guarantee. | Probe MPS/CUDA/CPU in the sidecar; use explicit per-backend loading and retain a CPU-safe failure state. |
| Generation settings | Keep model-specific prompt/config in the provider. | Bound input/output and redact child stderr. | Greedy decoding may be fast but must be measured. | Translations must stay deterministic and contain no assistant framing. | Current TranslateGemma path is already deployable. | Benchmark greedy (`do_sample=False`) against the card’s recommended sampling values before selecting a product default. |
| Distribution license | Ship notices with the model. | Territory and use restrictions are release blockers. | No material disagreement. | Avoid presenting a model as available where it cannot legally be used. | A globally distributed desktop app may make geo enforcement impractical. | Do not bundle/enable HY-MT in EU, UK, or South Korea without legal approval; make model acquisition conditional on accepted terms and maintain the Ollama alternative. |

## Risk Summary

| Risk | Severity | Persona | Concrete mitigation |
| --- | --- | --- | --- |
| The Tencent license excludes EU, UK, and South Korea, plus imposes distribution/notice obligations. | Critical for a worldwide release; High otherwise | Security | Legal review; release allowlist/geofencing decision; ship required license and notice; no automatic model download in excluded territories. |
| Direct Transformers requires a Python/PyTorch runtime that the Rust/Tauri app does not currently ship. | High | Architect | Add a dedicated managed sidecar with an explicit request/response protocol and lifecycle manager patterned after `vieneu.rs`. |
| The 4.08 GB BF16 weights plus PyTorch runtime, KV cache, Whisper, and TTS can exceed lower-end devices or cause paging. | High | Performance | Capability check, RAM/disk preflight, lazy loading, max input/output limits, one concurrent request, benchmark hardware tiers, and a retained Ollama/System fallback. |
| `device_map="auto"` requires Accelerate and may offload to CPU/disk; it is not the device policy to ship blindly. | High | Performance | Install/pin `accelerate`; select `mps`, CUDA, or CPU explicitly after a self-test; report actual device and first-token/total latency. |
| Partially downloaded or mutable Hub artifacts could leave the application unable to translate or running unreviewed weights. | High | Security | Pin immutable commit, use `snapshot_download(revision=...)` with a file allowlist, verify size/SHA-256, install atomically, and set offline environment variables during inference. |
| Model generation can contain explanations, wrong language, or stale output when cancelled. | Medium | UX | Use the official one-user-message template, slice prompt tokens before decode, validate non-empty target output, preserve generation/cancellation checks, and add JA→VI golden tests. |
| Python-side crashes/long generations can cause the local queue to fill. | Medium | Performance/UX | Per-request deadline, child health check, bounded restart, cancellation command, backpressure/error state, and transcript-first behaviour already used by the worker. |
| The sample uses BF16 and `device_map="auto"`; the installed macOS Python is 3.9 and has no `torch`, `transformers`, `accelerate`, `huggingface_hub`, or `safetensors`. | Medium | Architect | Do not depend on system Python; bundle a pinned per-platform Python environment or sidecar executable and validate it in CI. |

## Persona Details

### Architect

- The existing seam is `OllamaClient::translate` in `src-tauri/src/ai/local_whisper_ollama.rs`; replacing it preserves transcription, transcript reconciliation, TTS, and session lifecycle.
- `src-tauri/src/local_translation.rs` currently persists an Ollama URL/model and validates `POST /api/chat`. Introduce `LocalTranslationEngine` (`ollama` | `hy_mt`) and engine-specific config/test results instead of overloading the Ollama URL fields.
- A Python sidecar is the viable first implementation because the official artifact is a Transformers `HunYuanDenseV1ForCausalLM` model. No Rust-native inference runtime for that architecture is present in this repository.
- Reuse the existing managed-sidecar patterns in `src-tauri/src/vieneu.rs`: backend ownership, startup timeout, child health, stdout protocol, parent-pipe lifetime, and explicit error states. Prefer newline-delimited JSON over stdin/stdout: `{id, sourceLanguage, targetLanguage, text, maxNewTokens}` → `{id, text, latencyMs, device}`.

### Security

- The translation text is meeting content. The normal inference path must set `HF_HUB_OFFLINE=1`, `TRANSFORMERS_OFFLINE=1`, and use only the verified local directory; it must never silently send transcript text to Hugging Face or any fallback service.
- Keep the sidecar private: inherited stdin/stdout avoids a listening port and token-management surface. If HTTP is chosen for reuse, bind `127.0.0.1:0`, authenticate every request, and pass the token only through an inherited pipe.
- Pin a source revision, not `main`; the Hub download APIs support `revision`, `local_dir`, and download filtering. Store a product-owned manifest of exact filenames, sizes, and SHA-256 values.
- The official Tencent license is a gating risk, not boilerplate: it excludes EU/UK/South Korea; distribution requires license/notice conditions and high-MAU use needs a separate license. Get legal confirmation before enabling an installer download.

### Performance

- The current developer machine (Apple M5, 24 GB unified memory) is a reasonable POC target. PyTorch documents MPS as the macOS GPU backend; the sidecar should choose it only after `torch.backends.mps.is_available()` succeeds.
- The official model card’s primary BF16 weight file is 4.08 GB. Effective resident memory will be larger after Python/PyTorch allocations and generation cache. The exact ceiling and tokens/second are unproven and must be measured with Whisper/TTS running concurrently.
- `device_map="auto"` invokes Accelerate’s big-model placement. It can spill layers to CPU/disk, trading memory for lower speed, so it is unsuitable as the sole desktop policy. For MPS use a whole-model MPS path where supported; for CUDA use BF16 where capability permits; allow CPU only as a clearly labelled non-realtime fallback.
- Keep `max_new_tokens` near the current 256-token limit initially, serialize inference, and do not enlarge the utterance queue. Add telemetry for model-load time, first-token time, generation time, queue drops, memory errors, and selected device.

### UX

- Replace “Ollama server URL” and “installed Ollama model” with “Translation engine”, “Install HY-MT model”, device capability, model size/free-space check, download progress, and an explicit **Offline ready** test. Preserve the existing engine as a fallback choice.
- First use must show that the one-time model download is about 4.08 GB, requires a network connection only for installation/update, and that meeting content remains local after setup.
- Realtime expectation needs a hardware qualifier. When the self-test misses a latency threshold, show “text translation may fall behind” before the session begins rather than failing silently mid-call.
- Provide plain recovery actions: Retry download, Repair model, Switch to Ollama, and Copy diagnostics. Do not expose Python stack traces, a device-map setting, or raw cache paths in the primary UI.

### Devil's Advocate

- Core assumption challenged: removing Ollama makes local translation simpler. It only removes an external application; it moves model download, Python/PyTorch packaging, GPU compatibility, and model lifecycle into Baka Trans.
- Simpler alternatives: retain Ollama and use an HY-MT-compatible/quantized model only if an official, supported Ollama distribution becomes available; or add the managed Transformers engine as opt-in while keeping Ollama default.
- Worst case: an unsupported device loads or pages a multi-gigabyte model, translation falls behind, the sidecar crashes, and a globally shipped binary violates territorial license conditions. Bounded queues protect session stability, but not user trust; capability checks and release gating are required.

## Recommended Technical Direction

1. **Build a macOS POC sidecar first.** Pin Python, `torch`, `transformers==4.56.0`, `accelerate`, `huggingface_hub`, and `safetensors`; load `tencent/HY-MT1.5-1.8B` once; probe MPS; send one fixed JA→VI prompt through stdin/stdout. This directly validates the supplied guide on the M5/24 GB development machine.
2. **Do not copy the guide unchanged into production.** Its `device_map="auto"` depends on Accelerate and hides placement; use a backend policy and report the chosen backend. Its greedy generation is acceptable for a POC but differs from the official card’s recommended sampling values, so choose defaults only after quality testing.
3. **Implement app-owned model installation before replacing the provider.** Pin the immutable model revision and expected artifact digests, use resume-capable download into staging, verify, atomically activate, then run inference fully offline.
4. **Add engine abstraction and retain Ollama.** The Rust worker should call one translation contract, allowing safe fallback and A/B evaluation. Do not delete `OllamaClient` until HY-MT passes functional and performance gates.
5. **Define measurable release gates.** Test JA→VI terminology/format fidelity, cancellation, sidecar restart, corrupted/interrupted download, no-network inference, lower-memory failure, MPS/CUDA/CPU matrix, and sustained realtime sessions with Whisper plus TTS.
6. **Resolve licensing before public distribution.** If the product is global, this is a release-blocking decision; if it is limited to permitted territory, package required license/notice text and enforce the agreed distribution policy.

## Implementation Shape (after the POC passes)

```text
PCM -> Whisper (existing Rust) -> TranslationEngine trait
                                  |-> OllamaClient (existing fallback)
                                  `-> HyMtManager (Rust lifecycle)
                                         -> managed Python/torch sidecar
                                         -> local verified HY-MT files
                                      -> translated text
                         -> existing transcript + TTS + selected playback
```

## Next Steps

Proceed under **CAUTION** by planning the macOS POC and legal/distribution decision first. Escalate to **STOP** for any worldwide rollout that cannot exclude or license EU, UK, and South Korea, or if the POC cannot sustain the existing bounded pipeline without queue loss on a documented minimum hardware tier.

## References

- Repository evidence: `src-tauri/src/local_translation.rs`, `src-tauri/src/ai/local_whisper_ollama.rs`, `src-tauri/src/session.rs`, `src-tauri/src/vieneu.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, and `sidecars/vieneu-tts/`.
- [Official HY-MT1.5-1.8B model card](https://huggingface.co/tencent/HY-MT1.5-1.8B) — supported languages, prompt, Transformers version, weights, model options, and local-inference positioning.
- [Official Tencent HY Community License](https://huggingface.co/tencent/HY-MT1.5-1.8B/raw/main/License.txt) — territorial and distribution conditions.
- [Hugging Face model loading](https://huggingface.co/docs/transformers/models) and [Big Model Inference](https://huggingface.co/docs/transformers/main/big_models) — `device_map`, offloading, and dtype behaviour.
- [Hugging Face Hub download guide](https://huggingface.co/docs/huggingface_hub/guides/download) — revision pinning, local directory, and filtered downloads.
- [PyTorch MPS backend](https://docs.pytorch.org/docs/stable/notes/mps.html) — runtime capability probe and macOS GPU execution.
