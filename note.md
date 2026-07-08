# Tauri Desktop App Specification: Real-Time Meeting Translation for macOS

## 1. Purpose

Build a lightweight desktop app for macOS that captures audio from Microsoft Teams meetings, transcribes speech in real time, translates it from a source language to a target language, and plays the translated audio into the user’s headphones.

The primary use case is live meeting translation for one user listening privately, without changing the original meeting audio for other participants.

---

## 2. Goals

1. Capture meeting audio from Microsoft Teams on macOS.
2. Perform real-time speech-to-text conversion.
3. Translate speech from language A to language B.
4. Play translated speech back to the user through headphones or Bluetooth earphones.
5. Keep the app lightweight, reliable, and simple to start/stop.
6. Support transcript display for review during and after the meeting.

---

## 3. Non-Goals

* This version does not aim to provide perfect simultaneous interpretation quality.
* This version does not re-broadcast translated audio into Teams as a virtual microphone.
* This version does not provide meeting management features such as scheduling, recording, or attendee controls.
* This version does not require on-device model inference in the MVP.
* This version does not target Windows in the first release, though the architecture should remain portable.

---

## 4. Target Platform

* Operating System: macOS 13 or later
* Framework: Tauri
* UI Layer: React + TypeScript
* Backend: Rust
* Audio Routing: BlackHole 2ch or equivalent virtual audio device
* AI Services: OpenAI API for transcription, translation, and text-to-speech

---

## 5. Core User Story

As a user in a Teams meeting, I want the app to listen to the meeting audio, translate spoken content from one language into another in near real time, and play the translated result into my headphones so I can follow the conversation without reading captions.

---

## 6. User Flow

1. User opens the app.
2. User selects source language and target language.
3. User selects audio input source.
4. User selects playback output device.
5. User clicks Start.
6. The app captures Teams audio through a virtual audio route.
7. The app transcribes the incoming speech.
8. The app translates the transcript.
9. The app synthesizes translated audio.
10. The app plays translated audio to the user’s headphones.
11. The app shows live transcript and translated text in the UI.
12. User clicks Stop to end translation.

---

## 7. Functional Requirements

### 7.1 Meeting Audio Capture

* The app must capture audio from Microsoft Teams on macOS.
* The app should support BlackHole 2ch as the default virtual input source.
* The app should allow selecting from available system audio devices.
* The app should provide a setup guide for routing Teams audio through a virtual device.

### 7.2 Speech Recognition

* The app must transcribe incoming speech into text.
* The app should support automatic language detection.
* The app should support at least Japanese, English, and Vietnamese in the first release.
* The app should process speech in short chunks to reduce latency.

### 7.3 Translation

* The app must translate the transcript from source language to target language.
* The app should preserve technical terminology as much as possible.
* The app should support configurable translation style:

  * literal
  * natural
  * technical/meeting-safe

### 7.4 Text-to-Speech

* The app must synthesize translated text into audio.
* The app must play synthesized audio to the user’s selected headphones or output device.
* The app should allow voice selection if supported by the chosen TTS provider.

### 7.5 Transcript Display

* The app must display:

  * live source transcript
  * translated transcript
  * connection status
  * active language pair
* The app should keep a short transcript history for the current session.
* The app should allow exporting the transcript as plain text or Markdown.

### 7.6 Session Controls

* The app must provide Start and Stop controls.
* The app should support Pause and Resume.
* The app should show a live status indicator:

  * idle
  * listening
  * transcribing
  * translating
  * speaking
  * error

### 7.7 Error Handling

* The app must surface audio device errors clearly.
* The app must surface API authentication and quota errors clearly.
* The app must recover gracefully from temporary network failures.
* The app should retry transient failures automatically with backoff.

---

## 8. Non-Functional Requirements

### 8.1 Latency

* End-to-end latency target: 1 to 3 seconds for short utterances.
* Acceptable degradation: up to 5 seconds in poor network conditions.

### 8.2 Reliability

* The app should remain stable during meetings of at least 2 hours.
* The app should not crash when the selected audio device disappears temporarily.
* The app should preserve session state when possible.

### 8.3 Performance

* CPU usage should remain modest on Apple Silicon.
* Memory usage should remain reasonable for a background desktop app.
* The UI should remain responsive while audio processing runs in the background.

### 8.4 Privacy and Security

