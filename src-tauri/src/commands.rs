use crate::audio;
use crate::error::{AppError, AppResult};
use crate::models::{
    ApiKeyTestResult, AppStatus, AudioDevices, AudioOutputChannel, ExportRequest,
    ExportedTranscript, HyMtModelStatus, LlmProviderProfile, LlmProviderProfileDraft,
    LlmProviderTestResult, LocalTranslationConfig, LocalTranslationConfigDraft,
    LocalTranslationTestResult, LocalTtsProvider, LocalVoice, LookHelpConfig, LookHelpStatus,
    ManualBoundaryRequest, MeetingSummaryConfig, MeetingSummaryResult, MeetingSummaryStatus,
    MeetingSummaryStatusEvent, OverlayConfig, OverlayGeometry, OverlayStatus, SessionConfig,
    TranscriptItem, TranslationCredentialStatus, TranslationEngineTestResult,
    TranslationProvider, VieNeuRuntimeStatus, WhisperModelOption,
};
use crate::session::AppState;
use crate::{ai, hy_mt, llm, local_translation, look_help, overlay, security, summary_agent, tts, vieneu};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, State};

async fn run_blocking<T>(operation: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T>
where
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| AppError::new("background_task_error", error.to_string()))?
}

#[tauri::command]
pub async fn get_app_status(app: AppHandle) -> AppResult<AppStatus> {
    run_blocking(move || app.state::<AppState>().app_status()).await
}

#[tauri::command]
pub fn get_transcript_snapshot(state: State<'_, AppState>) -> AppResult<Vec<TranscriptItem>> {
    state.transcript_snapshot()
}

#[tauri::command]
pub async fn list_audio_devices() -> AppResult<AudioDevices> {
    run_blocking(audio::list_devices).await
}

#[tauri::command]
pub async fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    config: SessionConfig,
) -> AppResult<()> {
    state.start_session(app, config).await
}

#[tauri::command]
pub async fn pause_session(app: AppHandle) -> AppResult<()> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        state.pause_session(app.clone())
    })
    .await
}

#[tauri::command]
pub async fn resume_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.resume_session(app).await
}

#[tauri::command]
pub async fn stop_session(app: AppHandle) -> AppResult<()> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        state.stop_session(app.clone())
    })
    .await
}

#[tauri::command]
pub fn force_translate_boundary(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ManualBoundaryRequest,
) -> AppResult<()> {
    state.force_translate_boundary(app, request)
}

#[tauri::command]
pub async fn save_api_key(api_key: String) -> AppResult<()> {
    run_blocking(move || security::save_api_key(&api_key)).await
}

#[tauri::command]
pub async fn save_translation_api_key(
    provider: TranslationProvider,
    api_key: String,
) -> AppResult<()> {
    run_blocking(move || security::save_translation_api_key(provider, &api_key)).await
}

#[tauri::command]
pub async fn has_api_key() -> AppResult<bool> {
    run_blocking(|| Ok(security::has_api_key())).await
}

#[tauri::command]
pub async fn has_translation_api_key(provider: TranslationProvider) -> AppResult<bool> {
    run_blocking(move || Ok(security::has_translation_api_key(provider))).await
}

#[tauri::command]
pub async fn translation_credential_status(
    provider: TranslationProvider,
) -> AppResult<TranslationCredentialStatus> {
    run_blocking(move || Ok(security::translation_credential_status(provider))).await
}

#[tauri::command]
pub async fn test_api_key() -> AppResult<ApiKeyTestResult> {
    test_translation_api_key(TranslationProvider::OpenaiRealtime).await
}

#[tauri::command]
pub async fn test_translation_api_key(
    provider: TranslationProvider,
) -> AppResult<ApiKeyTestResult> {
    let info = security::load_translation_api_key_info(provider)?;
    let test_result = match provider {
        TranslationProvider::OpenaiRealtime => ai::test_realtime_connection(&info.key).await,
        TranslationProvider::GoogleLiveTranslate => {
            ai::test_google_live_translation_connection(&info.key).await
        }
        TranslationProvider::LocalWhisper => {
            return Err(AppError::new(
                "local_provider_has_no_api_key",
                "Use Test local pipeline in Local LLM settings.",
            ));
        }
    };
    if let Err(error) = test_result {
        return Err(AppError::new(
            error.code,
            format!(
                "{} test failed for {} key {}: {}",
                provider.label(),
                api_key_source_label(info.source),
                info.fingerprint,
                error.message
            ),
        ));
    }
    Ok(ApiKeyTestResult {
        provider,
        source: info.source,
        fingerprint: info.fingerprint,
        message: format!("{} accepted this key.", provider.label()),
    })
}

#[tauri::command]
pub async fn get_local_translation_config() -> AppResult<LocalTranslationConfig> {
    run_blocking(local_translation::get_config).await
}

