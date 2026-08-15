---
type: research
date: 2026-08-15
---

# Research: managed HY-MT translation runtime

## Summary

Direct local inference with `tencent/HY-MT1.5-1.8B` is feasible without exposing Python installation to internal users. The correct integration is a bundled, long-lived Python/PyTorch/Transformers sidecar managed by Rust; the existing Whisper segmentation, ordered worker, transcript snapshots, TTS queue, and playback should remain unchanged.

The first gate belongs on the current Apple M5/24 GB development machine. Product integration should not begin until actual quality, latency, memory, cancellation, offline, and one-folder packaging evidence is recorded.

## Scope Challenge

1. **Runtime boundary:** use a private managed sidecar, not hosted Hugging Face inference and not embedded Python inside the Tauri process.
2. **Migration:** keep Ollama explicitly selectable and migrate existing configs to it; do not silently fallback per utterance.
3. **Rollout:** POC and macOS arm64 first; Windows same-platform packaging and CPU/CUDA capability validation after the shared runtime is stable.

## Repository Evidence

- `src-tauri/src/ai/local_whisper_ollama.rs` already provides the narrow replacement seam at `ollama.translate(&source_text)`. Work before and after that call is provider-independent.
- `src-tauri/src/local_translation.rs` owns Ollama configuration, URL normalization, prompt/payload, response parsing, and readiness probe. It should retain the Ollama adapter and add engine dispatch rather than absorb process management.
- `src-tauri/src/session.rs` loads Whisper before capture and can also require HY-MT installation/prewarm before creating the local session.
- `src-tauri/src/vieneu.rs` proves the repository already supports an app-managed Python sidecar, staged model installation, process ownership, progress events, offline inference, and one-folder resources. HY-MT can reuse patterns while remaining independently deployable.
- `src/components/settings/LocalLlmSettings.tsx` currently validates Ollama fields unconditionally; engine selection must make its validation and readiness cards conditional.
- `scripts/release-macos.sh` does not build any Python sidecar today. `scripts/release-windows.ps1` builds VieNeu only for installer builds. HY-MT needs target-specific build and artifact gates on both paths.
- `docs/development-rules.md` does not exist in this checkout, so no additional rules could be applied from that expected path.

## Model and Runtime Evidence

- Model ID: `tencent/HY-MT1.5-1.8B`.
- Candidate pinned revision for the POC: `172d98efc7f534e05c86d3d49ed9d12d9c2a733b`; implementation must retain the exact revision that passes the gate.
- Seven required inference files: `chat_template.jinja`, `config.json`, `generation_config.json`, `model.safetensors`, `special_tokens_map.json`, `tokenizer.json`, and `tokenizer_config.json`.
- Published total for those files: 4,086,768,591 bytes. Published `model.safetensors` SHA-256: `07736f560253d8c991616060fb2d855420957c268fa7d32fa8593df2f83b21ab`. Pin hashes for every smaller file during implementation.
- Architecture: `HunYuanDenseV1ForCausalLM` / `hunyuan_v1_dense`, BF16, 32 layers. The model config declares Transformers 4.56.1; pin the final PyTorch and packaging versions only after the M5 POC.
- Prompt: one user message, `Translate the following segment into {target language name}, without additional explanation.\n\n{source text}`; no system prompt; `add_generation_prompt=False`; decode only tokens generated after the input length.
- Generation decision: compare deterministic greedy decoding with the model generation defaults; lock the product setting from quality/latency evidence rather than copying either sample blindly.

## Proposed Protocol

Use strict NDJSON over inherited stdin/stdout. Stdout contains protocol only; sanitized diagnostics go to stderr.

```text
ready     {protocolVersion, modelId, revision, device, dtype, pid, loadMs}
translate {type, id, sourceLanguage, targetLanguage, text, maxNewTokens}
result    {id, text, inputTokens, outputTokens, latencyMs}
cancel    {type, id}
error     {id, code, message, retryable}
```

The sidecar uses an input-reader thread and generation worker so cancellation can set a Transformers stopping criterion. Rust enforces one in-flight request, request IDs, a deadline, bounded restart, unload, and parent-owned shutdown. If cooperative cancellation misses its grace period, Rust terminates the child and starts a clean process for the next request.

## Model Lifecycle

1. Preflight free disk based on measured runtime plus staging/current model needs.
2. Download only the pinned allowlist into app-private staging using a fixed revision and no global cache.
3. Resume interrupted files, then verify exact sizes and SHA-256 values.
4. Write an install manifest and atomically activate on the same filesystem.
5. Retain `License.txt`, required notice, model ID, revision, runtime versions, and verification time.
6. In serve mode force offline environment variables, disable telemetry, load the verified local directory with `local_files_only=True`, and expose no listening port.

## Device and Packaging Policy

- macOS: require `torch.backends.mps.is_available()` for the realtime POC path, attempt the POC-approved BF16/dtype strategy, record fallbacks, build a same-platform one-folder bundle, and sign/notarize nested native libraries.
- Windows: build with PyInstaller on Windows, establish CPU as the compatibility baseline, probe CUDA explicitly, measure bundle impact, smoke the bundled executable offline, and record Defender/installer behavior.
- Do not depend on `/usr/bin/python3`, user virtual environments, or `device_map="auto"` in the shipped application.

## Recommended Phase Boundary

- 09: M5 feasibility and decision gate.
- 10: sidecar protocol and verified model lifecycle.
- 11: Rust engine/config/manager.
- 12: frontend readiness and language constraints.
- 13: pipeline/cancellation/regression integration.
- 14: macOS packaging and release evidence.
- 15: Windows packaging and release evidence.

## References

- [Prediction report](../../reports/prediction_report_20260815_1230.md)
- [Official model card](https://huggingface.co/tencent/HY-MT1.5-1.8B)
- [Tencent HY Community License](https://huggingface.co/tencent/HY-MT1.5-1.8B/raw/main/License.txt)
- [Hugging Face Hub download guide](https://huggingface.co/docs/huggingface_hub/guides/download)
- [PyTorch MPS backend](https://docs.pytorch.org/docs/stable/notes/mps.html)
