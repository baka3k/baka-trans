use crate::audio::pcm16_to_le_bytes;
use crate::error::{AppError, AppResult};
use crate::models::{
    ManualBoundaryEvent, ManualBoundaryRequest, ManualBoundaryStatus, SessionConfig, SessionStatus,
    TranscriptItem, TranscriptStatus, TranslatedAudioLevelEvent,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{Sink, SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{HeaderValue, AUTHORIZATION};
use tokio_tungstenite::tungstenite::http::Request;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

const REALTIME_TRANSLATION_URL: &str =
    "wss://api.openai.com/v1/realtime/translations?model=gpt-realtime-translate";

pub enum RealtimeControl {
    ForceBoundary(ManualBoundaryRequest),
    Stop,
}

pub async fn test_realtime_connection(api_key: &str) -> AppResult<()> {
    let token = mint_realtime_client_secret(api_key, "auto", "vi").await?;
    let request = realtime_translation_request(&token)?;
    let (mut socket, _) = connect_async(request)
        .await
        .map_err(|err| AppError::new("realtime_connect_error", err.to_string()))?;
    let _ = socket.close(None).await;
    Ok(())
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
        let realtime_token = mint_realtime_client_secret(
            &api_key,
            config.source_language.realtime_code(),
            config.target_language.realtime_code(),
        )
        .await?;
        let request = realtime_translation_request(&realtime_token)?;
        let (socket, _) = connect_async(request)
            .await
            .map_err(|err| AppError::new("realtime_connect_error", err.to_string()))?;
        let (mut writer, mut reader) = socket.split();

        let update = realtime_session_update(
            config.source_language.realtime_code(),
            config.target_language.realtime_code(),
        );
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
                        Some(RealtimeControl::Stop) => {
                            let _ = writer.send(Message::Close(None)).await;
                            break 'session;
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

#[derive(Deserialize)]
struct RealtimeClientSecretResponse {
    value: String,
}

async fn mint_realtime_client_secret(
    api_key: &str,
    source_language: &str,
    target_language: &str,
) -> AppResult<String> {
    let api_key = api_key.trim().to_string();
    let source_language = source_language.to_string();
    let target_language = target_language.to_string();
    tokio::task::spawn_blocking(move || {
        mint_realtime_client_secret_blocking(&api_key, &source_language, &target_language)
    })
    .await
    .map_err(|err| AppError::new("realtime_token_task_error", err.to_string()))?
}

fn mint_realtime_client_secret_blocking(
    api_key: &str,
    source_language: &str,
    target_language: &str,
) -> AppResult<String> {
    let body = json!({
            "session": {
                "model": "gpt-realtime-translate",
                "audio": realtime_audio_config(source_language, target_language),
            }
    })
    .to_string();
    let response = post_json_over_tls(
        "api.openai.com",
        "/v1/realtime/translations/client_secrets",
        api_key,
        &body,
    )?;

    if !(200..300).contains(&response.status) {
        let body = response.body;
        let message = extract_openai_error_message(&body).unwrap_or(body);
        return Err(AppError::new("realtime_token_error", message));
    }

    let token = serde_json::from_str::<RealtimeClientSecretResponse>(&response.body)
        .map_err(|err| AppError::new("realtime_token_parse_error", err.to_string()))?
        .value;

    if token.trim().is_empty() {
        return Err(AppError::new(
            "realtime_token_error",
            "OpenAI returned an empty Realtime client secret.",
        ));
    }

    Ok(token)
}

fn realtime_session_update(source_language: &str, target_language: &str) -> Value {
    json!({
        "type": "session.update",
        "session": {
            "audio": realtime_audio_config(source_language, target_language),
        }
    })
}

fn realtime_audio_config(source_language: &str, target_language: &str) -> Value {
    let mut transcription = json!({
        "model": "gpt-realtime-whisper",
    });
    if source_language != "auto" {
        transcription["language"] = json!(source_language);
    }

    json!({
        "input": {
            "transcription": transcription,
        },
        "output": {
            "language": target_language,
        }
    })
}

struct HttpResponse {
    status: u16,
    body: String,
}

fn post_json_over_tls(
    host: &str,
    path: &str,
    bearer_token: &str,
    body: &str,
) -> AppResult<HttpResponse> {
    let stream = TcpStream::connect((host, 443))
        .map_err(|err| AppError::new("realtime_token_connect_error", err.to_string()))?;
    let connector = native_tls::TlsConnector::new()
        .map_err(|err| AppError::new("realtime_token_tls_error", err.to_string()))?;
    let mut stream = connector
        .connect(host, stream)
        .map_err(|err| AppError::new("realtime_token_tls_error", err.to_string()))?;

    let request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host}\r\n\
         Authorization: Bearer {bearer_token}\r\n\
         Content-Type: application/json\r\n\
         Accept: application/json\r\n\
         OpenAI-Safety-Identifier: baka-trans-local-user\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n\
         {body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|err| AppError::new("realtime_token_request_error", err.to_string()))?;

    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|err| AppError::new("realtime_token_response_error", err.to_string()))?;
    parse_http_response(&raw)
}

fn parse_http_response(raw: &[u8]) -> AppResult<HttpResponse> {
    let response = String::from_utf8_lossy(raw);
    let (headers, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        AppError::new(
            "realtime_token_response_error",
            "OpenAI returned a malformed HTTP response.",
        )
    })?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| {
            AppError::new(
                "realtime_token_response_error",
                "OpenAI returned a response without an HTTP status code.",
            )
        })?;
    let body = if headers
        .to_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked_body(body)?
    } else {
        body.to_string()
    };

    Ok(HttpResponse { status, body })
}

