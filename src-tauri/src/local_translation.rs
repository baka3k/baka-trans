pub mod api_key;
pub mod openai_compatible;

use crate::error::{AppError, AppResult};
use crate::local_translation::openai_compatible::normalize_openai_chat_completions_url;
use crate::models::{
    Language, LocalTranslationConfig, LocalTranslationConfigDraft, LocalTranslationEngine,
    LocalTranslationTestResult, LocalTtsProvider, TranslationEngineTestResult,
    WhisperModelDownloadProgress, WhisperModelOption,
};
use futures_util::StreamExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;
use whisper_rs::{get_lang_id, WhisperContext, WhisperContextParameters};

const CONFIG_SCHEMA_VERSION: u32 = 3;
const CONFIG_DIR_NAME: &str = "dev.baka3k.baka-trans";
const CONFIG_FILE_NAME: &str = "local-translation-config.json";
const MODEL_CACHE_DIR_NAME: &str = ".bakatrans";
const WHISPER_CACHE_SUBDIR: &str = "whisper";
const WHISPER_MODEL_SOURCE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
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

pub fn whisper_model_dir() -> AppResult<PathBuf> {
    Ok(model_cache_dir()?.join(WHISPER_CACHE_SUBDIR))
}

/// Resolve the shared user-side model cache directory used for all downloaded
/// ML artifacts (Whisper, VieNeu, future Hy-MT2). Lives outside the
/// per-app Application Support folder so models survive reinstalls.
fn model_cache_dir() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE").ok_or_else(|| {
        AppError::new(
            "model_cache_dir_error",
            "Could not resolve USERPROFILE for the model cache directory.",
        )
    })?;
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME").ok_or_else(|| {
        AppError::new(
            "model_cache_dir_error",
            "Could not resolve HOME for the model cache directory.",
        )
    })?;
    Ok(PathBuf::from(home).join(MODEL_CACHE_DIR_NAME))
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
pub enum TranslationClient {
    OpenAiCompatible(openai_compatible::OpenAiCompatibleClient),
    HyMt(HyMtClient),
}

impl std::fmt::Debug for TranslationClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAiCompatible(client) => formatter
                .debug_tuple("OpenAiCompatible")
                .field(&client.endpoint())
                .finish(),
            Self::HyMt(client) => formatter
                .debug_struct("HyMt")
                .field("source_language", &client.source_language)
                .field("target_language", &client.target_language)
                .finish(),
        }
    }
}

/// Live-session handle for the offline Hy-MT2 runtime.
#[derive(Clone)]
pub struct HyMtClient {
    session: std::sync::Arc<std::sync::Mutex<crate::hy_mt::HyMtSession>>,
    source_language: String,
    target_language: String,
}

impl HyMtClient {
    async fn start(
        app: &tauri::AppHandle,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        let source = source_language.realtime_code().to_string();
        let target = target_language.realtime_code().to_string();
        let join_result = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            move || crate::hy_mt::HyMtSession::start(&app)
        })
        .await
        .map_err(|error| {
            AppError::new(
                "local_hy_mt2_session_join_error",
                format!("Could not join the Hy-MT2 session start task: {error}"),
            )
        })?;
        let session = join_result?;
        Ok(Self {
            session: std::sync::Arc::new(std::sync::Mutex::new(session)),
            source_language: source,
            target_language: target,
        })
    }

    /// Test-only entry point that builds a [`HyMtClient`] from a synthetic
    /// `CommandSpec`, bypassing the managed-runtime manifest lookup so unit
    /// tests can drive a fake shell through `start_with_spec`+`translate`.
    #[cfg(test)]
    pub(crate) async fn start_with_spec(
        spec: crate::hy_mt::CommandSpec,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        let source = source_language.realtime_code().to_string();
        let target = target_language.realtime_code().to_string();
        let join_result = tauri::async_runtime::spawn_blocking(move || {
            crate::hy_mt::HyMtSession::start_with_spec(&spec, std::path::Path::new("."), "cpu")
        })
        .await
        .map_err(|error| {
            AppError::new(
                "local_hy_mt2_session_join_error",
                format!("Could not join the Hy-MT2 session start task: {error}"),
            )
        })?;
        let session = join_result?;
        Ok(Self {
            session: std::sync::Arc::new(std::sync::Mutex::new(session)),
            source_language: source,
            target_language: target,
        })
    }

    async fn translate(&self, source_text: &str) -> AppResult<(String, u64)> {
        let session = self.session.clone();
        let source_language = self.source_language.clone();
        let target_language = self.target_language.clone();
        let text = source_text.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            let mut guard = session.lock().map_err(|error| {
                AppError::new(
                    "local_hy_mt2_session_lock_error",
                    format!("The Hy-MT2 session lock was poisoned: {error}"),
                )
            })?;
            guard.translate(&source_language, &target_language, &text)
        })
        .await
        .map_err(|error| {
            AppError::new(
                "local_hy_mt2_session_join_error",
                format!("Could not join the Hy-MT2 translate task: {error}"),
            )
        })?
    }
}

