mod ai;
mod audio;
mod commands;
mod error;
mod models;
mod security;
mod session;

use commands::{
    export_transcript, force_translate_boundary, get_app_status, has_api_key, list_audio_devices,
    pause_session, play_test_tone, resume_session, save_api_key, start_session, stop_session,
    test_api_key,
};
use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            list_audio_devices,
            start_session,
            pause_session,
            resume_session,
            stop_session,
            force_translate_boundary,
            save_api_key,
            has_api_key,
            test_api_key,
            export_transcript,
            play_test_tone
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Baka Trans");
}
