# Stability & Error Handling

## Local pipeline speech backlog (Whisper → translate → TTS)

The local spoken pipeline keeps translation text intact even when speech falls
behind. Three bounds protect the bounded speech queue between translation and
synthesis (`src-tauri/src/ai/speech_queue.rs`):

- **Coalesce on overflow.** When the queue (capacity 12) is full, a new
  sentence merges into the queued tail instead of being dropped. Content is
  preserved and no error is raised.
- **Drop oldest, never newest.** If the tail cannot absorb more text
  (`MAX_COALESCED_CHARS` = 600) or the queue front is older than
  `MAX_QUEUE_AGE_MS` = 10 s, the oldest queued sentence is shed and
  `local_tts_backlog_full` names the skipped utterance. The age bound exists so
  a stalled synthesizer cannot hide as a silent 30-40 s speech lag.
- **Pipelined playback.** Synthesis runs ahead of playback (bounded by 3
  buffers in flight), so per-sentence TTS throughput is `max(synthesis,
  playback)` instead of their sum; a speaking user no longer accumulates
  permanent backlog.

Stopping a session still cuts audio immediately and discards queued speech;
pausing lets the current sentence plus up to 3 already-synthesized sentences
finish. `local_tts_playback_error` (output overloaded/disconnected) is
suppressed during the stop race, when it is teardown noise rather than a live
failure.

