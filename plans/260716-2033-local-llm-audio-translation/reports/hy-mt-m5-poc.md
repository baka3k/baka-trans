---
type: feasibility-gate
date: 2026-08-15
verdict: caution
---

# HY-MT M5 POC and Decision Gate

## Summary

**CAUTION (owner-approved limited continuation). Phase 10 may proceed to build
the isolated sidecar/model lifecycle only. Keep the production Ollama path and
default engine unchanged; do not route live sessions to HY-MT or begin Phases
11-15 without a fresh quality gate.**

The exact `tencent/HY-MT1.5-1.8B` candidate runs locally on Apple M5 MPS with
BF16, packages into a same-platform PyInstaller one-folder runtime, works with
all network operations denied, and sustained 360 translations over 30 minutes
without a crash. It does not pass the translation-quality gate: an accepted
Japanese input returned English in both generation modes, and several other
cases changed critical technical terms, names, or instructions. The required
current TranslateGemma/Ollama comparison, bilingual human sign-off, and combined
Whisper/HY-MT/TTS memory run were also unavailable on this machine.

The original hard gate result was STOP. The product owner explicitly accepted a
limited-risk continuation on 2026-08-15: Phase 10 may make the candidate
installable and process-isolated for further evaluation. This is not quality
approval or permission to replace, route around, or remove Ollama. No
production Rust, Tauri, settings, engine-selection, or Ollama-default code has
changed as part of Phase 09.

## Decision Matrix

| Gate | Result | Evidence |
| --- | --- | --- |
| Isolated exact dependency lock | PASS | Python 3.12.13 under `uv`; exact runtime/build pins in `sidecars/hy-mt/uv.lock` |
| Immutable model and hashes | PASS | Revision `172d98efc7f534e05c86d3d49ed9d12d9c2a733b`; all 10 downloaded files hashed |
| Explicit MPS/device/dtype | PASS | Actual parameter device `mps:0`, actual dtype `bfloat16`, no fallback |
| Official prompt and suffix decode | PASS | One user message, no system message, `add_generation_prompt=False`, suffix-only decode tests |
| Accepted corpus produces only valid Vietnamese | **FAIL** | `punctuation-03` produced English in greedy and recommended modes |
| Critical semantic fidelity | **FAIL** | Regression test, availability, feature flag, transcription, production data, and proper-name errors |
| Current Ollama baseline and bilingual human review | **NOT RUN** | `ollama` executable/model absent; no named bilingual human reviewer was supplied |
| Offline inference | PASS | Successful load and translation under `sandbox-exec` with `deny network*`, local-only loading, and socket denial |
| 30-minute sustained HY-MT loop | PASS with rate note | 360/360 complete, zero crashes; 3 isolated requests exceeded the five-second fixture interval |
| Combined Whisper + HY-MT + TTS memory | **NOT RUN** | No Ollama, Whisper, or VieNeu process/runtime was resident or available |
| One-folder package | PASS for spike | arm64 runtime starts, loads, and translates offline; size/startup need mitigation |
| Internal Vietnam license assumption | CAUTION | Vietnam is in the defined Territory, but legal acceptance and operational territory controls remain required |

The quality failure violates the original Phase 09 success criteria. The
missing comparison, human review, and combined-process evidence independently
prevent GO. The owner-approved CAUTION is intentionally narrower than GO:
Phase 10 creates no user-selectable translation route and must retain the
evidence needed for a fresh quality gate.

## Tested Pins

These pins describe the failed POC candidate and are retained for reproduction
and the owner-approved, isolated Phase 10 sidecar. They are **not approved for
a user-selectable engine, live routing, or a default change**.

| Component | Exact value |
| --- | --- |
| Python | 3.12.13, uv-managed arm64 CPython |
| uv | 0.12.3 |
| Transformers | 4.56.1 |
| PyTorch | 2.12.0 |
| safetensors | 0.6.2 |
| Hugging Face Hub | 0.34.4 |
| psutil | 7.0.0 |
| PyInstaller | 6.18.0 |
| Model | `tencent/HY-MT1.5-1.8B` |
| Revision | `172d98efc7f534e05c86d3d49ed9d12d9c2a733b` |

Accelerate was intentionally omitted: the runner loads to one explicit MPS
device and does not use `device_map` or an Accelerate loading API.

## Hardware and Runtime

| Item | Measured value |
| --- | --- |
| Host | Apple M5, arm64 |
| Memory | 25,769,803,776 bytes (24 GiB) |
| macOS | 26.4, build 25E246 |
| Kernel | Darwin 25.4.0 |
| Selected/actual device | `mps` / `mps:0` |
| Selected/actual dtype | BF16 / BF16 |
| MPS built/available | true / true |
| Silent MPS fallback | disabled; none observed |
| Model load | 1.338-1.956 s across recorded local and bundled runs |
| MPS allocation after load | 3,582,161,152 bytes current; 4,337,369,088 bytes driver |
| Peak soak MPS driver allocation | 4,356,980,736 bytes |
| Peak soak process RSS | 636,731,392 bytes |