impl TranslationClient {
    pub async fn new(
        app: &tauri::AppHandle,
        config: &LocalTranslationConfig,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        match config.translation_engine {
            LocalTranslationEngine::HuggingfaceOffline => {
                HyMtClient::start(app, source_language, target_language)
                    .await
                    .map(Self::HyMt)
            }
            LocalTranslationEngine::OpenaiCompatible => {
                let api_key_info = api_key::load_local_translation_api_key()?;
                Ok(Self::OpenAiCompatible(
                    openai_compatible::OpenAiCompatibleClient::new(
                        &config.openai_base_url,
                        &config.openai_model,
                        config.openai_timeout_seconds,
                        config.openai_temperature,
                        config.openai_max_output_tokens,
                        api_key_info.map(|info| info.key),
                        source_language,
                        target_language,
                    )?,
                ))
            }
        }
    }

    pub async fn translate(&self, source_text: &str) -> AppResult<(String, u64)> {
        match self {
            Self::OpenAiCompatible(client) => client.translate(source_text).await,
            Self::HyMt(client) => client.translate(source_text).await,
        }
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
    let engine = draft.translation_engine;
    let fallback_model = effective_model(&draft);
    let fallback_endpoint = effective_endpoint(&draft);
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
            effective_model_from_config(&config),
            effective_endpoint_from_config(&config),
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
            effective_model_from_config(&config),
            effective_endpoint_from_config(&config),
            error,
            LocalTestHealth {
                whisper_model_readable: true,
                tts_voice_available: true,
                ..LocalTestHealth::default()
            },
        ));
    }

    match engine {
        LocalTranslationEngine::HuggingfaceOffline => {
            let Some(handle) = app else {
                return Ok(failed_test_result(
                    crate::hy_mt::HY_MT_MODEL_ID.to_string(),
                    "managed offline runtime".to_string(),
                    AppError::new(
                        "local_hy_mt2_no_app_handle",
                        "Cannot probe the offline Hy-MT2 engine without an app handle.",
                    ),
                    LocalTestHealth {
                        whisper_model_readable: true,
                        whisper_model_loaded: true,
                        tts_voice_available: true,
                        ..LocalTestHealth::default()
                    },
                ));
            };
            let probe = crate::hy_mt::probe_translation_engine(handle).await;
            if let Some(error) = probe.error {
                return Ok(failed_test_result(
                    crate::hy_mt::HY_MT_MODEL_ID.to_string(),
                    "managed offline runtime".to_string(),
                    error,
                    LocalTestHealth {
                        whisper_model_readable: true,
                        whisper_model_loaded: true,
                        engine_reachable: probe.reachable,
                        engine_accepted: probe.accepted,
                        tts_voice_available: true,
                    },
                ));
            }
            Ok(LocalTranslationTestResult {
                ok: true,
                message: format!(
                    "Whisper, the offline Hy-MT2 engine, and the selected voice are ready. Probe translation: {}",
                    truncate_probe_message(probe.translated_text.as_deref().unwrap_or_default())
                ),
                model: crate::hy_mt::HY_MT_MODEL_ID.to_string(),
                endpoint: "managed offline runtime".to_string(),
                whisper_model_readable: true,
                whisper_model_loaded: true,
                engine_reachable: true,
                engine_accepted: true,
                tts_voice_available: true,
            })
        }
        LocalTranslationEngine::OpenaiCompatible => test_openai_compatible_engine(&config).await,
    }
}

