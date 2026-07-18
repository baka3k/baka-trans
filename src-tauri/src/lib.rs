mod ai;
mod audio;
mod commands;
mod error;
mod llm;
mod local_translation;
mod look_help;
mod models;
mod overlay;
mod security;
mod session;
mod summary_agent;
mod tts;
mod vieneu;
#[cfg(target_os = "windows")]
mod windows_ocr;

use commands::{
    cancel_vieneu_runtime_install, capture_look_help, close_look_help_window, close_overlay_window,
    delete_llm_profile, download_whisper_model, export_transcript, force_translate_boundary,
    get_app_status, get_local_translation_config, get_transcript_snapshot,
    get_vieneu_runtime_status, has_api_key, has_translation_api_key, install_vieneu_runtime,
    list_audio_devices, list_llm_profiles, list_local_tts_voices, list_whisper_models,
    look_help_status, open_look_help_window, open_overlay_window, open_screen_recording_settings,
    overlay_status, pause_session, play_test_tone, preview_local_tts, restart_vieneu_runtime,
    resume_session, run_meeting_summary_agent, save_api_key, save_llm_profile,
    save_local_translation_config, save_translation_api_key, set_look_help_paused,
    set_overlay_paused, start_local_monitor, start_session, stop_local_monitor, stop_session,
    stop_test_tone, test_api_key, test_llm_profile, test_local_translation_config,
    test_translation_api_key, translation_credential_status, update_look_help_config,
    update_look_help_geometry, update_overlay_config, update_overlay_geometry,
};
use look_help::LookHelpState;
use overlay::OverlayState;
use session::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(OverlayState::new())
        .manage(LookHelpState::new())
        .manage(vieneu::VieNeuManager::new())
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            get_transcript_snapshot,
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
            get_local_translation_config,
            save_local_translation_config,
            test_local_translation_config,
            list_whisper_models,
            download_whisper_model,
            get_vieneu_runtime_status,
            install_vieneu_runtime,
            cancel_vieneu_runtime_install,
            restart_vieneu_runtime,
            list_local_tts_voices,
            preview_local_tts,
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
            open_look_help_window,
            close_overlay_window,
            close_look_help_window,
            overlay_status,
            look_help_status,
            capture_look_help,
            update_overlay_config,
            update_overlay_geometry,
            update_look_help_geometry,
            update_look_help_config,
            set_overlay_paused,
            set_look_help_paused,
            open_screen_recording_settings
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Baka Trans");
}
