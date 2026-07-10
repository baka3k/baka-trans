# Baka Trans User Guide: BlackHole and Microsoft Teams on macOS

This guide takes you from installing BlackHole to running your first translated Microsoft Teams meeting with Baka Trans.

> [!IMPORTANT]
> The product is currently designed for macOS. In this guide, **BlackHole** means **BlackHole 2ch**. Do not select BlackHole as the Teams microphone.

## 1. What the setup does

Microsoft Teams cannot send its meeting audio directly to Baka Trans. BlackHole provides the virtual cable between the two apps:

```text
Remote participant in Teams
  -> Teams Speaker: BlackHole 2ch
  -> Baka Trans Meeting source: BlackHole 2ch
  -> realtime translation
  -> Baka Trans Translated audio: your headphones
```

Your microphone follows a separate path:

```text
Your headset or Mac microphone
  -> Teams Microphone
  -> other meeting participants
```

## 2. Before you start

You need:

- a Mac running the Teams desktop app;
- Baka Trans (`.dmg` or `.app`);
- BlackHole 2ch;
- stereo headphones or a headset, strongly recommended to prevent echo;
- a Google Gemini or OpenAI API key for translation; and
- permission to change the audio devices in Teams.

Connect your headphones before opening Teams and Baka Trans so both apps can detect them.

## 3. Recommended configuration

The easiest private setup uses one stereo headset. Original audio plays in one ear and translated audio in the other.

| App | Setting | Value |
| --- | --- | --- |
| Teams | Speaker | `BlackHole 2ch` |
| Teams | Microphone | Your real headset or Mac microphone |
| Baka Trans | Meeting source | `BlackHole 2ch` |
| Baka Trans | Translated audio | Your headphones |
| Baka Trans | Translated channel | `Right ear` |
| Baka Trans | Original audio monitor | On |
| Baka Trans | Monitor output | The same headphones |
| Baka Trans | Original channel | `Left ear` |

You may swap left and right. When the same stereo device is used for both outputs, the two channels **must be opposite**. Baka Trans disables original monitoring if the same device is configured as `All` for both routes.

## 4. Install BlackHole 2ch

### Option A: official installer