* The app should minimize local persistence of audio data.
* The app should not store raw meeting audio unless explicitly enabled.
* API keys must be stored securely, preferably in macOS Keychain.
* The app should clearly inform the user when audio is sent to external services.

### 8.5 Maintainability

* The app should separate UI, audio capture, AI pipeline, and device management into distinct modules.
* The app should support future replacement of AI providers without major UI changes.

---

## 9. Proposed Architecture

### 9.1 High-Level Flow

Teams Audio
→ BlackHole / virtual audio device
→ Audio capture module in Rust
→ Transcription API
→ Translation module
→ TTS API
→ Audio output module
→ Headphones

### 9.2 Suggested Components

#### Frontend

* Tauri window
* React UI
* Language selectors
* Device selectors
* Session controls
* Transcript panel

#### Backend

* Rust audio pipeline
* Device enumeration
* Stream buffering
* API client integration
* Session state management

#### External Services

* Speech-to-text
* Translation
* Text-to-speech

---

## 10. Recommended Technical Choices

### 10.1 Tauri

Use Tauri for the desktop shell because it keeps the app small and suitable for macOS distribution.

### 10.2 Rust Audio Layer

Use Rust for device handling and streaming logic to keep the pipeline reliable and efficient.

### 10.3 AI Provider

Use OpenAI APIs for:

* transcription
* translation
* speech synthesis

### 10.4 Audio Routing

Use BlackHole 2ch for MVP because it is simple and widely used on macOS.

---

## 11. MVP Scope

### Included in MVP

* macOS support only
* Teams audio capture via BlackHole
* Source language and target language selection
* Real-time transcription
* Real-time translation
* Audio playback to headphones
* Live transcript panel
* Start/Stop controls
* Basic error handling

### Excluded from MVP

* Speaker diarization
* Meeting summaries
* Searchable transcript archive
* Two-way speaking assistance
* Native Teams integration
* Windows support

---

## 12. MVP Acceptance Criteria

The MVP is successful if:

1. A user can route Teams audio into the app.
2. The app can detect and transcribe speech in real time.
3. The app can translate the detected speech into the selected target language.
4. The translated speech can be played into headphones.
5. The user can follow a normal Teams meeting with no manual intervention after pressing Start.
6. The app remains stable for at least one continuous meeting session.
7. The average latency stays within the target range for normal speech.

---

## 13. Suggested UX Screens

### 13.1 Home Screen

* Start / Stop
* Current status
* Language pair
* Device selection

### 13.2 Settings Screen

* Audio input device
* Audio output device
* API key
* Translation style
* TTS voice
* Auto-start on launch

### 13.3 Transcript Screen

* Source text
* Translated text
* Timestamped conversation history
* Export button

---

## 14. Setup Requirements for the User

Before first use, the user must:

1. Install BlackHole or another virtual audio device.
2. Configure Teams output to route into the virtual device.
3. Select the correct headphone output in the app.
4. Paste a valid API key.
5. Grant macOS microphone and audio permissions if needed.

---

## 15. Risks and Tradeoffs

### 15.1 Audio Routing Complexity

macOS audio routing can be confusing for non-technical users. The setup guide must be clear.

### 15.2 Latency vs Quality

Shorter chunks improve latency but can reduce translation quality. The chunking strategy must balance both.

### 15.3 API Cost

Real-time translation will consume API usage. Cost controls should be added early.

### 15.4 Meeting Accuracy

Technical jargon, overlapping speakers, and noisy meetings may reduce transcription quality.

### 15.5 Dependency on External Services

Network issues or API outages can interrupt the translation pipeline.

---

## 16. Open Questions

1. Should the app prioritize Japanese-to-Vietnamese only, or support multiple language pairs from day one?
2. Should TTS be optional, with transcript-only mode available?
3. Should the app cache transcript history locally?
4. Should the app offer speaker labels in later versions?
5. Should the app support push-to-talk for user responses in a future version?

---

## 17. Future Enhancements

* Two-way live translation
* Meeting summary generation
* Action item extraction
* Searchable transcript archive
* Speaker diarization
* Native Teams integration
* Windows support
* On-device transcription fallback
* Automatic language detection with confidence display

---

## 18. Definition of Done

The first release is done when a user on macOS can:

* install the app,
* configure audio routing,
* connect API credentials,
* start a Teams meeting translation session,
* hear translated audio in their headphones,
* and review the transcript afterward without the app crashing.