fn effective_model(draft: &LocalTranslationConfigDraft) -> String {
    draft.openai_model.trim().to_string()
}

fn effective_endpoint(draft: &LocalTranslationConfigDraft) -> String {
    let url = draft.openai_base_url.trim();
    if !url.is_empty() {
        return openai_compatible::normalize_openai_chat_completions_url(url)
            .unwrap_or_else(|_| url.to_string());
    }
    String::new()
}

fn effective_model_from_config(config: &LocalTranslationConfig) -> String {
    config.openai_model.clone()
}

fn effective_endpoint_from_config(config: &LocalTranslationConfig) -> String {
    if !config.openai_base_url.is_empty() {
        return openai_compatible::normalize_openai_chat_completions_url(&config.openai_base_url)
            .unwrap_or_else(|_| config.openai_base_url.clone());
    }
    String::new()
}

async fn test_openai_compatible_engine(
    config: &LocalTranslationConfig,
) -> AppResult<LocalTranslationTestResult> {
    let probe = probe_openai_engine(
        &config.openai_base_url,
        &config.openai_model,
        config.openai_timeout_seconds,
        config.openai_temperature,
        config.openai_max_output_tokens,
    )
    .await;
    if let Some(error) = probe.error {
        return Ok(failed_test_result(
            config.openai_model.clone(),
            probe.endpoint,
            error,
            LocalTestHealth {
                whisper_model_readable: true,
                whisper_model_loaded: true,
                engine_reachable: probe.reachable,
                tts_voice_available: true,
                ..LocalTestHealth::default()
            },
        ));
    }
    Ok(LocalTranslationTestResult {
        ok: true,
        message: format!(
            "Whisper, the OpenAI-compatible endpoint, and the selected voice are ready. Probe translation: {} characters.",
            probe.probe_characters
        ),
        model: config.openai_model.clone(),
        endpoint: probe.endpoint,
        whisper_model_readable: true,
        whisper_model_loaded: true,
        engine_reachable: true,
        engine_accepted: true,
        tts_voice_available: true,
    })
}

pub async fn test_engine(
    app: &tauri::AppHandle,
    draft: LocalTranslationConfigDraft,
) -> AppResult<TranslationEngineTestResult> {
    match draft.translation_engine {
        LocalTranslationEngine::HuggingfaceOffline => {
            let probe = crate::hy_mt::probe_translation_engine(app).await;
            Ok(offline_engine_test_result(probe))
        }
        LocalTranslationEngine::OpenaiCompatible => test_engine_openai_compatible(draft).await,
    }
}

fn offline_engine_test_result(probe: crate::hy_mt::HyMtEngineProbe) -> TranslationEngineTestResult {
    let message = match &probe.error {
        Some(error) => error.message.clone(),
        None => format!(
            "The offline Hy-MT2 engine loaded in {:.0} ms on {} and translated the probe in {:.0} ms: {}",
            probe.load_ms,
            probe.device,
            probe.latency_ms.unwrap_or(0.0),
            truncate_probe_message(probe.translated_text.as_deref().unwrap_or_default())
        ),
    };
    TranslationEngineTestResult {
        engine: "huggingface_offline".to_string(),
        model: crate::hy_mt::HY_MT_MODEL_ID.to_string(),
        endpoint: "managed offline runtime".to_string(),
        reachable: probe.reachable,
        accepted: probe.accepted,
        message,
    }
}

