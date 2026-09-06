# Phase 01 — Bounded Speech Queue with Coalescing and Drop-Oldest

Parent: [plan.md](plan.md)

Covers user-selected optimizations **2** (coalesce instead of drop), **3**
(capacity 4 → 12), and **5** (drop-oldest fallback).

## Objective

Replace the 4-slot `tokio::sync::mpsc` TTS queue with a shared bounded
`SpeechQueue` whose producer can coalesce, so a backlog no longer destroys
content. A true drop happens only when (a) a consumer stall pushes even the
coalesce bound past its limit, or (b) queued speech ages past
`MAX_QUEUE_AGE_MS` — and then it drops the oldest, not the newest, with an
explicit error.

## New module: `src-tauri/src/ai/speech_queue.rs`

Register `mod speech_queue;` in `src-tauri/src/ai.rs` next to
`mod local_worker;`. The module is self-contained and takes `TtsRequest` from
`local_worker.rs` — move `TtsRequest` (`local_worker.rs:93-96`,
`{ utterance_id: String, translated_text: String }`) into `speech_queue.rs`
and re-export/use it from there, so the queue owns its element type without a
circular import.

```rust
pub(crate) const SPEECH_QUEUE_CAPACITY: usize = 12;
pub(crate) const MAX_COALESCED_CHARS: usize = 600;
pub(crate) const MAX_QUEUE_AGE_MS: u64 = 10_000;

#[derive(Debug)]
pub(crate) enum PushOutcome {
    Queued,
    Coalesced,
    DroppedOldest { dropped_id: String },
}

pub(crate) struct SpeechQueue {
    state: std::sync::Mutex<SpeechQueueState>,
    notify: tokio::sync::Notify,
}

struct SpeechQueueState {
    queue: VecDeque<TtsRequest>,   // TtsRequest gains `enqueued_at: Instant`
    capacity: usize,
    closed: bool,
}
```

Design rules:

- **`std::sync::Mutex`, never held across `.await`.** Both `push` and `close`
  are fully synchronous; `pop` locks, checks, drops the guard, then awaits
  `notify.notified()`. This matches the repo's existing sync-lock idiom
  (`transcript_store` in `local_worker.rs:763-770`). Lock acquisition uses
  `lock().unwrap_or_else(|poisoned| poisoned.into_inner())` — the queue's
  VecDeque ops cannot panic, so recovering the (consistent) data and
  continuing beats poisoning the queue forever.
- **Wakeup safety — permit-based, never `notify_waiters` alone.**
  `tokio::sync::Notify::notify_waiters` stores no permit: a `pop()` that
  checks state, unlocks, and has not yet polled its `Notified` future when
  `close()` fires would sleep forever. Red-team finding; rule:
  - `push` signals with `notify_one()` (permit-storing — a permit is stored
    when no waiter is registered, so a push is never missed).
  - `close` signals with `notify_one()` **and** `notify_waiters()` (the
    permit covers the single-consumer registration race; the broadcast keeps
    N-consumer futures correct).
  - `pop()` additionally re-locks and re-checks state after every awaited
    wakeup (spurious-wakeup hygiene).
- **Latency bound, not just memory bound.** Coalescing alone would let a
  stalled consumer accumulate a silent 30-40 s speech backlog with no error
  (red-team finding): for live translation, stale speech is a failure even
  when lossless. `push` therefore also sheds the FRONT item when it is older
  than `MAX_QUEUE_AGE_MS` (10 s) — the same drop-oldest outcome and error
  path as capacity overflow. A live `TtsRequest.enqueued_at: Instant` is set
  in `push`; the age check runs before the capacity/coalesce logic each push.
- **`push` algorithm** (loop until an outcome is returned):

