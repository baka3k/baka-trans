use crate::audio::pcm16_to_le_bytes;
use crate::error::{AppError, AppResult};
use crate::models::{
    ManualBoundaryEvent, ManualBoundaryStatus, SessionConfig, SessionStatus, TranscriptItem,
    TranscriptStatus, TranslatedAudioLevelEvent,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use super::openai_realtime::RealtimeControl;

const GOOGLE_LIVE_TRANSLATE_MODEL: &str = "models/gemini-3.5-live-translate-preview";
const GOOGLE_LIVE_TRANSLATE_WS: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
const GOOGLE_AUDIO_MIME_TYPE: &str = "audio/pcm;rate=16000";

pub async fn test_live_translation_connection(api_key: &str) -> AppResult<()> {
    let response = reqwest::Client::new()
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key.trim())])
        .send()
        .await
        .map_err(|err| AppError::new("google_live_key_test_error", err.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::new("google_live_key_test_error", err.to_string()))?;

    if !status.is_success() {
        let message = extract_google_error_message(&body).unwrap_or(body);
        return Err(AppError::new("google_live_key_test_error", message));
    }

    Ok(())
}

pub async fn run_live_translation(
    app: AppHandle,
    config: SessionConfig,
    api_key: String,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    mut control_rx: mpsc::Receiver<RealtimeControl>,
    playback_tx: std_mpsc::SyncSender<Vec<i16>>,
    transcript_store: Arc<Mutex<Vec<TranscriptItem>>>,
) -> AppResult<()> {
    let request_url = live_translate_url(&api_key);
    let (socket, _) = connect_async(&request_url).await.map_err(|err| {
        AppError::new(
            "google_live_connect_error",
            redact_api_key(&err.to_string(), &api_key),
        )
    })?;
    let (mut writer, mut reader) = socket.split();

    writer
        .send(Message::Text(
            google_setup_message(config.target_language.realtime_code())
                .to_string()
                .into(),
        ))
        .await
        .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;

    let _ = app.emit("session-status", SessionStatus::Listening);

    loop {
        tokio::select! {
            maybe_audio = audio_rx.recv() => {
                match maybe_audio {
                    Some(samples) => {
                        writer
                            .send(Message::Text(google_audio_message(&samples).to_string().into()))
                            .await
                            .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;
                    }
                    None => {
                        let _ = writer
                            .send(Message::Text(google_audio_stream_end_message().to_string().into()))
                            .await;
                        let _ = writer.send(Message::Close(None)).await;
                        break;
                    }
                }
            }
            maybe_control = control_rx.recv() => {
                match maybe_control {
                    Some(RealtimeControl::ForceBoundary(_)) => {
                        let _ = app.emit(
                            "manual-boundary-status",
                            ManualBoundaryEvent {
                                status: ManualBoundaryStatus::Error,
                                message: "Manual boundary is not supported by Google Live Translation yet.".to_string(),
                                committed_at_ms: None,
                            },
                        );
                    }
                    Some(RealtimeControl::Stop) => {
                        let _ = writer.send(Message::Close(None)).await;
                        break;
                    }
                    None => {}
                }
            }
            maybe_message = reader.next() => {
                match maybe_message {
                    Some(Ok(Message::Text(text))) => {
                        let event = serde_json::from_str::<Value>(&text)
                            .map_err(|err| AppError::new("google_live_event_parse_error", err.to_string()))?;
                        handle_google_live_event(
                            &app,
                            &event,
                            &playback_tx,
                            &transcript_store,
                        )?;
                    }
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        return Err(AppError::new("google_live_receive_error", err.to_string()));
                    }
                }
            }
        }
    }

    Ok(())
}

fn live_translate_url(api_key: &str) -> String {
    let encoded_key =
        url::form_urlencoded::byte_serialize(api_key.trim().as_bytes()).collect::<String>();
    format!("{GOOGLE_LIVE_TRANSLATE_WS}?key={encoded_key}")
}

fn google_setup_message(target_language: &str) -> Value {
    json!({
        "setup": {
            "model": GOOGLE_LIVE_TRANSLATE_MODEL,
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "inputAudioTranscription": {},
                "outputAudioTranscription": {},
                "translationConfig": {
                    "targetLanguageCode": target_language,
                    "echoTargetLanguage": false
                }
            }
        }
    })
}

fn google_audio_message(samples: &[i16]) -> Value {
    json!({
        "realtimeInput": {
            "audio": {
                "data": STANDARD.encode(pcm16_to_le_bytes(samples)),
                "mimeType": GOOGLE_AUDIO_MIME_TYPE
            }
        }
    })
}

fn google_audio_stream_end_message() -> Value {
    json!({
        "realtimeInput": {
            "audioStreamEnd": true
        }
    })
}

