use crate::error::{AppError, AppResult};
use crate::models::{
    LocalTranslationConfig, LocalTranslationConfigDraft, LocalTranslationTestResult,
};
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use url::Url;
use whisper_rs::{WhisperContext, WhisperContextParameters};

const CONFIG_SCHEMA_VERSION: u32 = 1;
const CONFIG_DIR_NAME: &str = "dev.baka3k.baka-trans";
const CONFIG_FILE_NAME: &str = "local-translation-config.json";
const TRANSLATION_SYSTEM_PROMPT: &str = "Translate Japanese to Vietnamese. Return only the translation. Preserve names, numbers, and technical terms. Do not explain.";

impl Default for LocalTranslationConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            base_url: "http://localhost:11434".to_string(),
            model: String::new(),
            timeout_seconds: 30,
            temperature: 0.0,
            max_output_tokens: 256,
            keep_alive: Some("10m".to_string()),
            model_path: String::new(),
            language: "ja".to_string(),
            threads: default_thread_count(),
            use_gpu: false,
            sample_rate_hz: 16_000,
            minimum_speech_ms: 300,
            silence_to_commit_ms: 700,
            maximum_utterance_ms: 15_000,
            pre_roll_ms: 250,
            speech_threshold: 0.015,
        }
    }
}

impl Default for LocalTranslationConfigDraft {
    fn default() -> Self {
        LocalTranslationConfig::default().into()
    }
}

#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    endpoint: String,
    model: String,
    temperature: f32,
    max_output_tokens: u32,
    keep_alive: Option<String>,
}

