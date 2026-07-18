# VieNeu-TTS bridge

This loopback-only service keeps VieNeu-TTS v3 Turbo loaded while baka-trans is running.
It exposes health, preset voice discovery, and PCM16 WAV synthesis to the Rust desktop backend.

## Run

```powershell
cd sidecars/vieneu-tts
uv sync
uv run python server.py
```

The first start downloads the VieNeu model. The default endpoint is
`http://127.0.0.1:23334`, using the ONNX int8 CPU backend. Optional flags:

```powershell
uv run python server.py --precision fp32 --threads 8
```

Keep the terminal running, select **VieNeu-TTS** in Local LLM settings, refresh the
voice list, then save and test the local pipeline.

## API

- `GET /health`
- `GET /voices`
- `POST /synthesize` with `text`, `voice`, `style`, `rate`, and `volume`

The server only accepts loopback bind addresses. Do not expose it through a public proxy.
