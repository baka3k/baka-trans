# HY-MT sidecar

This project contains the isolated Phase 09 POC and the Phase 10 managed
sidecar for `tencent/Hy-MT2-1.8B`. The desktop application owns the managed
model root, child process, installation, and shutdown. Users need no Python,
terminal, Ollama, or global Hugging Face cache for the HY-MT path.

The sidecar is not a network service. In serve mode it communicates only with
its parent over inherited stdin/stdout using versioned NDJSON, has no listening
port, forces local-only loading, rejects Hub credentials, and reserves stdout
for protocol frames. Ollama remains the product default until the later engine
selection and pipeline phases explicitly opt into HY-MT.

## Quick start

```sh
cd sidecars/hy-mt
uv sync --all-groups --frozen
uv run --frozen python -m pytest
uv run --frozen python -m hy_mt_poc probe
```

## Managed sidecar

The app passes an app-local `--model-root`; never point it at a shared Hugging
Face cache. Installation is revision-pinned, allowlisted, verified by fixed
sizes/SHA-256, staged under that root, and atomically activated only after all
checks pass. A previous verified active model remains available until a new
staging copy activates.

```sh
uv run --frozen python server.py install --model-root /absolute/app-data/hy-mt
uv run --frozen python server.py check --model-root /absolute/app-data/hy-mt
uv run --frozen python server.py serve --model-root /absolute/app-data/hy-mt --device mps
```

`serve` expects one JSON object per input line. It first emits `ready`, accepts
only `translate` (`ja` → `vi`) and `cancel`, processes one translation at a
time, and exits when the parent closes stdin. Primary protocol errors are
stable and do not include text, tokens, tracebacks, or absolute paths.

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
dist/hy-mt-sidecar/hy-mt-sidecar check --model-root /absolute/app-data/hy-mt
```

The final evidence records the nested native libraries and `codesign` result.
Production signing/notarization remains Phase 14 work.

## Cleanup

Only the POC-owned paths may be removed. From this directory, delete
`.artifacts/`, `build/`, and `dist/` when the evidence has been copied to the
Phase 09 report directory. Do not remove global Hugging Face or Python caches.
