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
PCM16 mono, 16000 Hz -> local Whisper (Japanese) -> selected translation engine (Hy-MT2 offline or OpenAI-compatible API) -> Vietnamese text -> system TTS or VieNeu-TTS -> PCM16 mono, 24000 Hz -> selected audio output
```

It does not require a Google/OpenAI translation key. The translated voice uses the same output-device and all/left/right routing controls as cloud playback. Google and OpenAI modes keep their existing credential and audio behavior.

1. Open **Local LLM**, choose a translation engine (offline Hy-MT2 or an OpenAI-compatible API), and select a multilingual Whisper model. Select **Download model** to fetch it; the app stores it in its private data folder and fills the GGML path automatically. The [whisper.cpp model guide](https://github.com/ggml-org/whisper.cpp/blob/master/models/README.md) lists the official model files.
2. Choose a speech engine. For System TTS, install a Vietnamese voice in Windows Speech settings or macOS system voice settings. For VieNeu-TTS, select **Install VieNeu-TTS**. The app downloads and verifies the pinned ONNX/int8 model (about 244 MiB), resumes interrupted downloads, and manages the private runtime automatically.
3. Choose **Local Whisper**, then open **Local LLM**. Configure the translation engine, download or select a Whisper model, select the speech engine, refresh and choose a Vietnamese voice, rate, and volume. Save, then select **Test local pipeline**.
4. Under **Audio**, choose the meeting source, translated output, and channel. Use **Test selected voice** to verify the routed output, then start. The language pair is fixed to Japanese (`ja`) -> Vietnamese (`vi`) for this version.

The OpenAI-compatible engine sends non-streaming requests to `POST {baseUrl}/chat/completions`. Default segmentation is 300 ms minimum speech, 700 ms trailing silence, 15 seconds maximum utterance, 250 ms pre-roll, and a 0.015 RMS speech threshold. Raise the threshold in noisy rooms; increase trailing silence if Japanese phrases are split too aggressively.

The distributed baseline is CPU-capable. More Whisper threads can reduce latency at the cost of CPU contention. The GPU toggle is capability-dependent and safely falls back when the build has no supported accelerator backend; GPU-enabled packaging requires compiling `whisper-rs` with the appropriate platform feature.

### Opt-in local smoke test

Default tests do not require a translation engine or a Whisper model.

Common local errors:

| Code | Action |
| --- | --- |
| `local_whisper_model_unreadable` | Select an existing, readable GGML model file. |
| `local_whisper_model_load_error` | Verify the file is a compatible, non-empty whisper.cpp model. |
| `local_openai_request_error` | Verify the configured OpenAI-compatible endpoint URL and network connectivity. |
| `local_openai_provider_error` | Check the configured model name and API key, then retry the Local LLM test. |
| `local_hy_mt2_not_available` | The offline Hy-MT2 engine is not enabled for live sessions yet (quality gate CAUTION); test it from settings or use the OpenAI-compatible engine. |
| `local_translation_backlog_full` | Use shorter utterances, a smaller/faster model, or more capable hardware. |
| `vieneu_model_not_installed` | Open Local LLM and select **Install VieNeu-TTS**. |
| `vieneu_install_failed` | Select **Resume setup**; existing verified download data is reused. |
| `vieneu_start_failed` | Select **Repair** or **Restart VieNeu-TTS** in Local LLM. |
| `local_vieneu_provider_error` | Restart VieNeu-TTS, then verify the selected preset voice and reading style. |
| `local_tts_voice_missing` | Refresh and reselect the configured system or VieNeu voice, then save and retest. |
| `local_tts_backlog_full` | Reduce local-model latency or pause until speech catches up. |
| `local_tts_playback_error` | Reconnect and reselect the translated output device. |

## Build Installer

Tauri produces target-native installers. Build on the operating system you intend to ship.

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

To create a Windows build, use a Windows machine or Windows CI with Node.js, Rust, CMake, `uv`, and Microsoft Visual Studio Build Tools (including C++) installed. The release script first creates the standalone VieNeu bridge and then builds the NSIS installer:

```powershell
npm ci
npm run release:windows
```

Expected Windows artifacts:

```text
src-tauri\target\release\bundle\nsis\
```

Run the Windows checks without producing an installer with `npm run release:windows:check`. See [the Windows user guide](docs/WINDOWS_TEAMS_USER_GUIDE.md) and [release guide](docs/WINDOWS_RELEASE_GUIDE.md).