fn handle_google_live_event(
    app: &AppHandle,
    event: &Value,
    playback_tx: &std_mpsc::SyncSender<Vec<i16>>,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
) -> AppResult<()> {
    if let Some(message) = google_error_message(event) {
        return Err(AppError::new("google_live_api_error", message));
    }

    if let Some(text) = transcript_text(event, true) {
        emit_transcript_delta(app, &text, true, transcript_store);
    }
    if let Some(text) = transcript_text(event, false) {
        emit_transcript_delta(app, &text, false, transcript_store);
    }

    for samples in audio_parts(event)? {
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
        let _ = app.emit("session-status", SessionStatus::Speaking);
    }

    Ok(())
}

fn transcript_text(event: &Value, source: bool) -> Option<String> {
    let key = if source {
        "inputTranscription"
    } else {
        "outputTranscription"
    };
    event
        .pointer(&format!("/serverContent/{key}/text"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToString::to_string)
}

fn audio_parts(event: &Value) -> AppResult<Vec<Vec<i16>>> {
    let Some(parts) = event
        .pointer("/serverContent/modelTurn/parts")
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };

    let mut decoded = Vec::new();
    for part in parts {
        let Some(data) = part.pointer("/inlineData/data").and_then(Value::as_str) else {
            continue;
        };
        let bytes = STANDARD
            .decode(data)
            .map_err(|err| AppError::new("google_live_audio_format_error", err.to_string()))?;
        if bytes.len() % 2 != 0 {
            return Err(AppError::new(
                "google_live_audio_format_error",
                "Google Live returned an odd number of PCM16 audio bytes.",
            ));
        }
        decoded.push(
            bytes
                .chunks_exact(2)
                .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                .collect(),
        );
    }

    Ok(decoded)
}

fn emit_transcript_delta(
    app: &AppHandle,
    text: &str,
    source: bool,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
) {
    let item = TranscriptItem {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: now_ms(),
        source_text: if source {
            text.to_string()
        } else {
            String::new()
        },
        translated_text: if source {
            String::new()
        } else {
            text.to_string()
        },
        status: TranscriptStatus::Partial,
        latency_ms: None,
    };
    if let Ok(mut transcript) = transcript_store.lock() {
        merge_transcript_delta(&mut transcript, item.clone());
    }
    let _ = app.emit("transcript-update", item);
    let _ = app.emit("session-status", SessionStatus::Translating);
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

fn google_error_message(event: &Value) -> Option<String> {
    event
        .pointer("/error/message")
        .or_else(|| event.get("message"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn extract_google_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| google_error_message(&value))
}

fn redact_api_key(message: &str, api_key: &str) -> String {
    message.replace(api_key.trim(), "[redacted]")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_google_error_message() {
        let body = r#"{"error":{"code":400,"message":"API key not valid."}}"#;

        assert_eq!(
            extract_google_error_message(body),
            Some("API key not valid.".to_string())
        );
    }

    #[test]
    fn builds_live_translate_setup_payload() {
        let payload = google_setup_message("vi");

        assert_eq!(
            payload.pointer("/setup/model").and_then(Value::as_str),
            Some(GOOGLE_LIVE_TRANSLATE_MODEL)
        );
        assert_eq!(
            payload
                .pointer("/setup/generationConfig/translationConfig/targetLanguageCode")
                .and_then(Value::as_str),
            Some("vi")
        );
        assert_eq!(
            payload
                .pointer("/setup/generationConfig/responseModalities/0")
                .and_then(Value::as_str),
            Some("AUDIO")
        );
    }

    #[test]
    fn builds_live_translate_audio_payload() {
        let payload = google_audio_message(&[1, -2]);

        assert_eq!(
            payload
                .pointer("/realtimeInput/audio/mimeType")
                .and_then(Value::as_str),
            Some(GOOGLE_AUDIO_MIME_TYPE)
        );
        assert_eq!(
            payload
                .pointer("/realtimeInput/audio/data")
                .and_then(Value::as_str),
            Some("AQD+/w==")
        );
        assert_eq!(crate::audio::GOOGLE_LIVE_INPUT_SAMPLE_RATE, 16_000);
    }

    #[test]
    fn parses_google_transcripts_and_audio_parts() {
        let audio = STANDARD.encode([1u8, 0, 254, 255]);
        let event = json!({
            "serverContent": {
                "inputTranscription": { "text": "hello" },
                "outputTranscription": { "text": "xin chao" },
                "modelTurn": {
                    "parts": [
                        { "inlineData": { "data": audio, "mimeType": "audio/pcm;rate=24000" } }
                    ]
                }
            }
        });

        assert_eq!(transcript_text(&event, true), Some("hello".to_string()));
        assert_eq!(transcript_text(&event, false), Some("xin chao".to_string()));
        assert_eq!(audio_parts(&event).expect("valid audio"), vec![vec![1, -2]]);
    }

    #[test]
    fn rejects_odd_audio_byte_count() {
        let event = json!({
            "serverContent": {
                "modelTurn": {
                    "parts": [
                        { "inlineData": { "data": STANDARD.encode([1u8]), "mimeType": "audio/pcm;rate=24000" } }
                    ]
                }
            }
        });

        let error = audio_parts(&event).expect_err("odd PCM16 bytes should fail");
        assert_eq!(error.code, "google_live_audio_format_error");
    }
}
