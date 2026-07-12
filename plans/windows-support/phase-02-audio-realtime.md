# Phase 02: Windows Audio Routing and Realtime Parity

Status: implemented; live Teams/hardware validation pending

## Goal

Prove the core meeting translation workflow on Windows before adding native overlay capture/OCR.

## Selected Windows Audio Approach

Use **WASAPI loopback** as the default meeting-audio source.

This is the simplest experience for the user because Windows can capture the audio already playing through the selected speaker/headset:

1. The user keeps the normal speaker/headset selected in Microsoft Teams.
2. In Baka Trans, the user selects **Teams audio (system output)**.
3. Baka Trans captures that output through WASAPI loopback and starts translation.

The user does not install an audio driver, create a virtual mixer, or reroute Teams to another device. Baka Trans should automatically select the current Windows default output and expose an advanced selector only when the user wants another playback device.

**VB-CABLE** is an optional troubleshooting fallback for hardware/drivers where loopback capture is unavailable or unreliable. **VoiceMeeter is not part of the normal workflow** because installing and configuring a mixer adds too many choices and creates more opportunities for echo and incorrect routing.

## Implementation Scope

1. Exercise `src-tauri/src/audio.rs` on Windows/WASAPI for device enumeration, common PCM formats, capture, playback, channel selection, hot-plug refresh, and shared-device contention.
2. Add a Windows-specific WASAPI loopback capture adapter behind `cfg(target_os = "windows")`. Keep `cpal` for normal microphone/input capture and playback; do not force macOS through the Windows adapter.
3. Expose a simple Windows source named **Teams audio (system output)**. It defaults to the current Windows output device and follows a default-device change when safe; advanced users can pin a specific output device.
4. Make routing help in `src/App.tsx` platform-aware. Windows instructions describe the three-step WASAPI workflow above; macOS keeps the existing BlackHole instructions.
5. Validate Google Live Translation, OpenAI fallback if retained, translated playback, session stop/restart, transcript export, and summaries on Windows. Because the user already hears the original Teams output normally, disable/hide the extra **Original audio monitor** path for the default Windows loopback workflow.
6. Add a Windows Teams routing guide and a compact manual test matrix covering headset, speakers, Bluetooth, mono/stereo devices, default-device changes, device removal, sleep/resume, and feedback prevention.
7. Document VB-CABLE only in troubleshooting as the fallback when WASAPI loopback cannot capture a specific output. Do not require or document VoiceMeeter in the standard setup.

## Verification

- Rust unit tests for platform-independent routing and sample conversion.
- Manual source/test-tone checks on at least one Windows 11 machine.
- A live Teams call using WASAPI loopback with Teams left on its normal speaker/headset output.
- A fallback smoke test using VB-CABLE on one loopback-incompatible or simulated-failure path.
- Confirm source meter, input/output transcript, translated playback, and clean shutdown/restart.
- Regression run on macOS with BlackHole.

## Exit Criteria

A Windows user can complete a Teams translation session without installing an audio driver or changing the Teams speaker setting. VB-CABLE is needed only as an explicitly documented fallback.