impl OllamaClient {
    pub fn new(config: &LocalTranslationConfig) -> AppResult<Self> {
        let endpoint = normalize_ollama_chat_url(&config.base_url)?;
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|err| AppError::new("local_ollama_client_error", err.to_string()))?;
        Ok(Self {
            client,
            endpoint,
            model: config.model.clone(),
            temperature: config.temperature,
            max_output_tokens: config.max_output_tokens,
            keep_alive: config.keep_alive.clone(),
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn translate(&self, source_text: &str) -> AppResult<(String, u64)> {
        let source_text = source_text.trim();
        if source_text.is_empty() {
            return Err(AppError::new(
                "local_translation_empty_source",
                "Whisper returned no Japanese text to translate.",
            ));
        }
        let payload = build_ollama_payload(
            &self.model,
            source_text,
            self.temperature,
            self.max_output_tokens,
            self.keep_alive.as_deref(),
        );
        let started = Instant::now();
        let response = self
            .client
            .post(&self.endpoint)
            .json(&payload)
            .send()
            .await
            .map_err(|err| AppError::new("local_ollama_request_error", err.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| AppError::new("local_ollama_response_error", err.to_string()))?;
        let content = parse_ollama_response(status, &body)?;
        let latency_ms = started.elapsed().as_millis().min(86_400_000) as u64;
        Ok((content, latency_ms))
    }
}

pub fn get_config() -> AppResult<LocalTranslationConfig> {
    read_config_from_path(&config_path()?)
}

pub fn save_config(draft: LocalTranslationConfigDraft) -> AppResult<LocalTranslationConfig> {
    let config = normalize_and_validate(draft, true)?;
    write_config_to_path(&config_path()?, &config)?;
    Ok(config)
}

pub fn validated_runtime_config() -> AppResult<LocalTranslationConfig> {
    let config = get_config()?;
    normalize_and_validate(config.into(), true)
}

pub async fn test_config(
    draft: LocalTranslationConfigDraft,
) -> AppResult<LocalTranslationTestResult> {
    let fallback_model = draft.model.trim().to_string();
    let fallback_endpoint = normalize_ollama_chat_url(&draft.base_url)
        .unwrap_or_else(|_| draft.base_url.trim().to_string());
    let config = match normalize_and_validate(draft.clone(), true) {
        Ok(config) => config,
        Err(error) => {
            return Ok(failed_test_result(
                fallback_model,
                fallback_endpoint,
                error,
                validate_model_path(draft.model_path.trim()).is_ok(),
                false,
                false,
                false,
            ));
        }
    };
    let model_path = config.model_path.clone();
    let use_gpu = config.use_gpu;
    let whisper_result =
        tauri::async_runtime::spawn_blocking(move || load_whisper_context(&model_path, use_gpu))
            .await
            .map_err(|err| AppError::new("local_whisper_test_join_error", err.to_string()))?;
    if let Err(error) = whisper_result {
        return Ok(failed_test_result(
            config.model,
            normalize_ollama_chat_url(&config.base_url)?,
            error,
            true,
            false,
            false,
            false,
        ));
    }

    let ollama = OllamaClient::new(&config)?;
    let endpoint = ollama.endpoint().to_string();
    let (probe, _) = match ollama.translate("こんにちは").await {
        Ok(response) => response,
        Err(error) => {
            let reachable = error.code != "local_ollama_request_error";
            return Ok(failed_test_result(
                config.model,
                endpoint,
                error,
                true,
                true,
                reachable,
                false,
            ));
        }
    };
    Ok(LocalTranslationTestResult {
        ok: true,
        message: format!(
            "Whisper loaded and Ollama returned a {} character translation.",
            probe.chars().count()
        ),
        model: config.model,
        endpoint,
        whisper_model_readable: true,
        whisper_model_loaded: true,
        ollama_reachable: true,
        ollama_model_accepted: true,
    })
}

fn failed_test_result(
    model: String,
    endpoint: String,
    error: AppError,
    whisper_model_readable: bool,
    whisper_model_loaded: bool,
    ollama_reachable: bool,
    ollama_model_accepted: bool,
) -> LocalTranslationTestResult {
    LocalTranslationTestResult {
        ok: false,
        message: error.message,
        model,
        endpoint,
        whisper_model_readable,
        whisper_model_loaded,
        ollama_reachable,
        ollama_model_accepted,
    }
}

pub fn load_whisper_context(model_path: &str, use_gpu: bool) -> AppResult<WhisperContext> {
    validate_model_path(model_path)?;
    let mut parameters = WhisperContextParameters::default();
    parameters.use_gpu(use_gpu);
    WhisperContext::new_with_params(model_path, parameters).map_err(|err| {
        AppError::new(
            "local_whisper_model_load_error",
            format!("Could not load the Whisper model: {err}"),
        )
    })
}

pub fn normalize_and_validate(
    draft: LocalTranslationConfigDraft,
    require_model_path: bool,
) -> AppResult<LocalTranslationConfig> {
    let endpoint = normalize_ollama_chat_url(&draft.base_url)?;
    let model = require_non_empty(
        &draft.model,
        "local_ollama_model_missing",
        "Choose an installed Ollama model.",
    )?;
    let model_path = draft.model_path.trim().to_string();
    if require_model_path {
        validate_model_path(&model_path)?;
    }
    if draft.language.trim() != "ja" {
        return Err(AppError::new(
            "local_whisper_language_invalid",
            "Local translation currently supports Japanese (ja) input only.",
        ));
    }
    if draft.sample_rate_hz != 16_000 {
        return Err(AppError::new(
            "local_audio_sample_rate_invalid",
            "Local Whisper input must be PCM16 mono at exactly 16000 Hz.",
        ));
    }
    if !(100..=3_000).contains(&draft.minimum_speech_ms) {
        return Err(range_error("minimumSpeechMs", 100, 3_000));
    }
    if !(200..=5_000).contains(&draft.silence_to_commit_ms) {
        return Err(range_error("silenceToCommitMs", 200, 5_000));
    }
    if !(1_000..=60_000).contains(&draft.maximum_utterance_ms) {
        return Err(range_error("maximumUtteranceMs", 1_000, 60_000));
    }
    if draft.minimum_speech_ms > draft.maximum_utterance_ms {
        return Err(AppError::new(
            "local_audio_segmentation_invalid",
            "minimumSpeechMs cannot exceed maximumUtteranceMs.",
        ));
    }
    if draft.pre_roll_ms > 2_000 {
        return Err(range_error("preRollMs", 0, 2_000));
    }
    if !(0.001..=0.25).contains(&draft.speech_threshold) {
        return Err(AppError::new(
            "local_audio_threshold_invalid",
            "speechThreshold must be between 0.001 and 0.25.",
        ));
    }
    let threads = draft.threads.clamp(1, maximum_thread_count());
    let keep_alive = draft
        .keep_alive
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);

    Ok(LocalTranslationConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        base_url: endpoint
            .strip_suffix("/api/chat")
            .unwrap_or(&endpoint)
            .trim_end_matches('/')
            .to_string(),
        model,
        timeout_seconds: draft.timeout_seconds.clamp(5, 300),
        temperature: draft.temperature.clamp(0.0, 1.0),
        max_output_tokens: draft.max_output_tokens.clamp(32, 2_048),
        keep_alive,
        model_path,
        language: "ja".to_string(),
        threads,
        use_gpu: draft.use_gpu,
        sample_rate_hz: 16_000,
        minimum_speech_ms: draft.minimum_speech_ms,
        silence_to_commit_ms: draft.silence_to_commit_ms,
        maximum_utterance_ms: draft.maximum_utterance_ms,
        pre_roll_ms: draft.pre_roll_ms,
        speech_threshold: draft.speech_threshold,
    })
}

pub fn normalize_ollama_chat_url(base_url: &str) -> AppResult<String> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "local_ollama_base_url_missing",
            "Enter the Ollama server URL.",
        ));
    }
    let mut url = Url::parse(trimmed).map_err(|err| {
        AppError::new(
            "local_ollama_base_url_invalid",
            format!("Enter a valid Ollama URL: {err}"),
        )
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::new(
            "local_ollama_base_url_invalid",
            "Ollama URL must use http or https.",
        ));
    }
    let path = url.path().trim_end_matches('/');
    if !path.is_empty() && path != "/api/chat" {
        return Err(AppError::new(
            "local_ollama_base_url_invalid",
            "Use the Ollama server origin or its full /api/chat endpoint.",
        ));
    }
    url.set_path("/api/chat");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub fn build_ollama_payload(
    model: &str,
    source_text: &str,
    temperature: f32,
    max_output_tokens: u32,
    keep_alive: Option<&str>,
) -> Value {
    let mut payload = json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": TRANSLATION_SYSTEM_PROMPT },
            { "role": "user", "content": source_text }
        ],
        "options": {
            "temperature": temperature,
            "num_predict": max_output_tokens
        }
    });
    if let Some(keep_alive) = keep_alive {
        payload["keep_alive"] = json!(keep_alive);
    }
    payload
}