fn truncate_probe_message(text: &str) -> String {
    const MAX_PROBE_MESSAGE_CHARS: usize = 400;
    if text.chars().count() <= MAX_PROBE_MESSAGE_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_PROBE_MESSAGE_CHARS).collect();
    format!("{truncated}…")
}

async fn test_engine_openai_compatible(
    draft: LocalTranslationConfigDraft,
) -> AppResult<TranslationEngineTestResult> {
    let (endpoint, model) = validate_openai_compatible_fields(&draft)?;
    let probe = probe_openai_engine(
        &endpoint,
        &model,
        draft.openai_timeout_seconds.clamp(5, 300),
        draft.openai_temperature.clamp(0.0, 2.0),
        draft.openai_max_output_tokens.clamp(32, 16_384),
    )
    .await;
    Ok(TranslationEngineTestResult {
        engine: "openai_compatible".to_string(),
        model,
        endpoint: probe.endpoint,
        reachable: probe.reachable,
        accepted: probe.accepted,
        message: match probe.error {
            Some(error) => error.message,
            None => format!(
                "The endpoint accepted a probe translation ({} characters).",
                probe.probe_characters
            ),
        },
    })
}

struct EngineProbe {
    endpoint: String,
    error: Option<AppError>,
    reachable: bool,
    accepted: bool,
    probe_characters: usize,
}

async fn probe_openai_engine(
    endpoint: &str,
    model: &str,
    timeout_seconds: u64,
    temperature: f32,
    max_output_tokens: u32,
) -> EngineProbe {
    let api_key_info = match api_key::load_local_translation_api_key() {
        Ok(info) => info,
        Err(error) => {
            return EngineProbe {
                endpoint: endpoint.to_string(),
                error: Some(error),
                reachable: false,
                accepted: false,
                probe_characters: 0,
            };
        }
    };
    let client = match openai_compatible::OpenAiCompatibleClient::new(
        endpoint,
        model,
        timeout_seconds,
        temperature,
        max_output_tokens,
        api_key_info.map(|info| info.key),
        Language::Ja,
        Language::Vi,
    ) {
        Ok(client) => client,
        Err(error) => {
            return EngineProbe {
                endpoint: endpoint.to_string(),
                error: Some(error),
                reachable: false,
                accepted: false,
                probe_characters: 0,
            };
        }
    };
    let endpoint = client.endpoint().to_string();
    match client.translate("こんにちは").await {
        Ok((probe, _)) => EngineProbe {
            endpoint,
            error: None,
            reachable: true,
            accepted: true,
            probe_characters: probe.chars().count(),
        },
        Err(error) => {
            let reachable = error.code != "local_openai_request_error";
            EngineProbe {
                endpoint,
                error: Some(error),
                reachable,
                accepted: false,
                probe_characters: 0,
            }
        }
    }
}

