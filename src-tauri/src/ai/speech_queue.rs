//! Bounded speech queue between the translation worker and the TTS worker.
//!
//! Replaces a plain `tokio::sync::mpsc` channel because the producer must be
//! able to inspect and mutate queued requests: on overflow it merges the new
//! sentence into the queue tail (coalescing) instead of dropping content, and
//! as a last resort sheds the OLDEST queued sentence so the freshest speech
//! survives. Memory stays bounded by [`SPEECH_QUEUE_CAPACITY`] and
//! [`MAX_COALESCED_CHARS`]; latency stays bounded by [`MAX_QUEUE_AGE_MS`],
//! which sheds stale fronts so a stalled consumer cannot hide behind silent
//! coalescing. Speech order is preserved: strict FIFO pops plus tail-merge
//! coalescing never reorder queued sentences.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use tokio::sync::Notify;

/// Queued sentences awaiting synthesis. Bounded so a stalled TTS worker can
/// never grow memory indefinitely.
pub(crate) const SPEECH_QUEUE_CAPACITY: usize = 12;
/// Upper bound on the characters of one coalesced request (~30 s of speech).
/// Past this bound the queue sheds its oldest request instead of merging.
pub(crate) const MAX_COALESCED_CHARS: usize = 600;
/// Queued speech older than this is shed so speech never lags silently beyond
/// a live-translation-usable window, even when coalescing still has room.
pub(crate) const MAX_QUEUE_AGE_MS: u64 = 10_000;

/// One sentence awaiting speech synthesis.
///
/// Deliberately no `impl Debug`: translated text must stay out of any future
/// debug dumps, logs, or error payloads — only `utterance_id` ever surfaces.
pub(crate) struct TtsRequest {
    pub(crate) utterance_id: String,
    pub(crate) translated_text: String,
    enqueued_at: Instant,
}

impl TtsRequest {
    /// `enqueued_at` is stamped by `push` right before the queue admits the
    /// request, so the age bound measures real wait time, not construction time.
    pub(crate) fn new(utterance_id: String, translated_text: String) -> Self {
        Self {
            utterance_id,
            translated_text,
            enqueued_at: Instant::now(),
        }
    }
}

#[derive(Debug)]
pub(crate) enum PushOutcome {
    /// Queued behind the existing requests.
    Queued,
    /// Merged into the tail request. Content is fully preserved — no error.
    Coalesced,
    /// The queue shed its OLDEST request to admit this one (capacity stall or
    /// `MAX_QUEUE_AGE_MS` staleness). `dropped_id` names the shed request.
    DroppedOldest { dropped_id: String },
    /// The queue is closed; the request was discarded during shutdown.
    Closed,
}

pub(crate) struct SpeechQueue {
    state: Mutex<SpeechQueueState>,
    notify: Notify,
}

struct SpeechQueueState {
    queue: VecDeque<TtsRequest>,
    capacity: usize,
    closed: bool,
}

impl SpeechQueue {
    pub(crate) fn new(capacity: usize) -> Self {
        debug_assert!(
            capacity >= 1,
            "capacity 0 would turn the push loop into a back_mut() panic"
        );
        Self {
            state: Mutex::new(SpeechQueueState {
                queue: VecDeque::new(),
                capacity,
                closed: false,
            }),
            notify: Notify::new(),
        }
    }

    fn lock_state(&self) -> MutexGuard<'_, SpeechQueueState> {
        // VecDeque operations cannot panic, so a poisoned queue still holds
        // consistent data: recover it instead of failing every future call.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Synchronous, never awaits under the lock: a stalled consumer cannot
    /// block the translation worker.
    pub(crate) fn push(&self, mut request: TtsRequest) -> PushOutcome {
        request.enqueued_at = Instant::now();
        self.push_inner(request)
    }

    fn push_inner(&self, request: TtsRequest) -> PushOutcome {
        let mut shed: Option<String> = None;
        let mut state = self.lock_state();
        loop {
            if state.closed {
                return PushOutcome::Closed;
            }
            while state.front_is_older_than(MAX_QUEUE_AGE_MS) {
                let dropped = state.queue.pop_front().expect("front checked just now");
                shed = Some(dropped.utterance_id);
            }
            if state.queue.len() < state.capacity {
                state.queue.push_back(request);
                drop(state);
                self.notify.notify_one();
                return match shed {
                    Some(dropped_id) => PushOutcome::DroppedOldest { dropped_id },
                    None => PushOutcome::Queued,
                };
            }
            let tail = state
                .queue
                .back_mut()
                .expect("capacity >= 1 keeps the queue non-empty here");
            // "+1" counts the separator the merge inserts.
            let merged_chars =
                tail.translated_text.chars().count() + 1 + request.translated_text.chars().count();
            if merged_chars <= MAX_COALESCED_CHARS {
                tail.translated_text.push(' ');
                tail.translated_text.push_str(&request.translated_text);
                drop(state);
                self.notify.notify_one();
                return match shed {
                    Some(dropped_id) => PushOutcome::DroppedOldest { dropped_id },
                    None => PushOutcome::Coalesced,
                };
            }
            let dropped = state.queue.pop_front().expect("queue is at capacity >= 1");
            shed = Some(dropped.utterance_id);
        }
    }