pub fn parse_ollama_response(status: StatusCode, body: &str) -> AppResult<String> {
    let parsed = serde_json::from_str::<Value>(body);
    if status != StatusCode::OK {
        if let Ok(value) = &parsed {
            if let Some(error) = value.get("error").and_then(Value::as_str) {
                return Err(AppError::new(
                    "local_ollama_provider_error",
                    format!("Ollama error: {}", compact(error)),
                ));
            }
        }
        return Err(AppError::new(
            "local_ollama_provider_error",
            format!("Ollama returned {status}: {}", compact(body)),
        ));
    }
    let value = parsed.map_err(|err| {
        AppError::new(
            "local_ollama_response_parse_error",
            format!("Ollama returned malformed JSON: {err}"),
        )
    })?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(AppError::new(
            "local_ollama_provider_error",
            format!("Ollama error: {}", compact(error)),
        ));
    }
    let content = value
        .pointer("/message/content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if content.is_empty() {
        return Err(AppError::new(
            "local_ollama_empty_response",
            "Ollama returned an empty translation.",
        ));
    }
    Ok(content)
}

fn read_config_from_path(path: &Path) -> AppResult<LocalTranslationConfig> {
    if !path.exists() {
        return Ok(LocalTranslationConfig::default());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|err| AppError::new("local_config_read_error", err.to_string()))?;
    let config: LocalTranslationConfig = serde_json::from_str(&raw).map_err(|err| {
        AppError::new(
            "local_config_parse_error",
            format!("Could not parse {}: {err}", path.display()),
        )
    })?;
    if config.schema_version != CONFIG_SCHEMA_VERSION {
        return Err(AppError::new(
            "local_config_version_error",
            format!(
                "Unsupported local translation config version {}.",
                config.schema_version
            ),
        ));
    }
    Ok(config)
}

