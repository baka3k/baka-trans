use crate::audio;
use crate::error::{AppError, AppResult};
use crate::models::{
    ApiKeyTestResult, AppStatus, AudioDevices, AudioOutputChannel, ExportRequest,
    ExportedTranscript, LlmProviderProfile, LlmProviderProfileDraft, LlmProviderTestResult,
    LocalTranslationConfig, LocalTranslationConfigDraft, LocalTranslationTestResult, LocalVoice,
    LookHelpConfig, LookHelpStatus, ManualBoundaryRequest, MeetingSummaryConfig,
    MeetingSummaryResult, MeetingSummaryStatus, MeetingSummaryStatusEvent, OverlayConfig,
    OverlayGeometry, OverlayStatus, SessionConfig, TranscriptItem, TranslationCredentialStatus,
    TranslationProvider,
};
use crate::session::AppState;
use crate::{ai, llm, local_translation, look_help, overlay, security, summary_agent, tts};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    state.app_status()
}

#[tauri::command]
pub fn get_transcript_snapshot(state: State<'_, AppState>) -> AppResult<Vec<TranscriptItem>> {
    state.transcript_snapshot()
}

#[tauri::command]
pub fn list_audio_devices() -> AppResult<AudioDevices> {
    audio::list_devices()
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
pub fn pause_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.pause_session(app)
}

#[tauri::command]
pub async fn resume_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.resume_session(app).await
}

#[tauri::command]
pub fn stop_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.stop_session(app)
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
pub fn save_api_key(api_key: String) -> AppResult<()> {
    security::save_api_key(&api_key)
}

#[tauri::command]
pub fn save_translation_api_key(provider: TranslationProvider, api_key: String) -> AppResult<()> {
    security::save_translation_api_key(provider, &api_key)
}

#[tauri::command]
pub fn has_api_key() -> bool {
    security::has_api_key()
}

#[tauri::command]
pub fn has_translation_api_key(provider: TranslationProvider) -> bool {
    security::has_translation_api_key(provider)
}

#[tauri::command]
pub fn translation_credential_status(provider: TranslationProvider) -> TranslationCredentialStatus {
    security::translation_credential_status(provider)
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
        TranslationProvider::LocalWhisperOllama => {
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
pub fn get_local_translation_config() -> AppResult<LocalTranslationConfig> {
    local_translation::get_config()
}

#[tauri::command]
pub fn save_local_translation_config(
    draft: LocalTranslationConfigDraft,
) -> AppResult<LocalTranslationConfig> {
    local_translation::save_config(draft)
}

#[tauri::command]
pub async fn test_local_translation_config(
    draft: LocalTranslationConfigDraft,
) -> AppResult<LocalTranslationTestResult> {
    local_translation::test_config(draft).await
}

#[tauri::command]
pub fn list_local_tts_voices() -> AppResult<Vec<LocalVoice>> {
    tts::list_voices()
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
pub fn list_llm_profiles() -> AppResult<Vec<LlmProviderProfile>> {
    llm::list_profiles()
}

#[tauri::command]
pub fn save_llm_profile(draft: LlmProviderProfileDraft) -> AppResult<LlmProviderProfile> {
    llm::save_profile(draft)
}

#[tauri::command]
pub fn delete_llm_profile(profile_id: String) -> AppResult<()> {
    llm::delete_profile(&profile_id)
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
pub fn play_test_tone(
    state: State<'_, AppState>,
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    state.start_test_tone(&output_device_id, output_channel)
}

#[tauri::command]
pub fn stop_test_tone(state: State<'_, AppState>) -> AppResult<()> {
    state.stop_test_tone()
}

#[tauri::command]
pub fn start_local_monitor(
    app: AppHandle,
    state: State<'_, AppState>,
    input_device_id: String,
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    state.start_local_monitor(app, &input_device_id, &output_device_id, output_channel)
}

#[tauri::command]
pub fn stop_local_monitor(state: State<'_, AppState>) -> AppResult<()> {
    state.stop_local_monitor()
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
