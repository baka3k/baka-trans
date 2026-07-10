# Phase 15: Speaker Diarization Sidecar and Transcript Attribution

Status: planned
Depends on: phases 12, 13
Primary files: `src-tauri/src/audio.rs`, `src-tauri/src/session.rs`, `src-tauri/src/models.rs`, `src-tauri/src/commands.rs`, `src-tauri/src/diarization.rs`, `src/App.tsx`, `src/types.ts`, `src/transcript.ts`

## Objective

Add optional speaker diarization without moving speaker detection into the realtime translation hot path. Gemini Live Translation should continue to prioritize low-latency translated audio and transcript deltas. A sidecar diarization pipeline should process bounded source-audio windows in the background, identify likely speaker segments, and annotate transcript/export/summary state as labels become available.

## Current State

- `src-tauri/src/audio.rs` emits realtime PCM samples into the translation pipeline and audio-level events, but does not retain source audio.
- `src-tauri/src/session.rs` stores transcript items in memory and exposes snapshots for export and meeting summaries.
- `src-tauri/src/models.rs` has `TranscriptItem` with source text, translated text, status, and latency, but no speaker attribution fields.
- Gemini Live Translation plans in phases 12 and 13 provide input/output transcripts and playback, but Google Live docs describe VAD and transcriptions rather than reliable speaker diarization.
- Meeting summaries currently operate over transcript text. Speaker labels should become optional context for summaries after attribution exists.

## API Direction

1. Cloud Speech-to-Text diarization
   - Use as the first reliable provider path when speaker tags matter.
   - It exposes explicit diarization configuration such as minimum and maximum speaker counts.
   - It can return word-level speaker tags, which can be collapsed into utterance-level speaker segments for the app UI.
   - Credential setup may require Google Cloud project and OAuth/Application Default Credentials rather than only a Gemini API key.

2. Gemini audio structured-output diarization
   - Use as a secondary/post-meeting enrichment path.
   - It can analyze audio and produce structured outputs with speaker labels and timestamps.
   - It is useful when the user already has Gemini configuration and wants a simpler experiment, but timing and label stability must be treated as less deterministic than an explicit STT diarization API.

3. Gemini Live Translation
   - Keep for realtime translation and translated audio playback.
   - Use Live API VAD only for speech activity/turn hints, not speaker identity.

## Scope Challenge

Question 1: Should diarization block transcript rendering until speaker labels are ready?
Decision: No. Render transcript text immediately, then patch in speaker labels asynchronously. This keeps translation usable under meeting pressure.

Question 2: Should raw meeting audio be stored permanently for diarization?
Decision: No. Add an opt-in, bounded retention policy. Start with in-memory rolling windows when feasible; use encrypted temporary files only when window size or provider upload requirements make memory-only impractical. Delete windows after successful diarization or at session end.

Question 3: Should speaker labels be treated as exact identity?
Decision: No. Start with anonymous labels such as `Speaker 1` and `Speaker 2`, allow user rename later, and mark low-confidence or conflicting alignments as `needs_review`.

## Architecture

```text
Source capture stream
  -> realtime translation pipeline
  -> optional DiarizationBuffer
       fixed-duration PCM windows
       overlap for label continuity
       privacy retention policy
  -> DiarizationService
       trigger: interval/manual/session_end
       provider adapter: Cloud STT or Gemini audio
  -> SpeakerSegment[]
  -> TranscriptAligner
       timestamp/window overlap
       source item IDs
       confidence thresholds
  -> session transcript store
  -> Tauri events
  -> React transcript/export/summary UI
```

## Data Model

Recommended additions:

