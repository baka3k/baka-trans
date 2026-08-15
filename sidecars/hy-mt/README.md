# HY-MT M5 proof of concept

This isolated project executes the Phase 09 gate for
`tencent/HY-MT1.5-1.8B`. It never changes the production Ollama engine and it
does not use `/usr/bin/python3`.

## Quick start

```sh
cd sidecars/hy-mt
uv sync --all-groups --frozen
uv run --frozen python -m pytest
uv run --frozen python -m hy_mt_poc probe
```

Download the immutable candidate into the POC-owned ignored directory:

```sh
uv run --frozen python -m hy_mt_poc download \
  --model-dir .artifacts/model \
  --manifest .artifacts/model-manifest.json
```

Run one translation only from local files, with MPS required and silent MPS
fallback disabled:

```sh
uv run --frozen python -m hy_mt_poc --deny-network translate \
  --model-dir .artifacts/model \
  --device mps \
  --mode greedy \
  'おはようございます。'
```

Run the fixed corpus and the 30-minute, five-second-rate sustained loop:

```sh
uv run --frozen python -m hy_mt_poc --deny-network benchmark \
  --model-dir .artifacts/model \
  --corpus corpus/ja-vi.jsonl \
  --output-dir .artifacts/benchmark

uv run --frozen python -m hy_mt_poc --deny-network soak \
  --model-dir .artifacts/model \
  --corpus corpus/ja-vi.jsonl \
  --duration-seconds 1800 \
  --interval-seconds 5 \
  --output-dir .artifacts/soak
```

## Packaging spike

PyInstaller must run on the target platform. The model remains external to the
runtime folder.

```sh
uv run --frozen pyinstaller --noconfirm --clean hy-mt-poc.spec
dist/hy-mt-poc/hy-mt-poc probe
```

The final evidence records the nested native libraries and `codesign` result.
Production signing/notarization remains Phase 14 work.

## Cleanup

Only the POC-owned paths may be removed. From this directory, delete
`.artifacts/`, `build/`, and `dist/` when the evidence has been copied to the
Phase 09 report directory. Do not remove global Hugging Face or Python caches.
