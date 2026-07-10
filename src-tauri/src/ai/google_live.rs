use crate::audio::{pcm16_to_le_bytes, GOOGLE_LIVE_INPUT_SAMPLE_RATE};
use crate::error::{AppError, AppResult};
use crate::models::{
    ManualBoundaryEvent, ManualBoundaryStatus, SessionConfig, SessionStatus, TranscriptItem,
    TranscriptStatus, TranslatedAudioLevelEvent,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use serde_json::{json, Value};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

use super::openai_realtime::RealtimeControl;

const GOOGLE_LIVE_TRANSLATE_MODEL: &str = "models/gemini-3.5-live-translate-preview";
const GOOGLE_LIVE_TRANSLATE_WS: &str = "wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";
const GOOGLE_AUDIO_MIME_TYPE: &str = "audio/pcm;rate=16000";
const GOOGLE_AUDIO_CHUNK_MS: usize = 100;
const GOOGLE_SETUP_TIMEOUT_SECS: u64 = 2;

pub async fn test_live_translation_connection(api_key: &str) -> AppResult<()> {
    test_google_models_key(api_key).await?;
    test_live_translation_setup(api_key, "vi").await
}

async fn test_google_models_key(api_key: &str) -> AppResult<()> {
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

async fn test_live_translation_setup(api_key: &str, target_language: &str) -> AppResult<()> {
    let request_url = live_translate_url(api_key);
    let (socket, _) = connect_async(&request_url).await.map_err(|err| {
        AppError::new(
            "google_live_connect_error",
            redact_api_key(&err.to_string(), api_key),
        )
    })?;
    let (mut writer, mut reader) = socket.split();

    writer
        .send(Message::Text(
            google_setup_message(target_language).to_string().into(),
        ))
        .await
        .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;
    wait_for_google_setup_complete(&mut reader).await?;
    let _ = writer.send(Message::Close(None)).await;

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
    let mut pending_audio = Vec::<i16>::new();

    writer
        .send(Message::Text(
            google_setup_message(config.target_language.realtime_code())
                .to_string()
                .into(),
        ))
        .await
        .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;

    wait_for_google_setup_complete(&mut reader).await?;
    let _ = app.emit("session-status", SessionStatus::Listening);

    loop {
        tokio::select! {
            maybe_audio = audio_rx.recv() => {
                match maybe_audio {
                    Some(samples) => {
                        for chunk in drain_complete_google_audio_chunks(&mut pending_audio, &samples) {
                            send_google_audio(&mut writer, &chunk).await?;
                        }
                    }
                    None => {
                        if let Some(chunk) = drain_remaining_google_audio(&mut pending_audio) {
                            send_google_audio(&mut writer, &chunk).await?;
                        }
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
                        let event = parse_google_event_text(text.as_ref())?;
                        handle_google_live_event(
                            &app,
                            &event,
                            &playback_tx,
                            &transcript_store,
                        )?;
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let event = parse_google_event_text(bytes.as_ref())?;
                        handle_google_live_event(
                            &app,
                            &event,
                            &playback_tx,
                            &transcript_store,
                        )?;
                    }
                    Some(Ok(Message::Close(frame))) => {
                        return Err(unexpected_google_close_error(frame.map(|frame| frame.to_string())));
                    }
                    None => {
                        return Err(unexpected_google_close_error(None));
                    }
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

async fn send_google_audio<W>(writer: &mut W, samples: &[i16]) -> AppResult<()>
where
    W: Sink<Message> + Unpin,
    <W as Sink<Message>>::Error: std::fmt::Display,
{
    if samples.is_empty() {
        return Ok(());
    }

    writer
        .send(Message::Text(
            google_audio_message(samples).to_string().into(),
        ))
        .await
        .map_err(|err| AppError::new("google_live_send_error", err.to_string()))
}

async fn wait_for_google_setup_complete<R>(reader: &mut R) -> AppResult<()>
where
    R: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    wait_for_google_setup_complete_with_timeout(
        reader,
        Duration::from_secs(GOOGLE_SETUP_TIMEOUT_SECS),
    )
    .await
}

async fn wait_for_google_setup_complete_with_timeout<R>(
    reader: &mut R,
    wait_duration: Duration,
) -> AppResult<()>
where
    R: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let result = timeout(wait_duration, async {
        loop {
            match reader.next().await {
                Some(Ok(Message::Text(text))) => {
                    let event = parse_google_event_text(text.as_ref())?;
                    if let Some(message) = google_error_message(&event) {
                        return Err(AppError::new("google_live_api_error", message));
                    }
                    if event.get("setupComplete").is_some() {
                        return Ok(());
                    }
                }
                Some(Ok(Message::Binary(bytes))) => {
                    let event = parse_google_event_text(bytes.as_ref())?;
                    if let Some(message) = google_error_message(&event) {
                        return Err(AppError::new("google_live_api_error", message));
                    }
                    if event.get("setupComplete").is_some() {
                        return Ok(());
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    return Err(unexpected_google_close_error(
                        frame.map(|frame| frame.to_string()),
                    ));
                }
                None => return Err(unexpected_google_close_error(None)),
                Some(Ok(_)) => {}
                Some(Err(err)) => {
                    return Err(AppError::new("google_live_receive_error", err.to_string()));
                }
            }
        }
    })
    .await;

    match result {
        Ok(setup_result) => setup_result,
        Err(_) => Ok(()),
    }
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
            "inputAudioTranscription": {},
            "outputAudioTranscription": {},
            "generationConfig": {
                "responseModalities": ["AUDIO"],
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

fn drain_complete_google_audio_chunks(
    pending_audio: &mut Vec<i16>,
    samples: &[i16],
) -> Vec<Vec<i16>> {
    pending_audio.extend_from_slice(samples);
    let chunk_size = google_audio_chunk_sample_count();
    let complete_len = (pending_audio.len() / chunk_size) * chunk_size;
    if complete_len == 0 {
        return Vec::new();
    }

    pending_audio
        .drain(..complete_len)
        .collect::<Vec<_>>()
        .chunks(chunk_size)
        .map(|chunk| chunk.to_vec())
        .collect()
}

fn drain_remaining_google_audio(pending_audio: &mut Vec<i16>) -> Option<Vec<i16>> {
    if pending_audio.is_empty() {
        return None;
    }

    Some(std::mem::take(pending_audio))
}

fn google_audio_chunk_sample_count() -> usize {
    (GOOGLE_LIVE_INPUT_SAMPLE_RATE as usize * GOOGLE_AUDIO_CHUNK_MS) / 1000
}

fn google_audio_stream_end_message() -> Value {
    json!({
        "realtimeInput": {
            "audioStreamEnd": true
        }
    })
}

fn unexpected_google_close_error(reason: Option<String>) -> AppError {
    let detail = reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .unwrap_or("the remote socket closed without a close reason");

    AppError::new(
        "google_live_connection_closed",
        format!("Google Live closed the translation connection unexpectedly: {detail}."),
    )
}

fn parse_google_event_text(bytes: &[u8]) -> AppResult<Value> {
    let text = std::str::from_utf8(bytes)
        .map_err(|err| AppError::new("google_live_event_parse_error", err.to_string()))?;
    serde_json::from_str::<Value>(text)
        .map_err(|err| AppError::new("google_live_event_parse_error", err.to_string()))
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
        append_transcript_text(&mut last.source_text, &item.source_text, false);
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
    if current.is_empty() {
        current.push_str(delta);
        return;
    }
    if break_after_sentence && should_start_new_transcript_line(current, delta) {
        current.truncate(current.trim_end().len());
        current.push('\n');
        current.push_str(delta.trim_start());
        return;
    }
    if should_insert_space_between_chunks(current, delta) {
        current.truncate(current.trim_end().len());
        current.push(' ');
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

fn should_insert_space_between_chunks(current: &str, delta: &str) -> bool {
    let previous = current.trim_end().chars().next_back();
    let next = delta.trim_start().chars().next();

    if previous.is_none() || next.is_none() {
        return false;
    }
    if current
        .chars()
        .next_back()
        .is_some_and(char::is_whitespace)
        || delta.chars().next().is_some_and(char::is_whitespace)
    {
        return false;
    }

    let previous = previous.unwrap();
    let next = next.unwrap();
    previous.is_alphanumeric()
        && next.is_alphanumeric()
        && !matches!(next, ',' | '.' | ';' | ':' | '!' | '?' | '%' | ')')
        && !matches!(previous, '(' | '[')
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
    fn unexpected_google_close_frame_is_an_error() {
        let error = unexpected_google_close_error(Some("policy violation".to_string()));

        assert_eq!(error.code, "google_live_connection_closed");
        assert!(error.message.contains("policy violation"));
    }

    #[tokio::test]
    async fn accepts_google_setup_complete_event() {
        let events: Vec<Result<Message, tokio_tungstenite::tungstenite::Error>> =
            vec![Ok(Message::Text(r#"{"setupComplete":{}}"#.into()))];
        let mut reader = futures_util::stream::iter(events);

        wait_for_google_setup_complete(&mut reader)
            .await
            .expect("setupComplete should pass");
    }

    #[tokio::test]
    async fn accepts_binary_google_setup_complete_event() {
        let events: Vec<Result<Message, tokio_tungstenite::tungstenite::Error>> = vec![Ok(
            Message::Binary(r#"{"setupComplete":{}}"#.as_bytes().to_vec().into()),
        )];
        let mut reader = futures_util::stream::iter(events);

        wait_for_google_setup_complete(&mut reader)
            .await
            .expect("binary setupComplete should pass");
    }

    #[tokio::test]
    async fn setup_waiter_surfaces_google_api_error() {
        let events: Vec<Result<Message, tokio_tungstenite::tungstenite::Error>> = vec![Ok(
            Message::Text(r#"{"error":{"message":"Invalid setup"}}"#.into()),
        )];
        let mut reader = futures_util::stream::iter(events);

        let error = wait_for_google_setup_complete(&mut reader)
            .await
            .expect_err("Google API errors should fail setup");

        assert_eq!(error.code, "google_live_api_error");
        assert_eq!(error.message, "Invalid setup");
    }

    #[tokio::test]
    async fn setup_waiter_allows_open_socket_without_confirmation() {
        let mut reader = futures_util::stream::pending::<
            Result<Message, tokio_tungstenite::tungstenite::Error>,
        >();

        wait_for_google_setup_complete_with_timeout(&mut reader, Duration::from_millis(1))
            .await
            .expect("quiet open sockets should be allowed to start audio");
    }

    #[test]
    fn repairs_missing_spaces_between_streamed_word_chunks() {
        let mut transcript = vec![TranscriptItem {
            id: "1".to_string(),
            timestamp_ms: 1,
            source_text: "To test your".to_string(),
            translated_text: "De kiem tra".to_string(),
            status: TranscriptStatus::Partial,
            latency_ms: None,
        }];

        merge_transcript_delta(
            &mut transcript,
            TranscriptItem {
                id: "2".to_string(),
                timestamp_ms: 2,
                source_text: "call quality,".to_string(),
                translated_text: String::new(),
                status: TranscriptStatus::Partial,
                latency_ms: None,
            },
        );
        merge_transcript_delta(
            &mut transcript,
            TranscriptItem {
                id: "3".to_string(),
                timestamp_ms: 3,
                source_text: String::new(),
                translated_text: "chat luong cuoc goi".to_string(),
                status: TranscriptStatus::Partial,
                latency_ms: None,
            },
        );

        assert_eq!(transcript.len(), 1);
        assert_eq!(transcript[0].source_text, "To test your call quality,");
        assert_eq!(
            transcript[0].translated_text,
            "De kiem tra chat luong cuoc goi"
        );
    }

    #[tokio::test]
    #[ignore = "requires GEMINI_API_KEY and live Google network access"]
    async fn google_live_translation_setup_smoke_test() {
        let api_key =
            std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run this smoke test");

        test_live_translation_connection(&api_key)
            .await
            .expect("Google Live Translation setup should complete");
    }

    #[tokio::test]
    #[ignore = "requires GEMINI_API_KEY, macOS say, ffmpeg, and live Google network access"]
    async fn google_live_translation_output_smoke_test() {
        let api_key =
            std::env::var("GEMINI_API_KEY").expect("set GEMINI_API_KEY to run this smoke test");
        let samples = speech_fixture_samples();
        let result = live_translation_smoke_result(&api_key, "vi", &samples)
            .await
            .expect("Google Live Translation should return translated text or audio");

        assert!(
            result.audio_sample_count > 0 || !result.output_text.trim().is_empty(),
            "expected translated audio samples or output transcript, got {result:?}"
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
                .pointer("/setup/inputAudioTranscription")
                .and_then(Value::as_object)
                .map(serde_json::Map::is_empty),
            Some(true)
        );
        assert_eq!(
            payload
                .pointer("/setup/outputAudioTranscription")
                .and_then(Value::as_object)
                .map(serde_json::Map::is_empty),
            Some(true)
        );
        assert!(payload
            .pointer("/setup/generationConfig/inputAudioTranscription")
            .is_none());
        assert!(payload
            .pointer("/setup/generationConfig/outputAudioTranscription")
            .is_none());
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
    fn batches_google_audio_into_100_ms_chunks() {
        let mut pending_audio = Vec::new();
        let chunks = drain_complete_google_audio_chunks(&mut pending_audio, &vec![1; 900]);

        assert!(chunks.is_empty());
        assert_eq!(pending_audio.len(), 900);

        let chunks = drain_complete_google_audio_chunks(&mut pending_audio, &vec![2; 2300]);

        assert_eq!(google_audio_chunk_sample_count(), 1600);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 1600);
        assert_eq!(chunks[1].len(), 1600);
        assert!(pending_audio.is_empty());
    }

    #[test]
    fn keeps_partial_google_audio_until_flush() {
        let mut pending_audio = Vec::new();
        let chunks = drain_complete_google_audio_chunks(&mut pending_audio, &vec![1; 1700]);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 1600);
        assert_eq!(pending_audio.len(), 100);

        let remainder = drain_remaining_google_audio(&mut pending_audio);

        assert_eq!(remainder.expect("remaining audio").len(), 100);
        assert!(pending_audio.is_empty());
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

    #[derive(Debug, Default)]
    struct LiveTranslationSmokeResult {
        input_text: String,
        output_text: String,
        audio_sample_count: usize,
    }

    async fn live_translation_smoke_result(
        api_key: &str,
        target_language: &str,
        samples: &[i16],
    ) -> AppResult<LiveTranslationSmokeResult> {
        let request_url = live_translate_url(api_key);
        let (socket, _) = connect_async(&request_url).await.map_err(|err| {
            AppError::new(
                "google_live_connect_error",
                redact_api_key(&err.to_string(), api_key),
            )
        })?;
        let (mut writer, mut reader) = socket.split();

        writer
            .send(Message::Text(
                google_setup_message(target_language).to_string().into(),
            ))
            .await
            .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;
        wait_for_google_setup_complete(&mut reader).await?;

        for chunk in samples.chunks(google_audio_chunk_sample_count()) {
            send_google_audio(&mut writer, chunk).await?;
            tokio::time::sleep(Duration::from_millis(GOOGLE_AUDIO_CHUNK_MS as u64)).await;
        }
        writer
            .send(Message::Text(
                google_audio_stream_end_message().to_string().into(),
            ))
            .await
            .map_err(|err| AppError::new("google_live_send_error", err.to_string()))?;

        let result = wait_for_translated_smoke_output(&mut reader).await;
        let _ = writer.send(Message::Close(None)).await;
        result
    }

    async fn wait_for_translated_smoke_output<R>(
        reader: &mut R,
    ) -> AppResult<LiveTranslationSmokeResult>
    where
        R: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        let result = timeout(Duration::from_secs(45), async {
            let mut smoke = LiveTranslationSmokeResult::default();
            loop {
                match reader.next().await {
                    Some(Ok(Message::Text(text))) => {
                        let event = parse_google_event_text(text.as_ref())?;
                        if let Some(message) = google_error_message(&event) {
                            return Err(AppError::new("google_live_api_error", message));
                        }
                        if let Some(text) = transcript_text(&event, true) {
                            smoke.input_text.push_str(&text);
                        }
                        if let Some(text) = transcript_text(&event, false) {
                            smoke.output_text.push_str(&text);
                        }
                        for samples in audio_parts(&event)? {
                            smoke.audio_sample_count += samples.len();
                        }
                        if smoke.audio_sample_count > 0 || !smoke.output_text.trim().is_empty() {
                            return Ok(smoke);
                        }
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let event = parse_google_event_text(bytes.as_ref())?;
                        if let Some(message) = google_error_message(&event) {
                            return Err(AppError::new("google_live_api_error", message));
                        }
                        if let Some(text) = transcript_text(&event, true) {
                            smoke.input_text.push_str(&text);
                        }
                        if let Some(text) = transcript_text(&event, false) {
                            smoke.output_text.push_str(&text);
                        }
                        for samples in audio_parts(&event)? {
                            smoke.audio_sample_count += samples.len();
                        }
                        if smoke.audio_sample_count > 0 || !smoke.output_text.trim().is_empty() {
                            return Ok(smoke);
                        }
                    }
                    Some(Ok(Message::Close(frame))) => {
                        return Err(unexpected_google_close_error(
                            frame.map(|frame| frame.to_string()),
                        ));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        return Err(AppError::new("google_live_receive_error", err.to_string()));
                    }
                    None => return Err(unexpected_google_close_error(None)),
                }
            }
        })
        .await;

        match result {
            Ok(output) => output,
            Err(_) => Err(AppError::new(
                "google_live_translation_output_timeout",
                "Google Live did not return translated audio or output transcript for the smoke fixture.",
            )),
        }
    }

    fn speech_fixture_samples() -> Vec<i16> {
        let temp_dir = std::env::temp_dir();
        let prefix = format!("baka-trans-google-live-{}", Uuid::new_v4());
        let aiff_path = temp_dir.join(format!("{prefix}.aiff"));
        let pcm_path = temp_dir.join(format!("{prefix}.pcm"));

        let say_status = std::process::Command::new("say")
            .args([
                "-v",
                "Samantha",
                "-o",
                aiff_path.to_string_lossy().as_ref(),
                "Hello everyone. This is a live translation test.",
            ])
            .status()
            .expect("macOS say command is required for this smoke test");
        assert!(
            say_status.success(),
            "say failed to generate speech fixture"
        );

        let ffmpeg_status = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                aiff_path.to_string_lossy().as_ref(),
                "-ac",
                "1",
                "-ar",
                "16000",
                "-f",
                "s16le",
                "-acodec",
                "pcm_s16le",
                pcm_path.to_string_lossy().as_ref(),
            ])
            .status()
            .expect("ffmpeg is required for this smoke test");
        assert!(
            ffmpeg_status.success(),
            "ffmpeg failed to convert speech fixture"
        );

        let bytes = std::fs::read(&pcm_path).expect("read generated PCM fixture");
        let _ = std::fs::remove_file(aiff_path);
        let _ = std::fs::remove_file(pcm_path);

        bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }
}