    /// Returns `None` only after the queue is closed AND fully drained, so the
    /// TTS worker keeps speaking everything queued when the session ends
    /// naturally, and exits only when no more speech exists.
    pub(crate) async fn pop(&self) -> Option<TtsRequest> {
        loop {
            {
                let mut state = self.lock_state();
                if let Some(request) = state.queue.pop_front() {
                    return Some(request);
                }
                if state.closed {
                    return None;
                }
            }
            self.notify.notified().await;
        }
    }

    /// Signals consumer termination. `notify_one` stores a permit for a
    /// consumer that has already checked the state but not yet registered its
    /// `Notified` future; `notify_waiters` additionally releases any
    /// already-registered waiters. Together they close the lost-wakeup window.
    pub(crate) fn close(&self) {
        {
            let mut state = self.lock_state();
            if state.closed {
                return;
            }
            state.closed = true;
        }
        self.notify.notify_one();
        self.notify.notify_waiters();
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.lock_state().closed
    }
}

impl SpeechQueueState {
    fn front_is_older_than(&self, max_age_ms: u64) -> bool {
        self.queue
            .front()
            .is_some_and(|front| front.enqueued_at.elapsed() > Duration::from_millis(max_age_ms))
    }
}

#[cfg(test)]
impl SpeechQueue {
    fn push_with_enqueued_at(&self, mut request: TtsRequest, enqueued_at: Instant) -> PushOutcome {
        request.enqueued_at = enqueued_at;
        self.push_inner(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    fn request(id: &str, text: &str) -> TtsRequest {
        TtsRequest::new(id.to_string(), text.to_string())
    }

    async fn drain_closed(queue: &SpeechQueue) -> Vec<TtsRequest> {
        queue.close();
        let mut items = Vec::new();
        while let Some(item) = queue.pop().await {
            items.push(item);
        }
        items
    }

    #[tokio::test]
    async fn push_queues_up_to_capacity_then_coalesces() {
        let queue = SpeechQueue::new(SPEECH_QUEUE_CAPACITY);
        for index in 0..SPEECH_QUEUE_CAPACITY {
            let outcome = queue.push(request(&format!("u{index}"), &format!("s{index}")));
            assert!(matches!(outcome, PushOutcome::Queued));
        }
        assert!(matches!(
            queue.push(request("u-merged", "extra")),
            PushOutcome::Coalesced
        ));
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), SPEECH_QUEUE_CAPACITY);
        assert_eq!(items[0].utterance_id, "u0");
        let tail = items.last().expect("queue was filled");
        assert_eq!(tail.utterance_id, format!("u{}", SPEECH_QUEUE_CAPACITY - 1));
        assert_eq!(
            tail.translated_text,
            format!("s{} extra", SPEECH_QUEUE_CAPACITY - 1)
        );
    }

