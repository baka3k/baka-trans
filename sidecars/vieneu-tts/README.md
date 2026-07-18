# VieNeu-TTS managed bridge

This private loopback bridge is bundled with Baka Trans. The desktop app owns its
installation, authenticated process, model download, health checks, and shutdown.
End users do not need Python, `uv`, a terminal, or a bridge URL.

The pinned ONNX/int8 model is downloaded only after the user chooses **Install
VieNeu-TTS**. It is verified by exact file size and SHA-256, then stored under the
application's local data directory. Interrupted downloads remain resumable.

## Developer checks

```powershell
cd sidecars/vieneu-tts
uv sync
uv run python -m py_compile server.py
uv run python server.py --help
```

Build the Windows one-folder runtime (PyInstaller must run on the target OS):

```powershell
./scripts/build-vieneu-sidecar.ps1
```

The release script runs this automatically before the Tauri installer build.
The bridge binds to `127.0.0.1` on an ephemeral port, requires a per-process bearer
token, and exits when its inherited parent-lifetime pipe closes.