fn write_config_to_path(path: &Path, config: &LocalTranslationConfig) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            "local_config_path_error",
            "Local translation config path has no parent directory.",
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|err| AppError::new("local_config_write_error", err.to_string()))?;
    let raw = serde_json::to_vec_pretty(config)
        .map_err(|err| AppError::new("local_config_write_error", err.to_string()))?;
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");
    std::fs::write(&temporary, raw)
        .map_err(|err| AppError::new("local_config_write_error", err.to_string()))?;

    if path.exists() {
        let _ = std::fs::remove_file(&backup);
        std::fs::rename(path, &backup)
            .map_err(|err| AppError::new("local_config_write_error", err.to_string()))?;
    }
    if let Err(err) = std::fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = std::fs::rename(&backup, path);
        }
        let _ = std::fs::remove_file(&temporary);
        return Err(AppError::new("local_config_write_error", err.to_string()));
    }
    let _ = std::fs::remove_file(backup);
    Ok(())
}

fn config_path() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA").ok_or_else(|| {
            AppError::new(
                "local_config_path_error",
                "Could not resolve APPDATA for local translation settings.",
            )
        })?;
        Ok(PathBuf::from(app_data)
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| {
            AppError::new(
                "local_config_path_error",
                "Could not resolve HOME for local translation settings.",
            )
        })?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(CONFIG_DIR_NAME)
            .join(CONFIG_FILE_NAME))
    }
}

fn validate_model_path(model_path: &str) -> AppResult<()> {
    if model_path.trim().is_empty() {
        return Err(AppError::new(
            "local_whisper_model_missing",
            "Choose a local Whisper GGML model file.",
        ));
    }
    let path = Path::new(model_path);
    let metadata = std::fs::metadata(path).map_err(|err| {
        AppError::new(
            "local_whisper_model_unreadable",
            format!("Cannot read Whisper model {}: {err}", path.display()),
        )
    })?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(AppError::new(
            "local_whisper_model_invalid",
            "The Whisper model path must point to a non-empty file.",
        ));
    }
    Ok(())
}

fn require_non_empty(value: &str, code: &str, message: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(code, message));
    }
    Ok(trimmed.to_string())
}

fn range_error(field: &str, minimum: u64, maximum: u64) -> AppError {
    AppError::new(
        "local_audio_segmentation_invalid",
        format!("{field} must be between {minimum} and {maximum}."),
    )
}

fn maximum_thread_count() -> u32 {
    std::thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1)
        .max(1)
}

fn default_thread_count() -> u32 {
    maximum_thread_count().min(4)
}