```text
DiarizationProvider:
  - google_cloud_speech_to_text
  - gemini_audio_analysis

DiarizationTrigger:
  - interval
  - manual
  - session_end

DiarizationRetentionMode:
  - memory_only
  - encrypted_temp_file_delete_on_complete

DiarizationConfig:
  - enabled
  - provider
  - trigger
  - minSpeakerCount
  - maxSpeakerCount
  - windowDurationSeconds
  - windowOverlapSeconds
  - retentionMode
  - confidenceThreshold

SpeakerSegment:
  - id
  - speakerLabel
  - startMs
  - endMs
  - sourceTranscriptItemIds
  - confidence
  - provider
  - status: pending | aligned | needs_review | error

TranscriptItem additions:
  - speakerLabel?: string
  - speakerSegmentId?: string
  - speakerConfidence?: number
```

## Implementation Steps

1. Add config and UI controls.
   - Add a compact diarization section near transcript/summary settings.
   - Include enable toggle, provider selector, speaker-count fields, trigger, and retention mode.
   - Show privacy copy in labels/tooltips, not as a blocking modal.
   - Disable provider options when credentials are missing.

2. Add bounded audio buffering.
   - Split source PCM into timestamped windows with configurable duration and overlap.
   - Keep buffering independent from translation send queues.
   - Track buffer byte size and dropped windows.
   - Delete buffers on stop, error, app close, or after successful provider processing.

3. Implement provider adapters.
   - Create `src-tauri/src/diarization.rs`.
   - Add a Cloud Speech-to-Text adapter with request construction and response parsing.
   - Add a Gemini audio adapter for structured-output post-processing if credentials/config are available.
   - Normalize both providers into `SpeakerSegment` results.

4. Align speaker segments to transcript items.
   - Add session-relative timestamps to transcript items if current timestamps are wall-clock only.
   - Match segment ranges to source transcript items by overlap.
   - Avoid overwriting an existing high-confidence label with a lower-confidence later result.
   - Emit `speaker-segments-update` or reuse `transcript-update` with enriched transcript items.

5. Update transcript display, export, and summary input.
   - Show speaker labels as compact badges in the transcript row.
   - Include speaker labels in Markdown/text export when present.
   - Pass speaker-labeled transcript text into the summary agent so action items and decisions can reference speakers when labels are available.
   - Keep summaries functional when no speaker labels exist.

6. Add diagnostics and error states.
   - Show diarization state: off, buffering, processing, aligned, needs review, error.
   - Track last provider error, last processed window, queue depth, and dropped windows.
   - Surface errors without interrupting translation.

## Acceptance Criteria

- The user can enable diarization explicitly and choose provider/settings before or during a session.
- Realtime translation, translated audio playback, and transcript deltas continue when diarization is processing.
- The app does not retain source audio beyond the configured bounded retention policy.
- A two-speaker fixture produces speaker labels attached to transcript items.
- Low-confidence/conflicting labels are marked `needs_review` rather than silently applied.
- Transcript export includes speaker labels when available.
- Meeting summary input can include speaker labels without requiring diarization.

## Verification

- Unit tests for diarization config validation and retention policy.
- Unit tests for PCM window chunking, overlap behavior, and cleanup.
- Unit tests for Cloud STT request construction and speaker-tag response parsing.
- Unit tests for Gemini structured-output parsing with malformed/partial JSON.
- Unit tests for transcript alignment by timestamp overlap and confidence threshold.
- Integration smoke test with a short two-speaker fixture.
- Manual test during a live translation session to confirm diarization does not cause audio dropouts, delayed playback, or blocked stop/pause.
- Privacy test that all temp buffers are deleted after stop/session end/provider success.

## Risks

- Speaker labels may drift across rolling windows unless overlapping context and label reconciliation are implemented carefully.
- Cloud Speech-to-Text credentials and project setup may be more complex than the existing Gemini API-key flow.
- Provider costs can grow if diarization windows are too frequent or too long.
- Retaining source audio, even briefly, changes privacy expectations and must be explicit.
- Teams/BlackHole mixed audio may contain overlapping speech, music, or notification sounds that reduce diarization accuracy.

## Out of Scope

- Biometric identification of real people.
- Guaranteed realtime speaker labels before translation output appears.
- Permanent searchable meeting archives.
- Speaker diarization as a hard dependency for meeting summaries.