#[tauri::command]
pub async fn save_local_translation_config(
    draft: LocalTranslationConfigDraft,
) -> AppResult<LocalTranslationConfig> {
    run_blocking(move || local_translation::save_config(draft)).await
}

#[tauri::command]
pub async fn test_local_translation_config(
    app: AppHandle,
    draft: LocalTranslationConfigDraft,
) -> AppResult<LocalTranslationTestResult> {
    local_translation::test_config(Some(&app), draft).await
}

#[tauri::command]
pub async fn test_translation_engine(
    draft: LocalTranslationConfigDraft,
) -> AppResult<TranslationEngineTestResult> {
    local_translation::test_engine(draft).await
}

#[tauri::command]
pub async fn list_whisper_models() -> AppResult<Vec<WhisperModelOption>> {
    run_blocking(|| Ok(local_translation::whisper_models())).await
}

#[tauri::command]
pub async fn download_whisper_model(app: AppHandle, model_id: String) -> AppResult<String> {
    local_translation::download_whisper_model(&app, model_id.trim()).await
}

#[tauri::command]
pub async fn get_whisper_model_dir() -> AppResult<String> {
    run_blocking(|| {
        Ok(local_translation::whisper_model_dir()?.to_string_lossy().into_owned())
    })
    .await
}

#[tauri::command]
pub async fn get_vieneu_runtime_status(
    app: AppHandle,
    manager: State<'_, vieneu::VieNeuManager>,
) -> AppResult<VieNeuRuntimeStatus> {
    manager.status(&app).await
}

#[tauri::command]
pub async fn get_hy_mt_model_status(
    app: AppHandle,
    manager: State<'_, hy_mt::HyMtManager>,
) -> AppResult<HyMtModelStatus> {
    manager.status(&app).await
}

#[tauri::command]
pub async fn install_hy_mt_model(
    app: AppHandle,
    manager: State<'_, hy_mt::HyMtManager>,
) -> AppResult<HyMtModelStatus> {
    manager.install(app).await
}

#[tauri::command]
pub fn cancel_hy_mt_model_install(manager: State<'_, hy_mt::HyMtManager>) {
    manager.cancel_install();
}

#[tauri::command]
pub async fn install_vieneu_runtime(
    app: AppHandle,
    manager: State<'_, vieneu::VieNeuManager>,
) -> AppResult<VieNeuRuntimeStatus> {
    manager.install(app).await
}

#[tauri::command]
pub fn cancel_vieneu_runtime_install(manager: State<'_, vieneu::VieNeuManager>) {
    manager.cancel_install();
}

#[tauri::command]
pub async fn restart_vieneu_runtime(
    app: AppHandle,
    manager: State<'_, vieneu::VieNeuManager>,
) -> AppResult<VieNeuRuntimeStatus> {
    manager.restart(&app).await
}

#[tauri::command]
pub async fn list_local_tts_voices(
    app: AppHandle,
    provider: LocalTtsProvider,
) -> AppResult<Vec<LocalVoice>> {
    tts::list_voices(Some(&app), provider).await
}

#[tauri::command]
pub async fn preview_local_tts(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: LocalTranslationConfigDraft,
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    if state.app_status()?.session_status != crate::models::SessionStatus::Idle {
        return Err(AppError::new(
            "session_busy",
            "Stop translation before testing the local voice.",
        ));
    }
    if output_device_id.trim().is_empty() {
        return Err(AppError::new(
            "local_tts_output_missing",
            "Choose a translated audio output before testing the voice.",
        ));
    }
    let config = local_translation::normalize_and_validate(draft, true)?;
    let audio = tts::synthesize(
        Some(&app),
        "Xin chào. Đây là giọng dịch cục bộ.",
        &config,
        Arc::new(AtomicBool::new(false)),
    )
    .await?;
    let playback = audio::start_playback_with_channel_at_sample_rate(
        app,
        &output_device_id,
        output_channel,
        audio.sample_rate_hz,
    )?;
    playback
        .sender()
        .send(audio.pcm16_mono.clone())
        .map_err(|_| {
            AppError::new(
                "local_tts_playback_error",
                "The selected output stopped before the voice preview could play.",
            )
        })?;
    let duration_ms = (audio.pcm16_mono.len() as u64 * 1_000)
        .saturating_div(u64::from(audio.sample_rate_hz))
        .saturating_add(250);
    tokio::time::sleep(Duration::from_millis(duration_ms)).await;
    drop(playback);
    Ok(())
}

#[tauri::command]
pub async fn list_llm_profiles() -> AppResult<Vec<LlmProviderProfile>> {
    run_blocking(llm::list_profiles).await
}