fn compact(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use uuid::Uuid;

    #[test]
    fn defaults_use_fixed_japanese_pcm_contract() {
        let config = LocalTranslationConfig::default();
        assert_eq!(config.language, "ja");
        assert_eq!(config.sample_rate_hz, 16_000);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn normalizes_server_origin_and_full_endpoint() {
        assert_eq!(
            normalize_ollama_chat_url("http://localhost:11434/").unwrap(),
            "http://localhost:11434/api/chat"
        );
        assert_eq!(
            normalize_ollama_chat_url("http://localhost:11434/api/chat").unwrap(),
            "http://localhost:11434/api/chat"
        );
        assert!(normalize_ollama_chat_url("file:///tmp/ollama").is_err());
        assert!(normalize_ollama_chat_url("http://localhost:11434/v1").is_err());
    }

    #[test]
    fn builds_exact_native_ollama_payload() {
        let payload = build_ollama_payload("qwen", "こんにちは", 0.0, 128, Some("10m"));
        assert_eq!(payload["model"], "qwen");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["messages"][1]["content"], "こんにちは");
        assert_eq!(payload["options"]["temperature"], 0.0);
        assert_eq!(payload["options"]["num_predict"], 128);
        assert_eq!(payload["keep_alive"], "10m");
        assert!(payload.get("max_tokens").is_none());
    }

    #[test]
    fn parses_success_and_native_errors() {
        assert_eq!(
            parse_ollama_response(StatusCode::OK, r#"{"message":{"content":" Xin chào "}}"#)
                .unwrap(),
            "Xin chào"
        );
        assert_eq!(
            parse_ollama_response(StatusCode::OK, r#"{"error":"model not found"}"#)
                .unwrap_err()
                .code,
            "local_ollama_provider_error"
        );
        assert_eq!(
            parse_ollama_response(StatusCode::OK, "not-json")
                .unwrap_err()
                .code,
            "local_ollama_response_parse_error"
        );
        assert_eq!(
            parse_ollama_response(StatusCode::BAD_GATEWAY, "upstream offline")
                .unwrap_err()
                .code,
            "local_ollama_provider_error"
        );
        assert_eq!(
            parse_ollama_response(StatusCode::OK, r#"{"message":{"content":" "}}"#)
                .unwrap_err()
                .code,
            "local_ollama_empty_response"
        );
    }

    #[test]
    fn persists_versioned_config_round_trip() {
        let directory = std::env::temp_dir().join(format!("baka-trans-{}", Uuid::new_v4()));
        let path = directory.join(CONFIG_FILE_NAME);
        let config = LocalTranslationConfig::default();
        write_config_to_path(&path, &config).unwrap();
        assert_eq!(read_config_from_path(&path).unwrap(), config);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn rejects_missing_models_and_invalid_audio_contracts() {
        let missing =
            normalize_and_validate(LocalTranslationConfigDraft::default(), true).unwrap_err();
        assert_eq!(missing.code, "local_ollama_model_missing");

        let mut draft = LocalTranslationConfigDraft {
            model: "qwen".to_string(),
            model_path: "definitely-not-a-real-whisper-model.bin".to_string(),
            ..LocalTranslationConfigDraft::default()
        };
        assert_eq!(
            normalize_and_validate(draft.clone(), true)
                .unwrap_err()
                .code,
            "local_whisper_model_unreadable"
        );
        draft.sample_rate_hz = 24_000;
        assert_eq!(
            normalize_and_validate(draft, false).unwrap_err().code,
            "local_audio_sample_rate_invalid"
        );
    }

    #[tokio::test]
    async fn test_config_reports_partial_whisper_health() {
        let model_path = std::env::temp_dir().join(format!("baka-trans-{}.bin", Uuid::new_v4()));
        std::fs::write(&model_path, b"not-a-whisper-model").unwrap();
        let result = test_config(LocalTranslationConfigDraft {
            model: "qwen".to_string(),
            model_path: model_path.to_string_lossy().into_owned(),
            ..LocalTranslationConfigDraft::default()
        })
        .await
        .unwrap();

        assert!(!result.ok);
        assert!(result.whisper_model_readable);
        assert!(!result.whisper_model_loaded);
        assert!(!result.ollama_reachable);
        let _ = std::fs::remove_file(model_path);
    }

    #[tokio::test]
    async fn posts_to_native_api_chat_and_parses_content() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = vec![0_u8; 16_384];
            let count = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]);
            assert!(request.starts_with("POST /api/chat HTTP/1.1"));
            assert!(request.contains("\"stream\":false"));
            assert!(!request.contains("/v1/chat/completions"));
            let body = r#"{"message":{"content":"Xin chào"}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let config = LocalTranslationConfig {
            base_url: format!("http://{address}"),
            model: "qwen".to_string(),
            ..LocalTranslationConfig::default()
        };
        let client = OllamaClient::new(&config).unwrap();
        let (content, _) = client.translate("こんにちは").await.unwrap();
        assert_eq!(content, "Xin chào");
        server.join().unwrap();
    }
}