    #[tokio::test]
    async fn coalesce_respects_max_chars() {
        let queue = SpeechQueue::new(2);
        let near_bound = "a".repeat(MAX_COALESCED_CHARS - 2);
        assert!(matches!(
            queue.push(request("u1", &near_bound)),
            PushOutcome::Queued
        ));
        assert!(matches!(
            queue.push(request("u2", "b")),
            PushOutcome::Queued
        ));
        // Queue is full; the tail cannot absorb the oversized request without
        // passing MAX_COALESCED_CHARS, so the front is shed to make room.
        let outcome = queue.push(request("u3", &"c".repeat(MAX_COALESCED_CHARS)));
        match outcome {
            PushOutcome::DroppedOldest { dropped_id } => assert_eq!(dropped_id, "u1"),
            other => panic!("unexpected outcome: {other:?}"),
        }
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].utterance_id, "u2");
        assert_eq!(items[1].utterance_id, "u3");
        assert_eq!(
            items[1].translated_text.chars().count(),
            MAX_COALESCED_CHARS
        );
    }

    #[tokio::test]
    async fn push_sheds_stale_front() {
        let queue = SpeechQueue::new(4);
        let stale_at = Instant::now() - Duration::from_millis(MAX_QUEUE_AGE_MS + 1);
        queue.push_with_enqueued_at(request("stale", "old text"), stale_at);
        let outcome = queue.push(request("fresh", "new text"));
        match outcome {
            PushOutcome::DroppedOldest { dropped_id } => assert_eq!(dropped_id, "stale"),
            other => panic!("unexpected outcome: {other:?}"),
        }
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].utterance_id, "fresh");
    }

    #[tokio::test]
    async fn pop_returns_fifo_then_none_after_close() {
        let queue = SpeechQueue::new(4);
        queue.push(request("a", "one"));
        queue.push(request("b", "two"));
        queue.close();
        assert_eq!(queue.pop().await.expect("first item").utterance_id, "a");
        assert_eq!(queue.pop().await.expect("second item").utterance_id, "b");
        assert!(queue.pop().await.is_none());
    }

    #[test]
    fn push_after_close_returns_closed_outcome() {
        let queue = SpeechQueue::new(4);
        queue.close();
        assert!(queue.is_closed());
        assert!(matches!(
            queue.push(request("late", "dropped silently")),
            PushOutcome::Closed
        ));
    }

    #[tokio::test]
    async fn pop_wakes_on_push() {
        let queue = Arc::new(SpeechQueue::new(4));
        let waiter_queue = Arc::clone(&queue);
        let waiter = tokio::spawn(async move { waiter_queue.pop().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiter.is_finished());
        queue.push(request("a", "wake up"));
        let item = tokio::time::timeout(Duration::from_millis(500), waiter)
            .await
            .expect("waiter must resolve after a push")
            .expect("waiter task must not panic")
            .expect("pushed item must be delivered");
        assert_eq!(item.utterance_id, "a");
    }

    #[tokio::test]
    async fn pop_wakes_on_close_without_registered_waiter() {
        let queue = Arc::new(SpeechQueue::new(4));
        let waiter_queue = Arc::clone(&queue);
        let waiter = tokio::spawn(async move { waiter_queue.pop().await });
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!waiter.is_finished());
        queue.close();
        let item = tokio::time::timeout(Duration::from_millis(500), waiter)
            .await
            .expect("close must wake the parked consumer")
            .expect("waiter task must not panic");
        assert!(item.is_none());
    }

    #[tokio::test]
    async fn consumer_exits_on_cancellation_before_close() {
        let queue = Arc::new(SpeechQueue::new(4));
        queue.push(request("a", "one"));
        queue.push(request("b", "two"));
        let cancellation = Arc::new(AtomicBool::new(false));
        // Real Stop order: cancellation is set BEFORE the queue ever closes,
        // so the worker must exit at its loop-top active check and abandon
        // whatever is still queued.
        cancellation.store(true, Ordering::SeqCst);
        queue.close();
        let mut consumed = 0;
        while let Some(_request) = queue.pop().await {
            if cancellation.load(Ordering::SeqCst) {
                break;
            }
            consumed += 1;
        }
        assert_eq!(consumed, 0);
    }

    #[tokio::test]
    async fn coalesce_separator_is_single_space() {
        let queue = SpeechQueue::new(1);
        assert!(matches!(
            queue.push(request("first", "Hello")),
            PushOutcome::Queued
        ));
        assert!(matches!(
            queue.push(request("second", "world")),
            PushOutcome::Coalesced
        ));
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].translated_text, "Hello world");
    }

    #[tokio::test]
    async fn coalesce_budget_counts_separator() {
        // tail(MAX-2) + separator(1) + 1 == MAX: exactly at the bound, merges.
        let queue = SpeechQueue::new(1);
        queue.push(request("tail", &"a".repeat(MAX_COALESCED_CHARS - 2)));
        assert!(matches!(
            queue.push(request("one", "b")),
            PushOutcome::Coalesced
        ));
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].translated_text.chars().count(),
            MAX_COALESCED_CHARS
        );

        // tail(MAX-2) + separator(1) + 2 == MAX + 1: past the bound, the tail
        // is shed and the fresh request is queued in its place.
        let queue = SpeechQueue::new(1);
        queue.push(request("tail", &"a".repeat(MAX_COALESCED_CHARS - 2)));
        match queue.push(request("two", "bc")) {
            PushOutcome::DroppedOldest { dropped_id } => assert_eq!(dropped_id, "tail"),
            other => panic!("unexpected outcome: {other:?}"),
        }
        let items = drain_closed(&queue).await;
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].utterance_id, "two");
    }
}
