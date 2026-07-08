use crate::audio;
use crate::error::AppResult;
use crate::models::{
    AppStatus, AudioDevices, AudioOutputChannel, ExportRequest, ExportedTranscript,
    ManualBoundaryRequest, SessionConfig,
};
use crate::security;
use crate::session::AppState;
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