The host-wide `psutil` used-memory figure peaked at 88.6%, above the requested
80% threshold, but it was already above 80% around model load and includes
unrelated host memory/cache. `memory_pressure -Q` reported 54% free before and
69% free after the soak, so no pressure escalation was observed. This cannot be
substituted for the required combined Whisper/HY-MT/TTS measurement; that gate
remains not run.

## Model Artifacts

The POC downloaded 4,086,794,769 bytes into its ignored, dedicated
`sidecars/hy-mt/.artifacts/model` directory. The production model store and
global Hugging Face cache were not used as product state.

| Artifact | Bytes | SHA-256 |
| --- | ---: | --- |
| `.gitattributes` | 1,519 | `11ad7efa24975ee4b0c3c3a38ed18737f0658a5f75a0a96787b576a78a023361` |
| `License.txt` | 16,270 | `d7d9db858500ac9073f4b5decef8e208454357226f535f65079ce4376047569f` |
| `README.md` | 8,389 | `302601d23ad541ef69827167e60d25c2c04265eff54fee690cb7c3c5638e7fcc` |
| `chat_template.jinja` | 654 | `b7491ec0e9c869dfce20f2176758099bf248d979dd05530ede99deb21698acee` |
| `config.json` | 1,342 | `a1788df3224420f43ed1a424ad58bfacc34f689b0e477ce69d1298fa6d26292b` |
| `generation_config.json` | 221 | `3586ba4829d9769b89523523cb562f2e894c519274f8a0e9b970287a0b1388a9` |
| `model.safetensors` | 4,077,072,784 | `07736f560253d8c991616060fb2d855420957c268fa7d32fa8593df2f83b21ab` |
| `special_tokens_map.json` | 488 | `bb9f59990034dae326581b9c62471523975417869f78a244b7ae2ce8cbb085eb` |
| `tokenizer.json` | 9,527,287 | `b475bbef1b0b2fd57dcb865332b546475bd1ede2deb3bb91bafd0c047a8a530a` |
| `tokenizer_config.json` | 165,815 | `53bd8581b601a8ee9caefeb988207de50b3fc0b733295bdf5ad68dec4cc0b07c` |

## Prompt and Decode Verification

The tested prompt was exactly:

```text
Translate the following segment into Vietnamese, without additional explanation.

{source text}
```

The tokenizer received exactly one `user` message. The recorded rendered
template begins with the model BOS/user tokens and ends with the template's
placeholder token; no system prompt or generation prompt was added. Generated
IDs are sliced at `input_ids.shape[1]` before `batch_decode`, so prompt tokens
cannot appear in the returned translation. Pure tests cover the exact prompt,
chat-template arguments, suffix slicing, boundary rejection, and explicit
device policy.

## Corpus and Quality

The fixed corpus contains 36 accepted Japanese utterances and four rejected
boundary cases. It covers short/long speech, names, numbers, technical meetings,
punctuation, mixed English, code identifiers, politeness, negation, ambiguity,
audio terminology, fragments, and units.

| Metric | Greedy | Model-card recommended |
| --- | ---: | ---: |
| Completed / attempted | 36 / 36 | 36 / 36 |
| Empty outputs | 0 | 0 |
| Output tokens | 1,033 | 1,045 |
| Warm p50 | 1,331.200 ms | 1,416.141 ms |
| Warm p95 | 3,567.634 ms | 3,711.432 ms |
| Mean | 1,539.048 ms | 1,590.255 ms |

Greedy used `do_sample=False`; sampling-only values were cleared to avoid
silently inheriting the model's defaults. Recommended used `do_sample=True`,
`temperature=0.7`, `top_k=20`, `top_p=0.6`, and
`repetition_penalty=1.05`, with seed 0 for reproducibility. The pinned README's
recommendation (`top_p=0.6`) intentionally takes precedence over the separate
repository `generation_config.json` value (`top_p=0.8`) for this named
model-card comparison. Only 19 of 36 cases were byte-identical between modes;
neither mode consistently fixed the defects.

### Fixed Review Dimensions

Every accepted output was inspected provisionally for target language,
semantic fidelity, names/numbers/technical terms, negation and instruction
polarity, formatting/no explanation, and Vietnamese fluency. This inspection is
not represented as the required bilingual human sign-off.