#[derive(Default)]
struct LocalTestHealth {
    whisper_model_readable: bool,
    whisper_model_loaded: bool,
    engine_reachable: bool,
    engine_accepted: bool,
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
        engine_reachable: health.engine_reachable,
        engine_accepted: health.engine_accepted,
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
    let (openai_endpoint, openai_model) = validate_openai_compatible_fields(&draft)?;
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
        openai_base_url: openai_endpoint,
        openai_model,
        openai_timeout_seconds: draft.openai_timeout_seconds.clamp(5, 300),
        openai_temperature: draft.openai_temperature.clamp(0.0, 2.0),
        openai_max_output_tokens: draft.openai_max_output_tokens.clamp(32, 16_384),
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

fn validate_openai_compatible_fields(
    draft: &LocalTranslationConfigDraft,
) -> AppResult<(String, String)> {
    let base_url = draft.openai_base_url.trim();
    let model = draft.openai_model.trim();
    if draft.translation_engine != LocalTranslationEngine::OpenaiCompatible {
        return Ok((base_url.to_string(), model.to_string()));
    }
    if base_url.is_empty() {
        return Err(AppError::new(
            "local_openai_base_url_missing",
            "Enter an OpenAI-compatible base URL.",
        ));
    }
    if model.is_empty() {
        return Err(AppError::new(
            "local_openai_model_missing",
            "Enter an OpenAI-compatible model name.",
        ));
    }
    let endpoint = normalize_openai_chat_completions_url(base_url)?;
    Ok((endpoint, model.to_string()))
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

fn read_config_from_path(path: &Path) -> AppResult<LocalTranslationConfig> {
    if !path.exists() {
        return Ok(LocalTranslationConfig::default());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|err| AppError::new("local_config_read_error", err.to_string()))?;

    let legacy_schema = detect_legacy_schema_version(&raw);
    if let Some(version) = legacy_schema {
        if version < CONFIG_SCHEMA_VERSION {
            return migrate_legacy_config(path, &raw, version);
        }
    }

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

fn detect_legacy_schema_version(raw: &str) -> Option<u32> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    value
        .get("schemaVersion")
        .or_else(|| value.get("schema_version"))
        .and_then(serde_json::Value::as_u64)
        .map(|v| v as u32)
}

fn migrate_legacy_config(
    path: &Path,
    raw: &str,
    from_version: u32,
) -> AppResult<LocalTranslationConfig> {
    let backup_path = path.with_extension("json.legacy-backup");
    if !backup_path.exists() {
        std::fs::write(&backup_path, raw).map_err(|err| {
            AppError::new(
                "local_config_write_error",
                format!("Could not create legacy backup: {err}"),
            )
        })?;
    }

    let legacy: serde_json::Value = serde_json::from_str(raw).map_err(|err| {
        AppError::new(
            "local_config_parse_error",
            format!("Could not parse legacy config: {err}"),
        )
    })?;

    let mut config = LocalTranslationConfig::default();
    config.schema_version = CONFIG_SCHEMA_VERSION;
    config.translation_engine = LocalTranslationEngine::HuggingfaceOffline;

    if let Some(model_path) = legacy.get("modelPath").and_then(|v| v.as_str()) {
        config.model_path = model_path.to_string();
    }
    if let Some(language) = legacy.get("language").and_then(|v| v.as_str()) {
        config.language = language.to_string();
    }
    if let Some(threads) = legacy.get("threads").and_then(|v| v.as_u64()) {
        config.threads = threads as u32;
    }
    if let Some(use_gpu) = legacy.get("useGpu").and_then(|v| v.as_bool()) {
        config.use_gpu = use_gpu;
    }
    if let Some(rate) = legacy.get("sampleRateHz").and_then(|v| v.as_u64()) {
        config.sample_rate_hz = rate as u32;
    }
    if let Some(v) = legacy.get("minimumSpeechMs").and_then(|v| v.as_u64()) {
        config.minimum_speech_ms = v;
    }
    if let Some(v) = legacy.get("silenceToCommitMs").and_then(|v| v.as_u64()) {
        config.silence_to_commit_ms = v;
    }
    if let Some(v) = legacy.get("maximumUtteranceMs").and_then(|v| v.as_u64()) {
        config.maximum_utterance_ms = v;
    }
    if let Some(v) = legacy.get("preRollMs").and_then(|v| v.as_u64()) {
        config.pre_roll_ms = v;
    }
    if let Some(v) = legacy.get("speechThreshold").and_then(|v| v.as_f64()) {
        config.speech_threshold = v as f32;
    }
    if let Some(v) = legacy.get("voiceId").and_then(|v| v.as_str()) {
        config.voice_id = v.to_string();
    }
    if let Some(v) = legacy.get("ttsRate").and_then(|v| v.as_f64()) {
        config.tts_rate = v as f32;
    }
    if let Some(v) = legacy.get("ttsVolume").and_then(|v| v.as_f64()) {
        config.tts_volume = v as f32;
    }
    if let Some(v) = legacy.get("ttsOutputSampleRateHz").and_then(|v| v.as_u64()) {
        config.tts_output_sample_rate_hz = v as u32;
    }
    if let Some(v) = legacy.get("vieneuStyle").and_then(|v| v.as_str()) {
        config.vieneu_style = v.to_string();
    }

    if from_version < 2 {
        if let Some(engine) = legacy.get("translationEngine").and_then(|v| v.as_str()) {
            match engine {
                "ollama" | "local" => {
                    config.translation_engine = LocalTranslationEngine::HuggingfaceOffline
                }
                _ => {}
            }
        }
    }

    write_config_to_path(path, &config)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn defaults_use_fixed_japanese_pcm_contract() {
        let config = LocalTranslationConfig::default();
        assert_eq!(config.language, "ja");
        assert_eq!(config.sample_rate_hz, 16_000);
        assert_eq!(config.openai_temperature, 0.0);
        assert_eq!(
            config.translation_engine,
            LocalTranslationEngine::HuggingfaceOffline
        );
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
        object.remove("voiceId");
        object.remove("ttsRate");
        object.remove("ttsVolume");
        object.remove("ttsOutputSampleRateHz");
        object.remove("ttsProvider");
        object.remove("vieneuBaseUrl");
        object.remove("vieneuStyle");

        let migrated: LocalTranslationConfig = serde_json::from_value(value).unwrap();

        assert!(migrated.voice_id.is_empty());
        assert_eq!(migrated.tts_rate, 1.0);
        assert_eq!(migrated.tts_volume, 1.0);
        assert_eq!(migrated.tts_output_sample_rate_hz, 24_000);
        assert_eq!(migrated.tts_provider, LocalTtsProvider::System);
        assert_eq!(migrated.vieneu_base_url, "http://127.0.0.1:23334");
        assert_eq!(migrated.vieneu_style, "tu_nhien");
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
    fn persists_versioned_config_round_trip() {
        let directory = std::env::temp_dir().join(format!("baka-trans-{}", Uuid::new_v4()));
        let path = directory.join(CONFIG_FILE_NAME);
        let config = LocalTranslationConfig::default();
        write_config_to_path(&path, &config).unwrap();
        let loaded = read_config_from_path(&path).unwrap();

        assert_eq!(loaded.schema_version, config.schema_version);
        assert_eq!(loaded.translation_engine, config.translation_engine);
        assert_eq!(loaded.openai_base_url, config.openai_base_url);
        assert_eq!(loaded.openai_model, config.openai_model);
        assert_eq!(loaded.openai_timeout_seconds, config.openai_timeout_seconds);
        assert_eq!(loaded.model_path, config.model_path);
        assert_eq!(loaded.voice_id, config.voice_id);
        assert_eq!(loaded.tts_rate, config.tts_rate);
        assert_eq!(loaded.tts_volume, config.tts_volume);

        let raw = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(
            json.get("baseUrl").is_none(),
            "Legacy base_url must not be persisted"
        );
        assert!(
            json.get("model").is_none(),
            "Legacy model must not be persisted"
        );
        assert!(
            json.get("keepAlive").is_none(),
            "Legacy keep_alive must not be persisted"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn migrates_legacy_v2_config_with_atomic_backup() {
        let directory = std::env::temp_dir().join(format!("baka-trans-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(CONFIG_FILE_NAME);
        let backup = path.with_extension("json.legacy-backup");

        let legacy_json = serde_json::json!({
            "schemaVersion": 2,
            "baseUrl": "http://localhost:11434",
            "model": "gemma3:4b",
            "timeoutSeconds": 30,
            "temperature": 0.0,
            "maxOutputTokens": 256,
            "keepAlive": "10m",
            "modelPath": "/some/whisper-model.bin",
            "language": "ja",
            "threads": 4,
            "useGpu": false,
            "sampleRateHz": 16000,
            "minimumSpeechMs": 300,
            "silenceToCommitMs": 700,
            "maximumUtteranceMs": 15000,
            "preRollMs": 250,
            "speechThreshold": 0.015,
            "voiceId": "test-voice",
            "ttsRate": 1.0,
            "ttsVolume": 1.0,
            "ttsOutputSampleRateHz": 24000,
            "vieneuStyle": "tu_nhien"
        });
        std::fs::write(&path, serde_json::to_string_pretty(&legacy_json).unwrap()).unwrap();

        let migrated = read_config_from_path(&path).unwrap();

        assert_eq!(migrated.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(
            migrated.translation_engine,
            LocalTranslationEngine::HuggingfaceOffline
        );
        assert_eq!(migrated.model_path, "/some/whisper-model.bin");
        assert_eq!(migrated.voice_id, "test-voice");

        assert!(backup.exists(), "Legacy backup should exist");
        let backup_content = std::fs::read_to_string(&backup).unwrap();
        assert!(backup_content.contains("\"schemaVersion\": 2"));
        assert!(backup_content.contains("gemma3:4b"));

        let persisted_raw = std::fs::read_to_string(&path).unwrap();
        let persisted: serde_json::Value = serde_json::from_str(&persisted_raw).unwrap();
        assert!(
            persisted.get("model").is_none(),
            "v3 JSON must not contain legacy model field"
        );
        assert!(
            persisted.get("baseUrl").is_none(),
            "v3 JSON must not contain legacy base_url field"
        );
        assert!(
            persisted.get("keepAlive").is_none(),
            "v3 JSON must not contain legacy keep_alive field"
        );
        assert_eq!(persisted["schemaVersion"], CONFIG_SCHEMA_VERSION);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn migrates_v1_config_to_huggingface_offline() {
        let directory = std::env::temp_dir().join(format!("baka-trans-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(CONFIG_FILE_NAME);

        let v1_json = serde_json::json!({
            "schemaVersion": 1,
            "baseUrl": "http://localhost:11434",
            "model": "translategemma:4b",
            "modelPath": "/model.bin",
            "language": "ja",
            "voiceId": "v1-voice"
        });
        std::fs::write(&path, serde_json::to_string_pretty(&v1_json).unwrap()).unwrap();

        let migrated = read_config_from_path(&path).unwrap();

        assert_eq!(migrated.schema_version, CONFIG_SCHEMA_VERSION);
        assert_eq!(
            migrated.translation_engine,
            LocalTranslationEngine::HuggingfaceOffline
        );
        assert_eq!(migrated.voice_id, "v1-voice");

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn rejects_invalid_audio_contracts_and_missing_openai_fields() {
        let openai_missing_url = normalize_and_validate(
            LocalTranslationConfigDraft {
                translation_engine: LocalTranslationEngine::OpenaiCompatible,
                openai_base_url: String::new(),
                openai_model: "test-model".to_string(),
                ..LocalTranslationConfigDraft::default()
            },
            false,
        )
        .unwrap_err();
        assert_eq!(openai_missing_url.code, "local_openai_base_url_missing");

        let openai_missing_model = normalize_and_validate(
            LocalTranslationConfigDraft {
                translation_engine: LocalTranslationEngine::OpenaiCompatible,
                openai_base_url: "https://api.example.com".to_string(),
                openai_model: String::new(),
                ..LocalTranslationConfigDraft::default()
            },
            false,
        )
        .unwrap_err();
        assert_eq!(openai_missing_model.code, "local_openai_model_missing");

        let mut draft = LocalTranslationConfigDraft {
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
                model_path: model_path.to_string_lossy().into_owned(),
                ..LocalTranslationConfigDraft::default()
            },
        )
        .await
        .unwrap();

        assert!(!result.ok);
        assert!(result.whisper_model_readable);
        assert!(!result.whisper_model_loaded);
        assert!(!result.engine_reachable);
        let _ = std::fs::remove_file(model_path);
    }

    #[tokio::test]
    async fn translation_client_dispatches_openai_compatible() {
        let api_key_info: Option<crate::local_translation::api_key::ApiKeyInfo> = None;
        let _ = api_key_info; // explicit silence
        let inner = openai_compatible::OpenAiCompatibleClient::new(
            "http://localhost:8080/v1",
            "gemma-3-4b-it",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        )
        .expect("OpenAI-compatible client should construct with a valid base URL");

        let client = TranslationClient::OpenAiCompatible(inner);
        match &client {
            TranslationClient::OpenAiCompatible(inner) => {
                assert_eq!(
                    inner.endpoint(),
                    "http://localhost:8080/v1/chat/completions"
                );
            }
            other => panic!("expected OpenAiCompatible variant, got {:?}", other),
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn translation_client_dispatches_huggingface_offline() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!("baka-trans-hy-mt-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("fake-hy-mt.sh");
        std::fs::write(
            &script,
            concat!(
                "#!/bin/sh\n",
                "printf '%s\\n' '{\"type\":\"ready\",\"protocolVersion\":1,\"runtimeVersion\":\"0.2.0\",\"modelId\":\"tencent/Hy-MT2-1.8B\",\"revision\":\"9a341cd1b679d3efd23b46e847b01745a71ed792\",\"trustRemoteCode\":false,\"device\":\"cpu\",\"dtype\":\"float32\",\"pid\":1,\"loadMs\":25.5}'\n",
                "IFS= read -r line\n",
                "id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\"\\([^\"]*\\)\".*/\\1/p')\n",
                "printf '{\"type\":\"result\",\"id\":\"%s\",\"text\":\"Xin chào\",\"inputTokens\":3,\"outputTokens\":3,\"latencyMs\":4.5}\\n' \"$id\"\n",
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let spec = crate::hy_mt::CommandSpec {
            program: script.clone(),
            prefix_args: Vec::new(),
            working_dir: directory.clone(),
        };
        let client = HyMtClient::start_with_spec(spec, Language::Ja, Language::Vi)
            .await
            .unwrap();
        let (text, _latency) = client.translate("こんにちは").await.unwrap();
        assert_eq!(text, "Xin chào");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn translation_client_rejects_missing_openai_base_url() {
        let result = openai_compatible::OpenAiCompatibleClient::new(
            "",
            "gemma-3-4b-it",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        );
        assert_eq!(result.unwrap_err().code, "local_openai_base_url_missing");
    }

    #[test]
    fn offline_engine_test_result_reports_probe_translation() {
        let result = offline_engine_test_result(crate::hy_mt::HyMtEngineProbe {
            device: "mps".to_string(),
            load_ms: 1234.5,
            translated_text: Some("Xin chào".to_string()),
            latency_ms: Some(56.5),
            reachable: true,
            accepted: true,
            error: None,
        });

        assert_eq!(result.engine, "huggingface_offline");
        assert_eq!(result.model, "tencent/Hy-MT2-1.8B");
        assert_eq!(result.endpoint, "managed offline runtime");
        assert!(result.reachable);
        assert!(result.accepted);
        assert!(result.message.contains("Xin chào"));
    }

    #[test]
    fn offline_engine_test_result_surfaces_probe_errors() {
        let result = offline_engine_test_result(crate::hy_mt::HyMtEngineProbe {
            device: String::new(),
            load_ms: 0.0,
            translated_text: None,
            latency_ms: None,
            reachable: false,
            accepted: false,
            error: Some(AppError::new(
                "hy_mt_model_missing",
                "The Hy-MT2 model is not installed yet.",
            )),
        });

        assert!(!result.reachable);
        assert!(!result.accepted);
        assert!(result.message.contains("not installed"));
    }

    #[test]
    fn probe_message_truncates_long_translations() {
        let long = "あ".repeat(500);
        assert_eq!(truncate_probe_message(&long).chars().count(), 401);
        assert_eq!(truncate_probe_message("short"), "short");
    }

    #[tokio::test]
    async fn test_engine_validates_openai_fields_before_probing() {
        let result = test_engine_openai_compatible(LocalTranslationConfigDraft {
            translation_engine: LocalTranslationEngine::OpenaiCompatible,
            openai_base_url: String::new(),
            openai_model: String::new(),
            ..LocalTranslationConfigDraft::default()
        })
        .await;

        assert_eq!(result.unwrap_err().code, "local_openai_base_url_missing");
    }
}
