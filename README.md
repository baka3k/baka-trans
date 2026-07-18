# Baka Trans

Real-time meeting translation desktop app for macOS and Windows, built with Tauri, React, TypeScript, and Rust.

## Development

Install dependencies:

```bash
npm ci
```

Run the app in development mode:

```bash
npm run tauri -- dev
```

The native Whisper dependency builds whisper.cpp from source. Install CMake and a C/C++ toolchain (Visual Studio Build Tools with Desktop development with C++ on Windows, or Xcode command-line tools on macOS) before the first Rust or Tauri build.

Build only the frontend assets:

```bash
npm run build
```

## Local Japanese-to-Vietnamese Translation

The main window asks you to choose **Cloud API** or **Local Whisper** on every launch. Cloud API opens the existing Google/OpenAI workspace unchanged. Local Whisper runs this spoken pipeline:

```text
PCM16 mono, 16000 Hz -> local Whisper (Japanese) -> Gemma via Ollama /api/chat -> Vietnamese text -> system TTS or VieNeu-TTS -> PCM16 mono, 24000 Hz -> selected audio output
```

It does not require a Google/OpenAI translation key. The translated voice uses the same output-device and all/left/right routing controls as cloud playback. Google and OpenAI modes keep their existing credential and audio behavior.

1. Install and start [Ollama](https://docs.ollama.com/quickstart).
2. Pull Gemma with `ollama pull gemma3:4b`, or configure another installed Gemma variant. Ollama also documents its [model pull API](https://docs.ollama.com/api/pull).
3. Open **Local LLM**, choose a multilingual Whisper model, and select **Download model**. The app stores it in its private data folder and fills the GGML path automatically. You can still use an existing absolute model path. The [whisper.cpp model guide](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) lists the official model files.
4. Choose a speech engine. For System TTS, install a Vietnamese voice in Windows Speech settings or macOS system voice settings. For VieNeu-TTS, start the [local VieNeu bridge](sidecars/vieneu-tts/README.md).
5. Choose **Local Whisper**, then open **Local LLM**. Set the Gemma model, download or select a Whisper model, select the speech engine, refresh and choose a Vietnamese voice, rate, and volume. Save, then select **Test local pipeline**.
6. Under **Audio**, choose the meeting source, translated output, and channel. Use **Test selected voice** to verify the routed output, then start. The language pair is fixed to Japanese (`ja`) -> Vietnamese (`vi`) for this version.

The native client sends non-streaming requests to `POST /api/chat`; it never routes local translation through `/v1/chat/completions`. Default segmentation is 300 ms minimum speech, 700 ms trailing silence, 15 seconds maximum utterance, 250 ms pre-roll, and a 0.015 RMS speech threshold. Raise the threshold in noisy rooms; increase trailing silence if Japanese phrases are split too aggressively.

The distributed baseline is CPU-capable. More Whisper threads can reduce latency at the cost of CPU contention. The GPU toggle is capability-dependent and safely falls back when the build has no supported accelerator backend; GPU-enabled packaging requires compiling `whisper-rs` with the appropriate platform feature.

### Opt-in local smoke test

Default tests do not require Ollama or a Whisper model. For a real end-to-end check, provide a raw little-endian PCM16 mono 16000 Hz Japanese fixture and run:

```powershell
$env:BAKA_TRANS_WHISPER_MODEL='C:\models\ggml-small.bin'
$env:BAKA_TRANS_OLLAMA_MODEL='<model>'
$env:BAKA_TRANS_JAPANESE_PCM='C:\fixtures\japanese-16k-mono.pcm'
cargo test --manifest-path src-tauri/Cargo.toml local_whisper_ollama_end_to_end_smoke_test -- --ignored --nocapture
```

Common local errors:

| Code | Action |
| --- | --- |
| `local_whisper_model_unreadable` | Select an existing, readable GGML model file. |
| `local_whisper_model_load_error` | Verify the file is a compatible, non-empty whisper.cpp model. |
| `local_ollama_request_error` | Start Ollama and verify the configured origin, normally `http://localhost:11434`. |
| `local_ollama_provider_error` | Pull the configured model and retry the Local LLM test. |
| `local_translation_backlog_full` | Use shorter utterances, a smaller/faster model, or more capable hardware. |
| `local_vieneu_unreachable` | Start `sidecars/vieneu-tts/server.py` and verify the loopback bridge URL. |
| `local_vieneu_provider_error` | Check the VieNeu bridge terminal, model download, selected preset voice, and reading style. |
| `local_tts_voice_missing` | Refresh and reselect the configured system or VieNeu voice, then save and retest. |
| `local_tts_backlog_full` | Reduce local-model latency or pause until speech catches up. |
| `local_tts_playback_error` | Reconnect and reselect the translated output device. |

## Build Installer

The current Tauri bundle config is set up for macOS only:

- `.dmg` installer
- `.app` application bundle

Build the macOS installer from this repository root:

```bash
npm ci
npm run tauri -- build
```

Build only the `.dmg` installer:

```bash
npm run tauri -- build --bundles dmg
```

After a successful macOS build, artifacts are written under:

```text
src-tauri/target/release/bundle/dmg/
src-tauri/target/release/bundle/macos/
```

For local testing, open the generated `.dmg` or launch the generated `.app`.
For public distribution, add Apple code signing and notarization before sharing the installer.

For the versioned build, verification, tag, and GitHub Release workflow, see
[the macOS release guide](docs/RELEASE_GUIDE.md).

## Windows Development

The generated macOS `.dmg` and `.app` files do not run on Windows.

Windows uses its built-in WASAPI loopback capture so users can leave Teams on their normal speaker or headset. No virtual audio driver is required for the default workflow. VB-CABLE is only a troubleshooting fallback.

To create a Windows build, use a Windows machine or Windows CI with Node.js, Rust, CMake, and Microsoft Visual Studio Build Tools (including C++) installed. Then build with Windows bundle targets:

```powershell
npm ci
npm run release:windows
```

Expected Windows artifacts:

```text
src-tauri\target\release\bundle\nsis\
```

Run the Windows checks without producing an installer with `npm run release:windows:check`. See [the Windows user guide](docs/WINDOWS_TEAMS_USER_GUIDE.md) and [release guide](docs/WINDOWS_RELEASE_GUIDE.md).
