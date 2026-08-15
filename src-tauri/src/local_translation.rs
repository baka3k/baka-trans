use crate::error::{AppError, AppResult};
use crate::models::{
    Language, LocalTranslationConfig, LocalTranslationConfigDraft, LocalTranslationEngine, LocalTranslationTestResult,
    LocalTtsProvider, WhisperModelDownloadProgress, WhisperModelOption,
};
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use url::Url;
use whisper_rs::{get_lang_id, WhisperContext, WhisperContextParameters};

const CONFIG_SCHEMA_VERSION: u32 = 3;
const CONFIG_DIR_NAME: &str = "dev.baka3k.baka-trans";
const CONFIG_FILE_NAME: &str = "local-translation-config.json";
const WHISPER_MODEL_DIR_NAME: &str = "whisper-models";
const WHISPER_MODEL_SOURCE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const DEFAULT_OLLAMA_MODEL: &str = "translategemma:4b";
const LEGACY_DEFAULT_OLLAMA_MODEL: &str = "gemma3:4b";
static WHISPER_DOWNLOAD_ACTIVE: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug)]
struct WhisperModelSpec {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    file_name: &'static str,
    size_mib: u64,
    recommended: bool,
}

const WHISPER_MODEL_SPECS: [WhisperModelSpec; 3] = [
    WhisperModelSpec {
        id: "base-q5_1",
        label: "Base Q5",
        description: "Fast and compact; suitable for a quick local setup.",
        file_name: "ggml-base-q5_1.bin",
        size_mib: 57,
        recommended: false,
    },
    WhisperModelSpec {
        id: "small-q5_1",
        label: "Small Q5",
        description: "Good Japanese accuracy without the full model size.",
        file_name: "ggml-small-q5_1.bin",
        size_mib: 181,
        recommended: true,
    },
    WhisperModelSpec {
        id: "small",
        label: "Small",
        description: "Higher accuracy, with a larger download and slower inference.",
        file_name: "ggml-small.bin",
        size_mib: 466,
        recommended: false,
    },
];

struct WhisperDownloadGuard;

impl WhisperDownloadGuard {
    fn acquire() -> AppResult<Self> {
        WHISPER_DOWNLOAD_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                AppError::new(
                    "local_whisper_download_busy",
                    "Another Whisper model download is already in progress.",
                )
            })?;
        Ok(Self)
    }
}

impl Drop for WhisperDownloadGuard {
    fn drop(&mut self) {
        WHISPER_DOWNLOAD_ACTIVE.store(false, Ordering::Release);
    }
}

pub fn whisper_models() -> Vec<WhisperModelOption> {
    WHISPER_MODEL_SPECS
        .iter()
        .map(|model| WhisperModelOption {
            id: model.id.to_string(),
            label: model.label.to_string(),
            description: model.description.to_string(),
            file_name: model.file_name.to_string(),
            size_mib: model.size_mib,
            recommended: model.recommended,
        })
        .collect()
}