1. Open the [official BlackHole download page](https://existential.audio/blackhole/download/).
2. Download **BlackHole 2ch**. The 16-channel and 64-channel versions are not needed for this workflow.
3. Close Teams, Baka Trans, music players, browsers playing audio, and Audio MIDI Setup.
4. Save the package to `Downloads`.
5. Control-click the package, choose **Open**, and complete the installer.
6. Restart the Mac if the installer asks you to.

### Option B: Homebrew

If Homebrew is already installed, run:

```bash
brew install blackhole-2ch
```

Close and reopen audio apps after installation. Restart the Mac if the device does not appear.

### Verify the installation

1. Press `Command + Space`, search for **Audio MIDI Setup**, and open it.
2. Choose **Window > Show Audio Devices** if the device list is hidden.
3. Confirm that **BlackHole 2ch** appears in the left sidebar.

BlackHole is an audio driver, not a normal application, so it does not appear in the Applications folder.

## 5. Install and authorize Baka Trans

1. Open the Baka Trans `.dmg` supplied by your team.
2. Copy or install **Baka Trans.app** into the Applications folder as shown by the installer.
3. Open Baka Trans.
4. When macOS asks for microphone access, choose **Allow**. macOS treats BlackHole as an input device, so this permission is required even when Baka Trans is not listening to your physical microphone.
5. If the prompt was missed, open **System Settings > Privacy & Security > Microphone** and enable **Baka Trans**.

If macOS blocks an unsigned build, request a signed and notarized build from the distributor. Do not disable Gatekeeper globally.

## 6. Configure Microsoft Teams

Configure Teams before joining the real meeting:

1. Open the Teams desktop app and sign in.
2. Select **Settings and more (`...`) > Settings > Devices**.
3. Under **Audio settings**, set **Speaker** to **BlackHole 2ch**.
4. Set **Microphone** to your real headset microphone, USB microphone, or Mac microphone.
5. Keep the camera setting unchanged.

During a meeting, you can recheck the route from the arrow beside the microphone button, then choose **More audio settings**.

> [!WARNING]
> Never set the Teams microphone to BlackHole 2ch for this workflow. Doing so can send the meeting audio back into the call and create echo or a feedback loop.

At this point, it is normal to stop hearing Teams audio directly: Teams is sending it into BlackHole. Baka Trans will route the original and translated audio to your selected outputs.

## 7. Configure Baka Trans

### 7.1 Translation provider and API key

1. Select **Google** or **OpenAI** in Baka Trans. Google Live Translation is the default provider.
2. Obtain a key from the provider:
   - [Google Gemini API key instructions](https://ai.google.dev/gemini-api/docs/api-key)
   - [OpenAI API keys](https://platform.openai.com/api-keys)
3. Paste the key into the translation key field and select **Save**.
4. Select **Test key** and wait for a success message.

Baka Trans stores saved translation credentials through the backend and macOS Keychain. Do not paste API keys into chat, screenshots, logs, or shared documents.

### 7.2 Languages and session

In the **Session** panel:

1. Set **Source** to the language spoken in the meeting, or choose automatic detection when supported.
2. Set **Target** to the language you want to hear and read.
3. Make sure Source and Target are different.

### 7.3 Audio routing

In the **Audio routing** panel, configure the recommended headset setup:

1. Set **Meeting source** to **BlackHole 2ch**.
2. Set **Translated audio** to your headphones.
3. Set **Translated channel** to **Right ear**.
4. Enable **Original audio monitor**.
5. Set **Monitor output** to the same headphones.
6. Set **Original channel** to **Left ear**.

The audio device list refreshes automatically every five seconds. If BlackHole or your headphones do not appear, close and reopen the app after checking the device in Audio MIDI Setup.

### 7.4 Test every route

Run these checks while no translation session is active:

1. Select **Test translated**. You should hear a tone in the translated ear, then stop the tone.
2. Select **Test original**. You should hear a tone in the original-audio ear, then stop the tone.
3. Start a Teams test call or play audio from a Teams meeting.
4. Select **Monitor mic to output** in Baka Trans. With BlackHole selected, this temporarily monitors the Teams source rather than your physical microphone.
5. Confirm that the input meter moves and the source status becomes **Receiving audio**.
6. Select **Stop mic monitor** before starting translation.

Do not continue until both output tests work and the BlackHole input meter moves.

## 8. Run the first translated meeting

1. Connect the headphones.
2. Open Baka Trans and verify the saved API key, languages, and audio routes.
3. Open Teams and verify:
   - Speaker: `BlackHole 2ch`
   - Microphone: your real microphone
4. Join the meeting.
5. In Baka Trans, select **Start**.
6. Ask another participant to speak, or wait for meeting audio.
7. Confirm the live status:
   - **Receiving audio**: Teams audio is reaching Baka Trans.
   - **Source silent**: the connection is active, but nobody is speaking.
   - **No recent audio**: the audio route is stale or disconnected.
   - **Capture error**: Baka Trans could not capture the selected device.
8. Confirm that source and translated text appear and that translated audio plays through the selected channel.
9. Select **Stop** when the meeting ends.

`Pause`, `Resume`, and `Translate now` are available only for providers and session states that support them.

## 9. Other useful routing configurations

### Separate outputs: original on Mac speakers, translation in headphones

Use this when you do not want left/right split audio:

| Baka Trans setting | Value |
| --- | --- |
| Meeting source | `BlackHole 2ch` |
| Translated audio | Headphones |
| Translated channel | `All` |
| Original audio monitor | On |
| Monitor output | MacBook speakers or another device |
| Original channel | `All` |

Keep the speaker volume low enough that the Teams microphone does not pick it up. Headphones are safer when other participants could hear echo.

### macOS Multi-Output Device

Use a Multi-Output Device when Teams should send the original audio directly to both BlackHole and a physical output. In this mode, leave **Original audio monitor** off in Baka Trans.

1. Open **Audio MIDI Setup > Window > Show Audio Devices**.
2. Select the `+` button and choose **Create Multi-Output Device**.
3. Rename it to `Teams + BlackHole`.
4. Enable your headphones or Mac speakers and **BlackHole 2ch**.
5. Choose the physical output as the primary device.
6. Enable drift correction for the device that is not the primary clock device.
7. Make sure the devices use the same sample rate; `48.0 kHz` is a common meeting-audio setting.
8. In Teams, set **Speaker** to `Teams + BlackHole`.
9. In Baka Trans, keep **Meeting source** set to `BlackHole 2ch` and turn **Original audio monitor** off.

macOS may disable the keyboard volume controls for Multi-Output Devices. Adjust the physical output volume before the meeting or in Audio MIDI Setup.

See Apple's [Multi-Output Device guide](https://support.apple.com/guide/audio-midi-setup/play-audio-through-multiple-devices-at-once-ams7c093f372/mac) and the [BlackHole Multi-Output guide](https://github.com/ExistentialAudio/BlackHole/wiki/Multi-Output-Device) for the current macOS steps.

## 10. Troubleshooting

| Symptom | What to check |
| --- | --- |
| BlackHole does not appear | Confirm it exists in Audio MIDI Setup, then restart Teams and Baka Trans. Restart the Mac if it was just installed. |
| **Start** is disabled | Save a translation API key, select a Meeting source and Translated audio device, stop active test tones/monitoring, and select a Monitor output if Original audio monitor is enabled. |
| Input shows **Waiting** or **No recent audio** | Confirm Teams Speaker is `BlackHole 2ch`, meeting audio is actually playing, and Baka Trans has microphone permission. |
| Input shows **Source silent** | The route is connected. Ask someone to speak and check that Teams is not muted at the speaker/output level. |
| Input shows **Capture error** | Stop the session, close other apps holding the device, refresh the device list, reselect BlackHole, and start again. |
| Original Teams audio cannot be heard | This is expected when Teams sends only to BlackHole. Enable Original audio monitor with a valid output, use opposite headset channels, or use a Multi-Output Device. |
| Translated audio cannot be heard | Stop the session and use **Test translated**. Confirm the correct headphones and channel are selected and the device volume is not muted. |
| Original monitor does not work on the same headset | The original and translated routes overlap. Set one to `Left ear` and the other to `Right ear`, or use separate devices. |
| Other participants hear echo | Confirm Teams Microphone is the real microphone, not BlackHole. Prefer headphones and disable speaker monitoring. |
| Audio crackles with Multi-Output | Use the same sample rate for all devices and enable drift correction for every non-primary device. |
| API key test fails | Confirm the selected provider matches the key, the account has access/quota, and the Mac has internet access. Replace the saved key if necessary. |
| Devices changed after reconnecting Bluetooth headphones | Wait for automatic refresh or reopen Baka Trans, then reselect and retest every output. |

## 11. Return Teams to normal audio

After the meeting:

1. Select **Stop** in Baka Trans.
2. In Teams, open **Settings > Devices**.
3. Change **Speaker** from BlackHole or the Multi-Output Device back to your normal headphones or Mac speakers.
4. Leave **Microphone** set to your real microphone.

If you forget this step, Teams may appear silent the next time Baka Trans is closed.

## 12. Quick pre-meeting checklist

- [ ] Headphones connected
- [ ] BlackHole 2ch visible in Audio MIDI Setup
- [ ] Baka Trans has microphone permission
- [ ] Translation provider selected and API key test passes
- [ ] Source and Target languages are different
- [ ] Teams Speaker is BlackHole 2ch or the configured Multi-Output Device
- [ ] Teams Microphone is a real microphone, not BlackHole
- [ ] Baka Trans Meeting source is BlackHole 2ch
- [ ] **Test translated** works
- [ ] **Test original** works when original monitoring is enabled
- [ ] Input meter moves when Teams audio plays
- [ ] Temporary mic/source monitor is stopped before selecting **Start**

## 13. Optional: build Baka Trans locally

For development or internal testing, run these commands from the repository root:

```bash
npm ci
npm run tauri -- build --bundles dmg
```

The generated installer is written under:

```text
src-tauri/target/release/bundle/dmg/
```

Public distribution requires Apple code signing and notarization.

## Official references

- [BlackHole project and installation options](https://github.com/ExistentialAudio/BlackHole)
- [BlackHole installer instructions](https://github.com/ExistentialAudio/BlackHole/wiki/Installation)
- [Microsoft Teams device settings](https://support.microsoft.com/en-us/teams/notifications-settings/manage-your-device-settings-in-microsoft-teams)
- [Apple Audio MIDI Setup guide](https://support.apple.com/guide/audio-midi-setup/set-up-audio-devices-ams59f301fda/mac)