fn decode_chunked_body(body: &str) -> AppResult<String> {
    let mut remaining = body;
    let mut decoded = String::new();
    loop {
        let Some((size_line, rest)) = remaining.split_once("\r\n") else {
            return Err(AppError::new(
                "realtime_token_response_error",
                "OpenAI returned a malformed chunked response.",
            ));
        };
        let size_hex = size_line.split(';').next().unwrap_or(size_line).trim();
        let size = usize::from_str_radix(size_hex, 16)
            .map_err(|err| AppError::new("realtime_token_response_error", err.to_string()))?;
        if size == 0 {
            break;
        }
        if rest.len() < size + 2 {
            return Err(AppError::new(
                "realtime_token_response_error",
                "OpenAI returned a truncated chunked response.",
            ));
        }
        decoded.push_str(&rest[..size]);
        remaining = &rest[size + 2..];
    }

    Ok(decoded)
}

fn extract_openai_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok().and_then(|value| {
        value
            .pointer("/error/message")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}

fn realtime_translation_request(bearer_token: &str) -> AppResult<Request<()>> {
    let mut request = REALTIME_TRANSLATION_URL
        .into_client_request()
        .map_err(|err| AppError::new("realtime_request_error", err.to_string()))?;
    let auth = HeaderValue::from_str(&format!("Bearer {}", bearer_token.trim()))
        .map_err(|err| AppError::new("realtime_auth_error", err.to_string()))?;
    request.headers_mut().insert(AUTHORIZATION, auth);
    request.headers_mut().insert(
        "OpenAI-Safety-Identifier",
        HeaderValue::from_static("baka-trans-local-user"),
    );

    Ok(request)
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
                let (rms, peak) = pcm16_level(&samples);
                let sample_count = samples.len();
                if let Err(error) = playback_tx.try_send(samples) {
                    if matches!(error, std_mpsc::TrySendError::Disconnected(_)) {
                        let _ = app.emit(
                            "app-error",
                            AppError::new(
                                "audio_playback_error",
                                "Translated audio arrived, but the playback stream is not available.",
                            ),
                        );
                    }
                }
                let _ = app.emit(
                    "translated-audio-level",
                    TranslatedAudioLevelEvent {
                        sample_count,
                        rms,
                        peak,
                    },
                );
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

fn pcm16_level(samples: &[i16]) -> (f32, f32) {
    let mut peak = 0.0f32;
    let mut sum = 0.0f32;
    for sample in samples {
        let value = *sample as f32 / i16::MAX as f32;
        let abs = value.abs();
        peak = peak.max(abs);
        sum += value * value;
    }
    let rms = if samples.is_empty() {
        0.0
    } else {
        (sum / samples.len() as f32).sqrt()
    };
    (rms, peak)
}

fn merge_transcript_delta(transcript: &mut Vec<TranscriptItem>, item: TranscriptItem) {
    let Some(last) = transcript.last_mut() else {
        transcript.push(item);
        return;
    };

    if last.status == TranscriptStatus::Final || !is_single_sided_delta(&item) {
        transcript.push(item);
        return;
    }

    if !item.source_text.is_empty() && item.translated_text.is_empty() {
        last.source_text.push_str(&item.source_text);
        last.status = item.status;
        return;
    }

    if !item.translated_text.is_empty() && item.source_text.is_empty() {
        append_transcript_text(&mut last.translated_text, &item.translated_text, true);
        last.status = item.status;
        return;
    }

    transcript.push(item);
}

fn is_single_sided_delta(item: &TranscriptItem) -> bool {
    (!item.source_text.is_empty() && item.translated_text.is_empty())
        || (!item.translated_text.is_empty() && item.source_text.is_empty())
}

fn append_transcript_text(current: &mut String, delta: &str, break_after_sentence: bool) {
    if delta.is_empty() {
        return;
    }
    if break_after_sentence && should_start_new_transcript_line(current, delta) {
        current.push('\n');
        current.push_str(delta.trim_start());
        return;
    }
    current.push_str(delta);
}

fn should_start_new_transcript_line(current: &str, delta: &str) -> bool {
    let next = delta.trim_start();
    current
        .trim_end()
        .chars()
        .next_back()
        .is_some_and(|ch| matches!(ch, '.' | '!' | '?' | '。' | '！' | '？'))
        && !next.is_empty()
        && !next
            .chars()
            .next()
            .is_some_and(|ch| matches!(ch, ',' | '.' | ';' | ':' | '!' | '?' | ')'))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        is_empty_buffer_error, is_manual_boundary_error, merge_transcript_delta,
        realtime_session_update,
    };
    use crate::models::{TranscriptItem, TranscriptStatus};

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

    #[test]
    fn realtime_session_update_enables_source_transcription() {
        let update = realtime_session_update("vi", "en");

        assert_eq!(update["type"], "session.update");
        assert_eq!(
            update["session"]["audio"]["input"]["transcription"]["model"],
            "gpt-realtime-whisper"
        );
        assert_eq!(
            update["session"]["audio"]["input"]["transcription"]["language"],
            "vi"
        );
        assert_eq!(update["session"]["audio"]["output"]["language"], "en");
    }

    #[test]
    fn realtime_session_update_omits_auto_source_language_hint() {
        let update = realtime_session_update("auto", "en");

        assert!(update["session"]["audio"]["input"]["transcription"]["language"].is_null());
    }

    #[test]
    fn merges_final_translation_only_delta_into_current_item() {
        let mut transcript = vec![TranscriptItem {
            id: "1".to_string(),
            timestamp_ms: 1,
            source_text: "Hello".to_string(),
            translated_text: String::new(),
            status: TranscriptStatus::Partial,
            latency_ms: None,
        }];

        merge_transcript_delta(
            &mut transcript,
            TranscriptItem {
                id: "2".to_string(),
                timestamp_ms: 2,
                source_text: String::new(),
                translated_text: "Xin chao".to_string(),
                status: TranscriptStatus::Final,
                latency_ms: None,
            },
        );

        assert_eq!(transcript.len(), 1);
        assert_eq!(transcript[0].source_text, "Hello");
        assert_eq!(transcript[0].translated_text, "Xin chao");
        assert_eq!(transcript[0].status, TranscriptStatus::Final);
    }

    #[test]
    fn adds_translated_line_break_after_sentence_boundary() {
        let mut transcript = vec![TranscriptItem {
            id: "1".to_string(),
            timestamp_ms: 1,
            source_text: "Hello".to_string(),
            translated_text: "Good morning.".to_string(),
            status: TranscriptStatus::Partial,
            latency_ms: None,
        }];

        merge_transcript_delta(
            &mut transcript,
            TranscriptItem {
                id: "2".to_string(),
                timestamp_ms: 2,
                source_text: String::new(),
                translated_text: " We can start now.".to_string(),
                status: TranscriptStatus::Partial,
                latency_ms: None,
            },
        );

        assert_eq!(transcript.len(), 1);
        assert_eq!(
            transcript[0].translated_text,
            "Good morning.\nWe can start now."
        );
    }
}