pub async fn download_whisper_model(app: &AppHandle, model_id: &str) -> AppResult<String> {
    let spec = whisper_model_spec(model_id)?;
    let _guard = WhisperDownloadGuard::acquire()?;
    let model_dir = whisper_model_dir()?;
    let destination = model_dir.join(spec.file_name);
    let temporary = model_dir.join(format!("{}.part", spec.file_name));

    if destination
        .metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
    {
        emit_whisper_download_progress(
            app,
            spec,
            destination
                .metadata()
                .ok()
                .map(|metadata| metadata.len())
                .unwrap_or(0),
            destination.metadata().ok().map(|metadata| metadata.len()),
            "completed",
            "Model is already downloaded.",
        );
        return Ok(destination.to_string_lossy().into_owned());
    }

    tokio::fs::create_dir_all(&model_dir).await.map_err(|err| {
        AppError::new(
            "local_whisper_download_write_error",
            format!("Could not create the Whisper model folder: {err}"),
        )
    })?;
    let _ = tokio::fs::remove_file(&temporary).await;

    let result: AppResult<String> = async {
        let url = format!("{WHISPER_MODEL_SOURCE}/{}?download=true", spec.file_name);
        let response = Client::builder()
            .timeout(Duration::from_secs(30 * 60))
            .build()
            .map_err(|err| AppError::new("local_whisper_download_client_error", err.to_string()))?
            .get(url)
            .send()
            .await
            .map_err(|err| {
                AppError::new(
                    "local_whisper_download_network_error",
                    format!("Could not start the Whisper model download: {err}"),
                )
            })?
            .error_for_status()
            .map_err(|err| {
                AppError::new(
                    "local_whisper_download_http_error",
                    format!("The Whisper model server rejected the download: {err}"),
                )
            })?;
        let total_bytes = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&temporary).await.map_err(|err| {
            AppError::new(
                "local_whisper_download_write_error",
                format!("Could not create the temporary model file: {err}"),
            )
        })?;
        let mut downloaded_bytes = 0_u64;
        let mut last_percent = None;

        emit_whisper_download_progress(
            app,
            spec,
            downloaded_bytes,
            total_bytes,
            "downloading",
            "Downloading model…",
        );

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|err| {
                AppError::new(
                    "local_whisper_download_network_error",
                    format!("The Whisper model download was interrupted: {err}"),
                )
            })?;
            file.write_all(&chunk).await.map_err(|err| {
                AppError::new(
                    "local_whisper_download_write_error",
                    format!("Could not write the Whisper model: {err}"),
                )
            })?;
            downloaded_bytes += chunk.len() as u64;
            let percent = download_percent(downloaded_bytes, total_bytes);
            if percent != last_percent {
                last_percent = percent;
                emit_whisper_download_progress(
                    app,
                    spec,
                    downloaded_bytes,
                    total_bytes,
                    "downloading",
                    "Downloading model…",
                );
            }
        }

        file.flush().await.map_err(|err| {
            AppError::new(
                "local_whisper_download_write_error",
                format!("Could not finish writing the Whisper model: {err}"),
            )
        })?;
        drop(file);

        if downloaded_bytes == 0 || total_bytes.is_some_and(|total| total != downloaded_bytes) {
            return Err(AppError::new(
                "local_whisper_download_incomplete",
                "The Whisper model download was incomplete. Please try again.",
            ));
        }

        tokio::fs::rename(&temporary, &destination)
            .await
            .map_err(|err| {
                AppError::new(
                    "local_whisper_download_write_error",
                    format!("Could not install the downloaded Whisper model: {err}"),
                )
            })?;
        emit_whisper_download_progress(
            app,
            spec,
            downloaded_bytes,
            total_bytes.or(Some(downloaded_bytes)),
            "completed",
            "Model downloaded. Save settings, then test the local pipeline.",
        );
        Ok(destination.to_string_lossy().into_owned())
    }
    .await;

    if let Err(error) = &result {
        let _ = tokio::fs::remove_file(&temporary).await;
        emit_whisper_download_progress(app, spec, 0, None, "error", &error.message);
    }
    result
}

fn whisper_model_spec(model_id: &str) -> AppResult<&'static WhisperModelSpec> {
    WHISPER_MODEL_SPECS
        .iter()
        .find(|model| model.id == model_id)
        .ok_or_else(|| {
            AppError::new(
                "local_whisper_model_unknown",
                "Choose a supported Whisper model from the download list.",
            )
        })
}

fn whisper_model_dir() -> AppResult<PathBuf> {
    let config = config_path()?;
    let parent = config.parent().ok_or_else(|| {
        AppError::new(
            "local_config_path_error",
            "Local translation config path has no parent directory.",
        )
    })?;
    Ok(parent.join(WHISPER_MODEL_DIR_NAME))
}

fn download_percent(downloaded_bytes: u64, total_bytes: Option<u64>) -> Option<u8> {
    total_bytes
        .filter(|total| *total > 0)
        .map(|total| ((downloaded_bytes.saturating_mul(100) / total).min(100)) as u8)
}

fn emit_whisper_download_progress(
    app: &AppHandle,
    spec: &WhisperModelSpec,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    status: &str,
    message: &str,
) {
    let _ = app.emit(
        "whisper-model-download-progress",
        WhisperModelDownloadProgress {
            model_id: spec.id.to_string(),
            file_name: spec.file_name.to_string(),
            downloaded_bytes,
            total_bytes,
            percent: download_percent(downloaded_bytes, total_bytes),
            status: status.to_string(),
            message: message.to_string(),
        },
    );
}