| Case | Dimension | Greedy / recommended issue |
| --- | --- | --- |
| `punctuation-03` | Target language | Both returned English, not Vietnamese |
| `meeting-05` | Technical term | Both rendered regression testing as regression-analysis testing |
| `meeting-06` | Semantic/technical | Incident/root cause became defect or defect/root cause |
| `names-01` | Proper name | Nguyễn became “Gueyn” in both modes |
| `numbers-03` | Technical metric | Availability became reliability in both modes |
| `mixed-03` | Technical term | `feature flag` was reduced to generic options |
| `politeness-01` | Speaker/addressee | Both modes changed “could you speak slower” to “could we speak slower” |
| `negation-02` | Technical term | Production data became original data; approval semantics changed |
| `code-01` | Instruction fidelity | “Pin revision” became “keep revision unchanged” |
| `audio-01` | Technical term | Transcription became text translation |
| `audio-02` | Semantic fidelity | “Read only the final confirmed sentence” lost confirmation/finality meaning |
| `mixed-02` | Sampling regression | Recommended translated Pull Request/CI into unrelated generic concepts |

The complete Japanese source, Vietnamese reference, both candidates, token
counts, timings, and memory samples are preserved in `benchmark.csv` and
`benchmark.json`. No byte-equality requirement was used for judging quality.

### Missing Baseline and Human Sign-off

`ollama` was not installed and no Ollama process/model was resident, so the
current TranslateGemma/Ollama result could not be generated on the same corpus.
The `referenceVi` field is a fixed review aid, not a fabricated Ollama result.
No named bilingual human reviewer was available, so human acceptance remains
explicitly incomplete.

## Offline, Cancellation, and Sustained Results

Offline inference ran after caching under both application-level denial and a
macOS sandbox profile containing `(deny network*)`. Loading used an absolute
local model directory, `local_files_only=True`, `trust_remote_code=False`, and
safetensors. The offline candidate returned Vietnamese successfully; no network
fallback was possible.

The cancellation experiment set a 10 ms deadline on a long request. The model
stopping criterion was observed at the first token boundary after 1,201.362 ms,
returning one partial token and `cancelled=true`. This proves cooperative stop
observation, not the later Phase 10 parent kill/restart protocol.

The sustained run used greedy decoding, six rotating fixtures, and one request
every five seconds:

| Metric | Result |
| --- | ---: |
| Target / actual duration | 1,800.0 / 1,800.925 s |
| Completed / attempted | 360 / 360 |
| Failures / crashes | 0 / 0 |
| p50 / p95 | 1,065.683 / 2,497.685 ms |
| Requests over five seconds | 3 |
| Peak process RSS | 636,731,392 bytes |
| Peak MPS driver allocation | 4,356,980,736 bytes |

Three isolated requests exceeded five seconds, including a 5.940 s late-run
sample; the p95 remained below the fixture interval, so there was no sustained
backlog. The result is HY-MT-only because combined runtime prerequisites were
absent.

## One-folder Packaging Spike

| Item | Result |
| --- | --- |
| Platform/architecture | macOS 26.4 / arm64 |
| PyInstaller mode | one-folder |
| Apparent file bytes | 809,037,904 |
| Allocated folder size | 553,428 KiB (about 540 MiB) |
| Files | 14,731 |
| Native candidates checked | 39 |
| Native signature failures | 0 |
| Signature type | PyInstaller ad-hoc; no Team ID |
| Fresh rebuilt-process MPS probe | 8.139 s wall, exit 0 |
| Fresh rebuilt-process cached offline translation | 11.989 s wall, exit 0 |
| Bundled model load / generation | 1,394.087 / 1,535.817 ms |

The model is external to the runtime folder. The recorded startup commands used
fresh processes after the bundle and model had already been read on this host;
filesystem caches were not purged, and that state is explicit in the execution
evidence. The bundle duplicates large torch library paths in apparent bytes and
has high file-count startup risk. A production build would need hook pruning and
size/startup optimization plus cold-host measurements.

PyInstaller's ad-hoc signatures make the spike runnable on Apple Silicon, but
they are not distribution signing. Phase 14 would need to replace nested
signatures with the application identity, apply the hardened runtime and any
required entitlements consistently, verify every nested Mach-O file, sign the
containing app in inside-out order, and notarize/staple the final artifact.

## License and Territory Assumption

This is an engineering record, not legal advice. The pinned Tencent HY Community
License defines its Territory as worldwide excluding the EU, UK, and South
Korea. Internal use physically and operationally limited to Vietnam is inside
that stated Territory. “Internal” does not override the territorial restriction:
the model and its outputs must not be used outside the Territory.

