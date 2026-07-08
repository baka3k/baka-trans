use crate::audio::pcm16_to_le_bytes;
use crate::error::{AppError, AppResult};
use crate::models::{
    ManualBoundaryEvent, ManualBoundaryRequest, ManualBoundaryStatus, SessionConfig, SessionStatus,
    TranscriptItem, TranscriptStatus,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{Sink, SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderValue, AUTHORIZATION};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const REALTIME_TRANSLATION_URL: &str =
    "wss://api.openai.com/v1/realtime/translations?model=gpt-realtime-translate";

pub enum RealtimeControl {
    ForceBoundary(ManualBoundaryRequest),
}

pub async fn run_realtime_translation(
    app: AppHandle,
    config: SessionConfig,
    api_key: String,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    mut control_rx: mpsc::Receiver<RealtimeControl>,
    playback_tx: std_mpsc::SyncSender<Vec<i16>>,
    transcript_store: Arc<Mutex<Vec<TranscriptItem>>>,
) -> AppResult<()> {
    let mut boundary_state = ManualBoundaryRuntimeState::default();

    'session: loop {
        let mut request = REALTIME_TRANSLATION_URL
            .into_client_request()
            .map_err(|err| AppError::new("realtime_request_error", err.to_string()))?;
        let auth = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|err| AppError::new("realtime_auth_error", err.to_string()))?;
        request.headers_mut().insert(AUTHORIZATION, auth);
        request.headers_mut().insert(
            "OpenAI-Safety-Identifier",
            HeaderValue::from_static("baka-trans-local-user"),
        );

        let (socket, _) = connect_async(request)
            .await
            .map_err(|err| AppError::new("realtime_connect_error", err.to_string()))?;
        let (mut writer, mut reader) = socket.split();

        let update = json!({
            "type": "session.update",
            "session": {
                "audio": {
                    "output": {
                        "language": config.target_language.realtime_code(),
                        "voice": config.voice_id,
                    }
                },
                "instructions": config.translation_style.instructions()
            }
        });
        writer
            .send(Message::Text(update.to_string().into()))
            .await
            .map_err(|err| AppError::new("realtime_send_error", err.to_string()))?;
        flush_reconnect_audio(&mut writer, &mut boundary_state).await?;

        let _ = app.emit("session-status", SessionStatus::Listening);

        let mut input_closed = false;

        loop {
            tokio::select! {
                maybe_audio = audio_rx.recv(), if !input_closed => {
                    match maybe_audio {
                        Some(samples) => {
                            if boundary_state.reconnect_after_close {
                                boundary_state.pending_reconnect_audio.push_back(samples);
                            } else {
                                append_audio(&mut writer, &mut boundary_state, samples).await?;
                            }
                        }
                        None => {
                            input_closed = true;
                            if !boundary_state.reconnect_after_close {
                                let _ = writer.send(Message::Text(json!({"type": "session.close"}).to_string().into())).await;
                            }
                        }
                    }
                }
                maybe_control = control_rx.recv() => {
                    match maybe_control {
                        Some(RealtimeControl::ForceBoundary(request)) => {
                            flush_pending_audio(&mut audio_rx, &mut writer, &mut boundary_state).await?;
                            handle_manual_boundary_request(
                                &app,
                                &mut writer,
                                &mut boundary_state,
                                request,
                            )
                            .await?;
                        }
                        None => {}
                    }
                }
                maybe_message = reader.next() => {
                    match maybe_message {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                                match handle_realtime_event(
                                    &app,
                                    value,
                                    &playback_tx,
                                    &transcript_store,
                                    &mut boundary_state,
                                )? {
                                    RealtimeEventOutcome::Continue => {}
                                    RealtimeEventOutcome::Closed => break 'session,
                                    RealtimeEventOutcome::Reconnect => continue 'session,
                                }
                            }
                        }
                        Some(Ok(Message::Binary(_))) => {}
                        Some(Ok(Message::Close(_))) | None => {
                            if boundary_state.reconnect_after_close {
                                boundary_state.pending_event_id = None;
                                boundary_state.pending_requested_at_ms = None;
                                boundary_state.reconnect_after_close = false;
                                continue 'session;
                            }
                            break 'session;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            return Err(AppError::new("realtime_receive_error", err.to_string()));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn append_audio<W>(
    writer: &mut W,
    boundary_state: &mut ManualBoundaryRuntimeState,
    samples: Vec<i16>,
) -> AppResult<()>
where
    W: Sink<Message> + Unpin,
    <W as Sink<Message>>::Error: std::fmt::Display,
{
    if !samples.is_empty() {
        boundary_state.has_buffered_audio = true;
    }
    let audio = STANDARD.encode(pcm16_to_le_bytes(&samples));
    let append = json!({
        "type": "session.input_audio_buffer.append",
        "audio": audio
    });
    writer
        .send(Message::Text(append.to_string().into()))
        .await
        .map_err(|err| AppError::new("realtime_send_error", err.to_string()))
}

async fn flush_pending_audio<W>(
    audio_rx: &mut mpsc::Receiver<Vec<i16>>,
    writer: &mut W,
    boundary_state: &mut ManualBoundaryRuntimeState,
) -> AppResult<()>
where
    W: Sink<Message> + Unpin,
    <W as Sink<Message>>::Error: std::fmt::Display,
{
    if boundary_state.reconnect_after_close {
        return Ok(());
    }

    while let Ok(samples) = audio_rx.try_recv() {
        append_audio(writer, boundary_state, samples).await?;
    }

    Ok(())
}

async fn flush_reconnect_audio<W>(
    writer: &mut W,
    boundary_state: &mut ManualBoundaryRuntimeState,
) -> AppResult<()>
where
    W: Sink<Message> + Unpin,
    <W as Sink<Message>>::Error: std::fmt::Display,
{
    while let Some(samples) = boundary_state.pending_reconnect_audio.pop_front() {
        append_audio(writer, boundary_state, samples).await?;
    }

    Ok(())
}

async fn handle_manual_boundary_request<W>(
    app: &AppHandle,
    writer: &mut W,
    boundary_state: &mut ManualBoundaryRuntimeState,
    request: ManualBoundaryRequest,
) -> AppResult<()>
where
    W: Sink<Message> + Unpin,
    <W as Sink<Message>>::Error: std::fmt::Display,
{
    boundary_state.metrics.requests += 1;
    if boundary_state.reconnect_after_close {
        emit_manual_boundary(
            app,
            ManualBoundaryStatus::RateLimited,
            "Still translating",
            None,
            &boundary_state.metrics,
        )?;
        return Ok(());
    }

    if !boundary_state.has_buffered_audio {
        boundary_state.metrics.ignored_empty += 1;
        emit_manual_boundary(
            app,
            ManualBoundaryStatus::IgnoredEmptyBuffer,
            "No buffered speech",
            None,
            &boundary_state.metrics,
        )?;
        return Ok(());
    }

    let event_id = format!("manual_boundary_{}", Uuid::new_v4());
    let close = json!({
        "event_id": event_id,
        "type": "session.close"
    });
    writer
        .send(Message::Text(close.to_string().into()))
        .await
        .map_err(|err| AppError::new("realtime_send_error", err.to_string()))?;

    boundary_state.has_buffered_audio = false;
    boundary_state.pending_event_id = Some(event_id);
    boundary_state.pending_requested_at_ms = Some(request.requested_at_ms);
    boundary_state.reconnect_after_close = true;
    boundary_state.metrics.commits += 1;
    emit_manual_boundary(
        app,
        ManualBoundaryStatus::Committed,
        "Boundary sent",
        Some(now_ms()),
        &boundary_state.metrics,
    )
}

fn handle_realtime_event(
    app: &AppHandle,
    event: Value,
    playback_tx: &std_mpsc::SyncSender<Vec<i16>>,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
    boundary_state: &mut ManualBoundaryRuntimeState,
) -> AppResult<RealtimeEventOutcome> {
    let Some(event_type) = event.get("type").and_then(Value::as_str) else {
        return Ok(RealtimeEventOutcome::Continue);
    };

    match event_type {
        "session.input_transcript.delta" => {
            emit_transcript_delta(app, event, true, transcript_store)
        }
        "session.output_transcript.delta" => {
            emit_transcript_delta(app, event, false, transcript_store)
        }
        "session.output_audio.delta" => {
            if let Some(samples) = decode_output_audio(&event) {
                let _ = playback_tx.try_send(samples);
            }
            let _ = app.emit("session-status", SessionStatus::Speaking);
        }
        "session.closed" => {
            if boundary_state.reconnect_after_close {
                boundary_state.pending_event_id = None;
                boundary_state.pending_requested_at_ms = None;
                boundary_state.reconnect_after_close = false;
                return Ok(RealtimeEventOutcome::Reconnect);
            }

            return Ok(RealtimeEventOutcome::Closed);
        }
        "error" => {
            let message = event
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("Realtime API error.");
            if boundary_state.is_pending_error(&event) {
                boundary_state.pending_event_id = None;
                boundary_state.pending_requested_at_ms = None;
                boundary_state.reconnect_after_close = false;
                if is_empty_buffer_error(message) {
                    boundary_state.metrics.ignored_empty += 1;
                    emit_manual_boundary(
                        app,
                        ManualBoundaryStatus::IgnoredEmptyBuffer,
                        "No buffered speech",
                        None,
                        &boundary_state.metrics,
                    )?;
                    return Ok(RealtimeEventOutcome::Continue);
                }

                boundary_state.metrics.errors += 1;
                emit_manual_boundary(
                    app,
                    ManualBoundaryStatus::Error,
                    message,
                    None,
                    &boundary_state.metrics,
                )?;
                return Ok(RealtimeEventOutcome::Continue);
            }
            return Err(AppError::new("realtime_api_error", message));
        }
        _ => {}
    }

    Ok(RealtimeEventOutcome::Continue)
}

#[derive(Default)]
struct ManualBoundaryRuntimeState {
    has_buffered_audio: bool,
    pending_event_id: Option<String>,
    pending_requested_at_ms: Option<u64>,
    reconnect_after_close: bool,
    pending_reconnect_audio: VecDeque<Vec<i16>>,
    metrics: ManualBoundaryMetrics,
}

impl ManualBoundaryRuntimeState {
    fn is_pending_error(&self, event: &Value) -> bool {
        let Some(pending_event_id) = self.pending_event_id.as_deref() else {
            return false;
        };

        if event
            .pointer("/error/event_id")
            .and_then(Value::as_str)
            .is_some_and(|event_id| event_id == pending_event_id)
            || event
                .get("event_id")
                .and_then(Value::as_str)
                .is_some_and(|event_id| event_id == pending_event_id)
        {
            return true;
        }

        event
            .pointer("/error/message")
            .and_then(Value::as_str)
            .is_some_and(is_manual_boundary_error)
    }
}

#[derive(Default)]
struct ManualBoundaryMetrics {
    requests: u64,
    commits: u64,
    ignored_empty: u64,
    errors: u64,
}

fn emit_manual_boundary(
    app: &AppHandle,
    status: ManualBoundaryStatus,
    message: impl Into<String>,
    committed_at_ms: Option<u64>,
    metrics: &ManualBoundaryMetrics,
) -> AppResult<()> {
    tracing::info!(
        manual_boundary_requests = metrics.requests,
        manual_boundary_commits = metrics.commits,
        manual_boundary_ignored_empty = metrics.ignored_empty,
        manual_boundary_errors = metrics.errors,
        "manual boundary status updated"
    );

    app.emit(
        "manual-boundary-status",
        ManualBoundaryEvent {
            status,
            message: message.into(),
            committed_at_ms,
        },
    )
    .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

fn is_empty_buffer_error(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("empty") && normalized.contains("buffer")
}

fn is_manual_boundary_error(message: &str) -> bool {
    let normalized = message.to_lowercase();
    normalized.contains("session.close")
        || normalized.contains("input_audio_buffer")
        || (normalized.contains("close") && normalized.contains("session"))
        || (normalized.contains("commit") && normalized.contains("buffer"))
}

enum RealtimeEventOutcome {
    Continue,
    Closed,
    Reconnect,
}

fn emit_transcript_delta(
    app: &AppHandle,
    event: Value,
    source: bool,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
) {
    let delta = event
        .get("delta")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    if delta.is_empty() {
        return;
    }

    let item = TranscriptItem {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: now_ms(),
        source_text: if source { delta.clone() } else { String::new() },
        translated_text: if source { String::new() } else { delta },
        status: TranscriptStatus::Partial,
        latency_ms: None,
    };
    if let Ok(mut transcript) = transcript_store.lock() {
        merge_transcript_delta(&mut transcript, item.clone());
    }
    let _ = app.emit("transcript-update", item);
    let _ = app.emit("session-status", SessionStatus::Translating);
}

fn decode_output_audio(event: &Value) -> Option<Vec<i16>> {
    let delta = event.get("delta").and_then(Value::as_str)?;
    let bytes = STANDARD.decode(delta).ok()?;
    Some(
        bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect(),
    )
}

fn merge_transcript_delta(transcript: &mut Vec<TranscriptItem>, item: TranscriptItem) {
    let Some(last) = transcript.last_mut() else {
        transcript.push(item);
        return;
    };

    if last.status != TranscriptStatus::Partial || item.status != TranscriptStatus::Partial {
        transcript.push(item);
        return;
    }

    if !item.source_text.is_empty() && item.translated_text.is_empty() {
        last.source_text.push_str(&item.source_text);
        return;
    }

    if !item.translated_text.is_empty() && item.source_text.is_empty() {
        last.translated_text.push_str(&item.translated_text);
        return;
    }

    transcript.push(item);
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{is_empty_buffer_error, is_manual_boundary_error};

    #[test]
    fn identifies_empty_input_audio_buffer_errors() {
        assert!(is_empty_buffer_error("Input audio buffer is empty."));
        assert!(is_empty_buffer_error("Cannot commit empty buffer"));
        assert!(!is_empty_buffer_error("Realtime API rate limit exceeded."));
    }

    #[test]
    fn identifies_manual_boundary_transport_errors() {
        assert!(is_manual_boundary_error("Invalid event: session.close"));
        assert!(is_manual_boundary_error("Cannot close translation session"));
        assert!(!is_manual_boundary_error(
            "Realtime API rate limit exceeded."
        ));
    }
}