impl Default for LocalTranslationConfig {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            translation_engine: LocalTranslationEngine::HuggingfaceOffline,
            openai_base_url: String::new(),
            openai_model: String::new(),
            openai_timeout_seconds: 30,
            openai_temperature: 0.0,
            openai_max_output_tokens: 256,
            base_url: String::new(),
            model: String::new(),
            timeout_seconds: 0,
            temperature: 0.0,
            max_output_tokens: 0,
            keep_alive: None,
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
            tts_provider: LocalTtsProvider::System,
            vieneu_base_url: "http://127.0.0.1:23334".to_string(),
            vieneu_style: "tu_nhien".to_string(),
            voice_id: String::new(),
            tts_rate: 1.0,
            tts_volume: 1.0,
            tts_output_sample_rate_hz: crate::tts::LOCAL_TTS_SAMPLE_RATE,
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
    system_prompt: String,
}

impl OllamaClient {
    pub fn new(
        config: &LocalTranslationConfig,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        let endpoint = normalize_ollama_chat_url(&config.base_url)?;
        if target_language == Language::Auto {
            return Err(AppError::new(
                "unsupported_target_language",
                "Choose an explicit target language for local translation.",
            ));
        }
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
            system_prompt: translation_system_prompt(source_language, target_language),
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
                "Whisper returned no text to translate.",
            ));
        }
        let payload = build_ollama_payload(
            &self.model,
            source_text,
            &self.system_prompt,
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
    app: Option<&tauri::AppHandle>,
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
                LocalTestHealth {
                    whisper_model_readable: validate_model_path(draft.model_path.trim()).is_ok(),
                    ..LocalTestHealth::default()
                },
            ));
        }
    };
    let tts_voice_available = crate::tts::voice_is_available(app, &config).await?;
    if !tts_voice_available {
        let provider_name = match config.tts_provider {
            LocalTtsProvider::System => "system",
            LocalTtsProvider::Vieneu => "VieNeu-TTS",
        };
        return Ok(failed_test_result(
            config.model,
            normalize_ollama_chat_url(&config.base_url)?,
            AppError::new(
                "local_tts_voice_missing",
                format!("The selected {provider_name} voice is no longer available."),
            ),
            LocalTestHealth {
                whisper_model_readable: true,
                ..LocalTestHealth::default()
            },
        ));
    }
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
            LocalTestHealth {
                whisper_model_readable: true,
                tts_voice_available: true,
                ..LocalTestHealth::default()
            },
        ));
    }

    let ollama = OllamaClient::new(&config, Language::Ja, Language::Vi)?;
    let endpoint = ollama.endpoint().to_string();
    let (probe, _) = match ollama.translate("こんにちは").await {
        Ok(response) => response,
        Err(error) => {
            let reachable = error.code != "local_ollama_request_error";
            return Ok(failed_test_result(
                config.model,
                endpoint,
                error,
                LocalTestHealth {
                    whisper_model_readable: true,
                    whisper_model_loaded: true,
                    ollama_reachable: reachable,
                    tts_voice_available: true,
                    ..LocalTestHealth::default()
                },
            ));
        }
    };
    Ok(LocalTranslationTestResult {
        ok: true,
        message: format!(
            "Whisper, Gemma, and the selected voice are ready. Probe translation: {} characters.",
            probe.chars().count()
        ),
        model: config.model,
        endpoint,
        whisper_model_readable: true,
        whisper_model_loaded: true,
        ollama_reachable: true,
        ollama_model_accepted: true,
        tts_voice_available: true,
    })
}

#[derive(Default)]
struct LocalTestHealth {
    whisper_model_readable: bool,
    whisper_model_loaded: bool,
    ollama_reachable: bool,
    ollama_model_accepted: bool,
    tts_voice_available: bool,
}