If the model is ever distributed to third parties, the agreement and required
Notice must accompany it, modified files need notices, downstream use
restrictions must be passed through, and provider/non-affiliation disclosure is
required. The agreement also has an over-100-million-MAU licensing threshold and
restricts use of outputs to improve other AI models. Legal acceptance, employee
travel/remote-access controls, model acquisition controls, required notices, and
dependency license inventory would remain release blockers even if quality were
fixed.

## Raw Commands

Commands were run from `sidecars/hy-mt` unless noted:

```sh
uv lock
uv sync --all-groups --frozen
uv run --frozen python -m pytest
uv run --frozen python -m hy_mt_poc probe --output .artifacts/probe.json
uv run --frozen python -m hy_mt_poc download --model-dir .artifacts/model --manifest .artifacts/model-manifest.json
uv run --frozen python -m hy_mt_poc --deny-network prompt --model-dir .artifacts/model --device mps --output .artifacts/prompt.json '田中さん、会議は午前9時30分からです。'
uv run --frozen python -m hy_mt_poc --deny-network translate --model-dir .artifacts/model --device mps --mode greedy --timeout-seconds 0.01 --output .artifacts/cancellation.json 'ネットワークを完全に無効にした状態でも、必要なファイルがローカルに揃っていれば、会議の内容を外部へ送信せずに翻訳を続けられることを確認します。'
uv run --frozen python -m hy_mt_poc --deny-network benchmark --model-dir .artifacts/model --device mps --corpus corpus/ja-vi.jsonl --output-dir .artifacts/benchmark
uv run --frozen python -m hy_mt_poc.experiment --label offline-python --output .artifacts/offline-execution.json --sandbox-profile '(version 1)(allow default)(deny network*)' --state-note 'fresh Python process; model cached locally; filesystem cache not purged' -- sandbox-exec -p '(version 1)(allow default)(deny network*)' .venv/bin/python -m hy_mt_poc --deny-network translate --model-dir .artifacts/model --device mps --mode greedy --output .artifacts/offline.json 'オフラインでも翻訳を続けます。'
uv run --frozen python -m hy_mt_poc --deny-network soak --model-dir .artifacts/model --device mps --corpus corpus/ja-vi.jsonl --duration-seconds 1800 --interval-seconds 5 --output-dir .artifacts/soak
uv run --frozen pyinstaller --noconfirm --clean hy-mt-poc.spec
uv run --frozen python -m hy_mt_poc.experiment --label bundle-probe --output .artifacts/bundle-probe-execution.json --state-note 'fresh bundled process; filesystem cache not purged' -- dist/hy-mt-poc/hy-mt-poc probe --output .artifacts/bundle-probe.json
uv run --frozen python -m hy_mt_poc.experiment --label bundle-offline-translation --output .artifacts/bundle-translation-execution.json --sandbox-profile '(version 1)(allow default)(deny network*)' --state-note 'fresh bundled process; model cached locally; filesystem cache not purged' -- sandbox-exec -p '(version 1)(allow default)(deny network*)' dist/hy-mt-poc/hy-mt-poc --deny-network translate --model-dir .artifacts/model --device mps --mode greedy --output .artifacts/bundle-translation.json 'パッケージ版でも翻訳できます。'
uv run --frozen python -m hy_mt_poc.package_evidence --bundle-dir dist/hy-mt-poc --output .artifacts/package.json
```

## Evidence

Raw evidence is in [hy-mt-m5-poc-evidence](hy-mt-m5-poc-evidence/):

- `model-manifest.json`: exact model files, sizes, and SHA-256 values.
- `probe.json`, `environment-before.json`, `environment-after.json`: runtime,
  MPS, host, resident-process, and memory-pressure evidence.
- `prompt.json`, `cancellation.json`, `offline.json`,
  `offline-execution.json`: template, stop, network-denied inference, exact
  sandbox command/profile, exit status, wall time, and cache-state evidence.
- `benchmark.json`, `benchmark.csv`, `boundaries.csv`: complete quality and
  latency corpus outputs.
- `soak.json`, `soak.csv`: all 360 sustained-loop iterations.
- `package.json`, `bundle-probe.json`, `bundle-probe-execution.json`,
  `bundle-translation.json`, `bundle-translation-execution.json`: bundle size,
  native signatures, exact commands/cache state, startup wall time, load, and
  offline bundled translation.

## Required Follow-up

The owner-approved continuation opens **Phase 10 only**. Phases 11-15 remain
closed. Before any engine selection, Tauri wiring, live-session routing, or
default change, a new gate must evaluate a reviewed model/revision or prompting
approach against the same corpus with bilingual human acceptance, an installed
Ollama baseline, and a combined Whisper/HY-MT/TTS memory run. This CAUTION must
not be interpreted as permission to wire the failed candidate into Tauri.
