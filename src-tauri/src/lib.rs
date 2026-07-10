mod ai;
mod audio;
mod commands;
mod error;
mod llm;
mod models;
mod overlay;
mod security;
mod session;
mod summary_agent;

use commands::{
    close_overlay_window, delete_llm_profile, export_transcript, force_translate_boundary,
    get_app_status, has_api_key, has_translation_api_key, list_audio_devices, list_llm_profiles,
    open_overlay_window, open_screen_recording_settings, overlay_status, pause_session,
    play_test_tone, resume_session, run_meeting_summary_agent, save_api_key, save_llm_profile,
    save_translation_api_key, set_overlay_paused, start_local_monitor, start_session,
    stop_local_monitor, stop_session, stop_test_tone, test_api_key, test_llm_profile,
    test_translation_api_key, translation_credential_status, update_overlay_geometry,
};
use overlay::OverlayState;
use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(OverlayState::new())
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            list_audio_devices,
            start_session,
            pause_session,
            resume_session,
            stop_session,
            force_translate_boundary,
            save_api_key,
            save_translation_api_key,
            has_api_key,
            has_translation_api_key,
            translation_credential_status,
            test_api_key,
            test_translation_api_key,
            list_llm_profiles,
            save_llm_profile,
            delete_llm_profile,
            test_llm_profile,
            run_meeting_summary_agent,
            export_transcript,
            play_test_tone,
            stop_test_tone,
            start_local_monitor,
            stop_local_monitor,
            open_overlay_window,
            close_overlay_window,
            overlay_status,
            update_overlay_geometry,
            set_overlay_paused,
            open_screen_recording_settings
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Baka Trans");
}