```text
lock state (poison-recovering)
req.enqueued_at = Instant::now()
shed: Option<String> = None
loop {
    if state.closed            -> unlock; return the Closed outcome variant
                                  // shutdown race: caller sees it and stays silent
    while front is older than MAX_QUEUE_AGE_MS {
        shed = Some(queue.pop_front().unwrap().utterance_id)   // stale: shed oldest
    }
    if queue.len() < capacity  -> push_back(req); notify_one;
                                  return shed.map(DroppedOldest).unwrap_or(Queued)
    tail = queue.back_mut().unwrap()                 // capacity >= 1: debug_assert! in new()
    if tail.chars().count() + 1 + req.chars().count() <= MAX_COALESCED_CHARS {
                                   // "+1" counts the separator (red-team off-by-one fix)
        tail.translated_text.push(' ');
        tail.translated_text.push_str(&req.translated_text);
        notify_one; return shed.map(DroppedOldest).unwrap_or(Coalesced)
    }
    shed = Some(queue.pop_front().unwrap().utterance_id);   // stall: shed the oldest
    // loop continues; with a slot now free the next iteration either Queues
    // or Coalesces — the loop terminates because capacity >= 1 and the
    // front is strictly decreasing in age
}
```

  If any request was shed while making room (capacity or age), return
  `DroppedOldest { dropped_id }` carrying the SHED id (not the new one).

- **`pop().await -> Option<TtsRequest>`**: lock; `pop_front()` if non-empty;
  else `closed → None`; unlock; `notify.notified().await`; repeat (the
  permit rules above make the close race impossible).
- **`close()`**: set `closed = true`, then `notify_one()` + `notify_waiters()`.
  Idempotent.
- **`new(capacity)`**: `debug_assert!(capacity >= 1)` — capacity 0 would
  turn the push loop into a `back_mut().unwrap()` panic (red-team finding).

Separator on coalesce: a single ASCII space, matching the pipeline's existing
normalization (`split_sentences` collapses whitespace at
`local_worker.rs:818`, `transcribe` at `:1034`). Vietnamese target text is
space-delimited; a stray space before CJK punctuation is harmless to TTS.

## Producer changes — `SentenceTranslator` (`local_worker.rs:504-529`)

Replace the `tts_tx.try_send(...)` match:

- `PushOutcome::Queued | PushOutcome::Coalesced` → nothing (no event, no log
  noise; content is fully preserved, so the sticky frontend error banner at
  `MainApp.tsx:2398-2402` must not fire).
- `PushOutcome::DroppedOldest { dropped_id }` (a live shed — capacity stall
  or age bound) → emit the existing
  `local_tts_backlog_full` app-error with an updated message naming the
  skipped utterance, mirroring the playback-path format at
  `local_worker.rs:604-610`. The message must cover both shed reasons
  (capacity stall and `MAX_QUEUE_AGE_MS` staleness):
  `"Local speech cannot keep up. The oldest queued sentence was skipped to stay live; its translated text remains in the transcript. Utterance {dropped_id}."`

Distinguish "dropped because closed" (shutdown — stay silent) from "dropped
while live": the outcome enum carries a distinct `Closed` variant rather than
relying on a racy post-hoc `is_closed()` read (red-team note).

Note: the current `TrySendError::Closed → local_tts_worker_closed`
diagnostic (`local_worker.rs:518-528`) has no shared-queue equivalent — the
producer cannot observe consumer death on an `Arc<Mutex>` (red-team finding;
accepted). Removal is compensated by the existing drain ladder: a dead TTS
task surfaces as `local_translation_join_error` via `drain_worker`
(`local_worker.rs:1206`). Do not keep a half-migrated diagnostic.

## Lifecycle changes — `run_local_translation` (`local_worker.rs:159-311`)

- `let speech_queue = Arc<SpeechQueue>::new(SpeechQueue::new(SPEECH_QUEUE_CAPACITY));`
  replaces `let (tts_tx, tts_rx) = mpsc::channel(TTS_QUEUE_CAPACITY);`.
  Clone into `SentenceTranslator` and `TtsWorker` as today. Remove the
  `TTS_QUEUE_CAPACITY` const (`local_worker.rs:30`).
- **`close()` placement mirrors mpsc ownership:** today `tts_tx` lives inside
  `SentenceTranslator` (`local_worker.rs:78`), so the TTS channel closes only
  when the translator task exits — NOT when the loop's stop paths drop
  `utterance_tx`/`translation_tx` (those two drops stay as-is; there is no
  "TTS half" to replace there — red-team finding). Therefore:
  - Do NOT call `close()` in the three in-loop stop paths.
  - Call `speech_queue.close()` in `run_local_translation` **between
    `drain_worker(&mut translator, ...)` and
    `drain_worker(&mut tts_worker, ...)`** (all three return paths:
    `local_worker.rs:240-243, 294-296, 305-307`).
  - This preserves the pause/natural-end path exactly: translation jobs that
    finish during the translator's drain window are pushed while the queue is
    still open and get spoken during the TTS drain — matching today's
    open-channel behavior.