#[tauri::command]
pub async fn save_llm_profile(draft: LlmProviderProfileDraft) -> AppResult<LlmProviderProfile> {
    run_blocking(move || llm::save_profile(draft)).await
}

#[tauri::command]
pub async fn delete_llm_profile(profile_id: String) -> AppResult<()> {
    run_blocking(move || llm::delete_profile(&profile_id)).await
}

#[tauri::command]
pub async fn test_llm_profile(profile_id: String) -> AppResult<LlmProviderTestResult> {
    llm::test_profile(&profile_id).await
}

#[tauri::command]
pub async fn run_meeting_summary_agent(
    app: AppHandle,
    state: State<'_, AppState>,
    config: MeetingSummaryConfig,
) -> AppResult<MeetingSummaryResult> {
    let transcript = state.transcript_snapshot()?;
    let result = summary_agent::run_meeting_summary_agent(app.clone(), transcript, config).await;
    if let Err(error) = &result {
        let _ = app.emit(
            "summary-agent-status",
            MeetingSummaryStatusEvent {
                status: MeetingSummaryStatus::Error,
                message: error.message.clone(),
            },
        );
        let _ = app.emit("app-error", error.clone());
    }
    result
}

fn api_key_source_label(source: crate::models::ApiKeySource) -> &'static str {
    match source {
        crate::models::ApiKeySource::Environment => "environment",
        crate::models::ApiKeySource::Keychain => {
            if cfg!(target_os = "windows") {
                "Windows Credential Manager"
            } else {
                "Keychain"
            }
        }
        crate::models::ApiKeySource::Memory => "memory",
    }
}

#[tauri::command]
pub fn export_transcript(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> AppResult<ExportedTranscript> {
    state.export_transcript(request)
}

#[tauri::command]
pub async fn play_test_tone(
    app: AppHandle,
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    run_blocking(move || {
        app.state::<AppState>()
            .start_test_tone(&output_device_id, output_channel)
    })
    .await
}

#[tauri::command]
pub async fn stop_test_tone(app: AppHandle) -> AppResult<()> {
    run_blocking(move || app.state::<AppState>().stop_test_tone()).await
}

#[tauri::command]
pub async fn start_local_monitor(
    app: AppHandle,
    input_device_id: String,
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    run_blocking(move || {
        let state = app.state::<AppState>();
        state.start_local_monitor(
            app.clone(),
            &input_device_id,
            &output_device_id,
            output_channel,
        )
    })
    .await
}

#[tauri::command]
pub async fn stop_local_monitor(app: AppHandle) -> AppResult<()> {
    run_blocking(move || app.state::<AppState>().stop_local_monitor()).await
}

#[tauri::command]
pub fn open_overlay_window(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
    config: OverlayConfig,
) -> AppResult<()> {
    state.open_overlay_window(app, config)
}

#[tauri::command]
pub fn open_look_help_window(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
    config: LookHelpConfig,
) -> AppResult<()> {
    state.open_window(app, config)
}

#[tauri::command]
pub fn close_overlay_window(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
) -> AppResult<()> {
    state.close_overlay_window(app)
}

#[tauri::command]
pub fn close_look_help_window(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
) -> AppResult<()> {
    state.close_window(app)
}

#[tauri::command]
pub fn overlay_status(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
) -> AppResult<OverlayStatus> {
    state.status(&app)
}

#[tauri::command]
pub fn look_help_status(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
) -> AppResult<LookHelpStatus> {
    state.status(&app)
}

#[tauri::command]
pub async fn capture_look_help(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
) -> AppResult<()> {
    state.capture_once(app).await
}

#[tauri::command]
pub fn update_overlay_geometry(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
    geometry: OverlayGeometry,
) -> AppResult<()> {
    state.update_geometry(&app, geometry)
}

#[tauri::command]
pub fn update_overlay_config(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
    config: OverlayConfig,
) -> AppResult<()> {
    state.update_config(&app, config)
}

#[tauri::command]
pub fn update_look_help_geometry(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
    geometry: OverlayGeometry,
) -> AppResult<()> {
    state.update_geometry(&app, geometry)
}

#[tauri::command]
pub fn update_look_help_config(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
    config: LookHelpConfig,
) -> AppResult<()> {
    state.update_config(&app, config)
}

#[tauri::command]
pub fn set_overlay_paused(
    app: AppHandle,
    state: State<'_, overlay::OverlayState>,
    paused: bool,
) -> AppResult<()> {
    state.set_paused(app, paused)
}

#[tauri::command]
pub fn set_look_help_paused(
    app: AppHandle,
    state: State<'_, look_help::LookHelpState>,
    paused: bool,
) -> AppResult<()> {
    state.set_paused(app, paused)
}

#[tauri::command]
pub fn open_screen_recording_settings() -> AppResult<()> {
    overlay::open_screen_recording_settings()
}
