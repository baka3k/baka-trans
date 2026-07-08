use crate::audio::pcm16_to_le_bytes;
use crate::error::{AppError, AppResult};
use crate::models::{SessionConfig, SessionStatus, TranscriptItem, TranscriptStatus};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
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

pub async fn run_realtime_translation(
    app: AppHandle,
    config: SessionConfig,
    api_key: String,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    playback_tx: std_mpsc::Sender<Vec<i16>>,
    transcript_store: Arc<Mutex<Vec<TranscriptItem>>>,
) -> AppResult<()> {
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

    let _ = app.emit("session-status", SessionStatus::Listening);

    let mut input_closed = false;

    loop {
        tokio::select! {
            maybe_audio = audio_rx.recv(), if !input_closed => {
                match maybe_audio {
                    Some(samples) => {
                        let audio = STANDARD.encode(pcm16_to_le_bytes(&samples));
                        let append = json!({
                            "type": "session.input_audio_buffer.append",
                            "audio": audio
                        });
                        writer
                            .send(Message::Text(append.to_string().into()))
                            .await
                            .map_err(|err| AppError::new("realtime_send_error", err.to_string()))?;
                    }
                    None => {
                        input_closed = true;
                        let _ = writer.send(Message::Text(json!({"type": "session.close"}).to_string().into())).await;
                    }
                }
            }
            maybe_message = reader.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            if handle_realtime_event(&app, value, &playback_tx, &transcript_store)? {
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        return Err(AppError::new("realtime_receive_error", err.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}

fn handle_realtime_event(
    app: &AppHandle,
    event: Value,
    playback_tx: &std_mpsc::Sender<Vec<i16>>,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
) -> AppResult<bool> {
    let Some(event_type) = event.get("type").and_then(Value::as_str) else {
        return Ok(false);
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
                let _ = playback_tx.send(samples);
            }
            let _ = app.emit("session-status", SessionStatus::Speaking);
        }
        "session.closed" => {
            return Ok(true);
        }
        "error" => {
            let message = event
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("Realtime API error.");
            return Err(AppError::new("realtime_api_error", message));
        }
        _ => {}
    }

    Ok(false)
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