## Stop/drain semantics (red-team corrected — preserve BOTH real paths)

Current behavior, verified: the loop's `Stop` branch sets `cancellation`
**before** draining (`local_worker.rs:291`), so the TTS worker exits at its
loop-top `is_worker_active` check after at most the in-flight sentence —
queued sentences are discarded unspoken today. The pause/natural-end path
(`audio_rx → None`, `local_worker.rs:227-243`) never sets cancellation, so
draining there speaks everything queued. `SpeechQueue` must reproduce both:

- Stop path: `cancellation` set → worker breaks at loop top regardless of
  queue contents (unchanged code) → queued speech discarded, as today.
- Pause/natural-end path: no cancellation → worker pops until the queue is
  empty, then `pop()` returns `None` (queue closed after the translator
  drained) → queued speech is spoken during the drain window, as today.

The parity test must exercise the stop path truthfully: set `cancellation`
BEFORE `close()`, and assert the consumer exits via the active check rather
than draining (a drain-only test proves nothing about Stop — red-team
finding).

## Tests (`speech_queue.rs` `mod tests`, sync unless noted)

- `push_queues_up_to_capacity_then_coalesces`: fill to 12; the 13th push
  returns `Coalesced`; the tail item's text is `tail + " " + new`; queue
  length stays 12; popped order is FIFO.
- `coalesce_respects_max_chars`: with a tail within
  `MAX_COALESCED_CHARS - 2` of the bound plus a request that would overflow
  **including the separator** (`tail + 1 + req > MAX`), push returns
  `DroppedOldest` carrying the FRONT request's id; queue length never
  exceeds capacity; the new request ends up queued.
- `push_sheds_stale_front`: seed a request with `enqueued_at` older than
  `MAX_QUEUE_AGE_MS` (accept an internal `#[cfg(test)]` seam or construct
  via `Instant::now() - Duration`), push a fresh request → `DroppedOldest`
  naming the stale id, and the fresh request is queued. Guards the
  silent-latency regression (red-team finding).
- `pop_returns_fifo_then_none_after_close` (`#[tokio::test]`): push a, b;
  close; pop yields a, b, then `None`.
- `push_after_close_returns_closed_outcome`: close, then push → the
  closed/`DroppedOldest` outcome with `is_closed()` true (no error path).
- `pop_wakes_on_push` (`#[tokio::test]`): spawn a `pop()` waiter, assert it
  has not resolved, `push`, assert it resolves with the item.
- `pop_wakes_on_close_without_registered_waiter` (`#[tokio::test]`, red-team
  lost-wakeup regression): spawn a `pop()` waiter, yield a few ticks so it
  is parked in the check-unlock-await window, then `close()` — the waiter
  must resolve to `None` without any push. (If this test ever flakes, the
  permit rule for `close()` is broken.)
- `consumer_exits_on_cancellation_before_close` (`#[tokio::test]`): model
  the real Stop path — cancellation flag set first, then close; the
  consumer loop's active-check exits without draining (documents the true
  stop semantics, red-team finding).
- `coalesce_separator_is_single_space`: exact string assertion.
- `coalesce_budget_counts_separator`: tail length `MAX - 2` plus request
  length 1 coalesces (total `MAX`); plus request length 2 sheds instead.

Existing tests to update: none reference the TTS mpsc directly, but
`local_worker.rs` tests stay green; the `TtsRequest` move (plus the new
`enqueued_at` field and the deliberate absence of `impl Debug` on
`TtsRequest` — translated text must stay out of any future debug dumps,
red-team hardening) must keep `local_worker.rs` compiling (update imports
only).

## Acceptance

- `cargo test`, `cargo fmt --check`, `cargo clippy --all-targets -- -D
  warnings` pass on macOS.
- Behavior: during a live session, overflowing the queue no longer drops the
  newest sentence and no longer raises the sticky banner; text appears in the
  transcript exactly once and is spoken exactly once (coalesced audio).
- Manual stop mid-session still ends speech immediately.
