use crate::audio;
use crate::error::{AppError, AppResult};
use crate::models::{
    ApiKeyTestResult, AppStatus, AudioDevices, AudioOutputChannel, ExportRequest,
    ExportedTranscript, ManualBoundaryRequest, SessionConfig,
};
use crate::session::AppState;
use crate::{ai, security};
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    state.app_status()
}

#[tauri::command]
pub fn list_audio_devices() -> AppResult<AudioDevices> {
    audio::list_devices()
}

#[tauri::command]
pub fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    config: SessionConfig,
) -> AppResult<()> {
    state.start_session(app, config)
}

#[tauri::command]
pub fn pause_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.pause_session(app)
}

#[tauri::command]
pub fn resume_session(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.resume_session(app)
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
pub fn has_api_key() -> bool {
    security::has_api_key()
}

#[tauri::command]
pub async fn test_api_key() -> AppResult<ApiKeyTestResult> {
    let info = security::load_api_key_info()?;
    if let Err(error) = ai::test_realtime_connection(&info.key).await {
        return Err(AppError::new(
            error.code,
            format!(
                "Realtime test failed for {} key {}: {}",
                api_key_source_label(info.source),
                info.fingerprint,
                error.message
            ),
        ));
    }
    Ok(ApiKeyTestResult {
        source: info.source,
        fingerprint: info.fingerprint,
        message: "Realtime translation accepted this key.".to_string(),
    })
}

fn api_key_source_label(source: crate::models::ApiKeySource) -> &'static str {
    match source {
        crate::models::ApiKeySource::Environment => "environment",
        crate::models::ApiKeySource::Keychain => "Keychain",
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
    output_device_id: String,
    output_channel: AudioOutputChannel,
) -> AppResult<()> {
    audio::play_test_tone(&output_device_id, output_channel)
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
