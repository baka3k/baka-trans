# Baka Trans on Windows

## Requirements

- Windows 10 version 2004 or newer; Windows 11 is recommended.
- Microsoft Teams desktop or web app.
- A speaker, wired headset, USB headset, or Bluetooth headset selected in Windows.
- A saved Google/OpenAI translation credential, or a tested Local LLM configuration.

## Start a Translation

The normal Windows workflow does not require BlackHole, VB-CABLE, or VoiceMeeter.

1. In Teams, keep **Speaker** set to the device you normally listen through.
2. Open Baka Trans and refresh **Audio routing**.
3. Under **Meeting source**, select **Teams audio (system output)** for that same speaker or headset. The Windows default output is selected automatically on first use.
4. Under **Translated audio**, select where the translated voice should play.
5. Use **Test translated** and confirm that you can hear the test sound.
6. Play meeting audio and confirm that the **Input signal** meter moves.
7. Start translation.

Windows captures the selected output with WASAPI loopback. The original Teams audio continues to play normally, so Windows does not show the extra original-audio monitor controls used by the macOS BlackHole workflow.

## Local Spoken Translation

For Japanese-to-Vietnamese meetings, choose **Local Whisper** at launch. Configure and test Whisper, Gemma in Ollama, and either an installed Vietnamese Windows voice or app-managed VieNeu-TTS under **Local LLM**. VieNeu-TTS installs with one button and needs no Python, terminal, or bridge URL. Select both **Meeting source** and **Translated audio**, choose the target channel, then use **Test selected voice**. The translated text appears on the conversation card before the selected TTS engine plays it through the selected output. See the [local setup and troubleshooting section](../README.md#local-japanese-to-vietnamese-translation).

## Bluetooth and Device Changes

- Prefer the stereo/headphones Bluetooth endpoint. Hands-free/headset mode has lower quality and may change when the microphone is active.
- If Windows changes the default output during a meeting, stop translation, refresh devices, reselect the matching **Teams audio (system output)** source, and start again.
- Sleep, docking, and unplugging a USB headset can invalidate the active audio stream. Stop and restart after the device returns.

## Troubleshooting

| Problem | Action |
| --- | --- |
| No system-output source | Refresh devices and confirm the output works in Windows Sound settings. |
| Input meter does not move | Select the loopback source whose device name matches Teams Speaker. Ensure Teams is currently playing audio. |
| Translated voice feeds back | Use headphones and do not select a speaker that a live microphone can hear. |
| Bluetooth audio becomes low quality | Select the stereo endpoint or use a wired/USB headset. |
| Loopback fails on a specific driver | Update the audio driver. As a last resort, install VB-CABLE and use its input/output pair. |
| Local Start is disabled | Save and test Local LLM settings after every runtime-critical edit, then select both meeting source and translated output. |
| No Vietnamese voice appears | For System TTS, install a Windows Vietnamese voice. For VieNeu-TTS, finish **Install VieNeu-TTS**, then select **Refresh voice list**. |
| Local translation is slow | Use a smaller/quantized multilingual Whisper model, a faster Ollama model, or reduce the maximum utterance duration. |

VB-CABLE is a fallback, not part of the normal setup. VoiceMeeter is intentionally not included because its mixer and routing controls are unnecessary for the standard Baka Trans workflow.

## Look Through and Look & Help

Windows uses native desktop-region capture and the installed Windows OCR language packs. If OCR is unavailable, install the Windows language pack for the text you want to recognize. Overlay windows are excluded from capture so their own output is not read back into the prompt.