fn failed_test_result(
    model: String,
    endpoint: String,
    error: AppError,
    health: LocalTestHealth,
) -> LocalTranslationTestResult {
    LocalTranslationTestResult {
        ok: false,
        message: error.message,
        model,
        endpoint,
        whisper_model_readable: health.whisper_model_readable,
        whisper_model_loaded: health.whisper_model_loaded,
        ollama_reachable: health.ollama_reachable,
        ollama_model_accepted: health.ollama_model_accepted,
        tts_voice_available: health.tts_voice_available,
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
    // Retained in the persisted schema for backward compatibility. VieNeu is now
    // reached exclusively through the app-managed, authenticated runtime.
    let vieneu_base_url = "managed://vieneu".to_string();
    let vieneu_style = draft.vieneu_style.trim();
    if !matches!(vieneu_style, "tu_nhien" | "tin_tuc" | "doc_truyen") {
        return Err(AppError::new(
            "local_vieneu_style_invalid",
            "Choose a supported VieNeu-TTS reading style.",
        ));
    }

    Ok(LocalTranslationConfig {
        schema_version: CONFIG_SCHEMA_VERSION,
        translation_engine: draft.translation_engine,
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
        tts_provider: draft.tts_provider,
        vieneu_base_url,
        vieneu_style: vieneu_style.to_string(),
        voice_id: require_non_empty(
            &draft.voice_id,
            "local_tts_voice_missing",
            "Choose an installed local voice.",
        )?,
        tts_rate: draft.tts_rate.clamp(0.5, 2.0),
        tts_volume: draft.tts_volume.clamp(0.0, 1.0),
        tts_output_sample_rate_hz: crate::tts::LOCAL_TTS_SAMPLE_RATE,
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
    system_prompt: &str,
    temperature: f32,
    max_output_tokens: u32,
    keep_alive: Option<&str>,
) -> Value {
    let mut payload = json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": system_prompt },
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

pub fn validate_local_session_languages(
    source_language: Language,
    target_language: Language,
    tts_provider: LocalTtsProvider,
) -> AppResult<()> {
    let _ = whisper_language_code(source_language)?;
    if target_language == Language::Auto {
        return Err(AppError::new(
            "unsupported_target_language",
            "Choose an explicit target language for local translation.",
        ));
    }
    if tts_provider == LocalTtsProvider::Vieneu && target_language != Language::Vi {
        return Err(AppError::new(
            "local_tts_target_language_unsupported",
            "VieNeu-TTS supports Vietnamese output only. Choose Vietnamese or switch to a matching system voice.",
        ));
    }
    Ok(())
}

pub fn whisper_language_code(language: Language) -> AppResult<Option<&'static str>> {
    let code = match language {
        Language::Auto => return Ok(None),
        Language::Fil | Language::Tl => "tl",
        Language::Jv => "jw",
        Language::Pt | Language::PtBr | Language::PtPt => "pt",
        Language::Zh | Language::ZhHans | Language::ZhHant => "zh",
        _ => language.realtime_code(),
    };
    if get_lang_id(code).is_none() {
        return Err(AppError::new(
            "unsupported_source_language",
            format!(
                "The selected source language '{}' is not supported by Whisper.",
                language.realtime_code()
            ),
        ));
    }
    Ok(Some(code))
}

fn translation_system_prompt(source_language: Language, target_language: Language) -> String {
    let source = if source_language == Language::Auto {
        "the automatically detected source language".to_string()
    } else {
        format!("language code '{}'", source_language.realtime_code())
    };
    format!(
        "Translate from {source} to language code '{}'. Return only the translation. Preserve names, numbers, and technical terms. Do not explain.",
        target_language.realtime_code()
    )
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
    let mut config: LocalTranslationConfig = serde_json::from_str(&raw).map_err(|err| {
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
    if config.model.trim() == LEGACY_DEFAULT_OLLAMA_MODEL {
        config.model = DEFAULT_OLLAMA_MODEL.to_string();
        write_config_to_path(path, &config)?;
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
        assert_eq!(config.model, DEFAULT_OLLAMA_MODEL);
        assert_eq!(config.tts_rate, 1.0);
        assert_eq!(config.tts_volume, 1.0);
        assert_eq!(config.tts_output_sample_rate_hz, 24_000);
        assert_eq!(config.tts_provider, LocalTtsProvider::System);
        assert_eq!(config.vieneu_base_url, "http://127.0.0.1:23334");
        assert_eq!(config.vieneu_style, "tu_nhien");
    }

    #[test]
    fn whisper_download_catalog_is_multilingual_and_allowlisted() {
        let models = whisper_models();
        assert_eq!(models.len(), 3);
        assert_eq!(models.iter().filter(|model| model.recommended).count(), 1);
        assert!(models.iter().all(|model| {
            model.file_name.starts_with("ggml-")
                && model.file_name.ends_with(".bin")
                && !model.file_name.contains(".en")
        }));
        assert_eq!(
            whisper_model_spec("../../arbitrary-model")
                .unwrap_err()
                .code,
            "local_whisper_model_unknown"
        );
    }

    #[test]
    fn whisper_download_percentage_is_bounded() {
        assert_eq!(download_percent(50, Some(100)), Some(50));
        assert_eq!(download_percent(101, Some(100)), Some(100));
        assert_eq!(download_percent(50, None), None);
        assert_eq!(download_percent(50, Some(0)), None);
    }

    #[test]
    fn old_config_json_migrates_tts_defaults_without_losing_existing_values() {
        let mut value = serde_json::to_value(LocalTranslationConfig::default()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert(
            "model".to_string(),
            Value::String("existing-gemma".to_string()),
        );
        object.remove("voiceId");
        object.remove("ttsRate");
        object.remove("ttsVolume");
        object.remove("ttsOutputSampleRateHz");
        object.remove("ttsProvider");
        object.remove("vieneuBaseUrl");
        object.remove("vieneuStyle");

        let migrated: LocalTranslationConfig = serde_json::from_value(value).unwrap();

        assert_eq!(migrated.model, "existing-gemma");
        assert!(migrated.voice_id.is_empty());
        assert_eq!(migrated.tts_rate, 1.0);
        assert_eq!(migrated.tts_volume, 1.0);
        assert_eq!(migrated.tts_output_sample_rate_hz, 24_000);
        assert_eq!(migrated.tts_provider, LocalTtsProvider::System);
        assert_eq!(migrated.vieneu_base_url, "http://127.0.0.1:23334");
        assert_eq!(migrated.vieneu_style, "tu_nhien");
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
        let payload = build_ollama_payload(
            "qwen",
            "こんにちは",
            &translation_system_prompt(Language::Ja, Language::Vi),
            0.0,
            128,
            Some("10m"),
        );
        assert_eq!(payload["model"], "qwen");
        assert_eq!(payload["stream"], false);
        assert_eq!(payload["messages"][1]["content"], "こんにちは");
        assert!(payload["messages"][0]["content"]
            .as_str()
            .unwrap()
            .contains("language code 'ja' to language code 'vi'"));
        assert_eq!(payload["options"]["temperature"], 0.0);
        assert_eq!(payload["options"]["num_predict"], 128);
        assert_eq!(payload["keep_alive"], "10m");
        assert!(payload.get("max_tokens").is_none());
    }

    #[test]
    fn normalizes_whisper_language_aliases_and_auto_detection() {
        assert_eq!(whisper_language_code(Language::Auto).unwrap(), None);
        assert_eq!(whisper_language_code(Language::PtBr).unwrap(), Some("pt"));
        assert_eq!(whisper_language_code(Language::ZhHant).unwrap(), Some("zh"));
        assert_eq!(whisper_language_code(Language::Jv).unwrap(), Some("jw"));
        assert!(whisper_language_code(Language::Dz).is_err());
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
    fn migrates_the_legacy_default_ollama_model() {
        let directory = std::env::temp_dir().join(format!("baka-trans-{}", Uuid::new_v4()));
        let path = directory.join(CONFIG_FILE_NAME);
        let legacy = LocalTranslationConfig {
            model: "gemma3:4b".to_string(),
            ..LocalTranslationConfig::default()
        };
        write_config_to_path(&path, &legacy).unwrap();

        let migrated = read_config_from_path(&path).unwrap();

        assert_eq!(migrated.model, "translategemma:4b");
        let persisted: LocalTranslationConfig =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(persisted.model, "translategemma:4b");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn rejects_missing_models_and_invalid_audio_contracts() {
        let missing = normalize_and_validate(
            LocalTranslationConfigDraft {
                model: String::new(),
                ..LocalTranslationConfigDraft::default()
            },
            true,
        )
        .unwrap_err();
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
        let result = test_config(
            None,
            LocalTranslationConfigDraft {
                model: "qwen".to_string(),
                model_path: model_path.to_string_lossy().into_owned(),
                ..LocalTranslationConfigDraft::default()
            },
        )
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
        let client = OllamaClient::new(&config, Language::Ja, Language::Vi).unwrap();
        let (content, _) = client.translate("こんにちは").await.unwrap();
        assert_eq!(content, "Xin chào");
        server.join().unwrap();
    }
}
