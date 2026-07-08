use crate::audio::{self, CaptureRuntime, PlaybackRuntime};
use crate::error::{AppError, AppResult};
use crate::models::{
    AppStatus, ExportFormat, ExportRequest, ExportedTranscript, SessionConfig, SessionStatus,
    TranscriptItem, TranscriptStatus,
};
use crate::{ai, security};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    status: Mutex<SessionStatus>,
    capture: Mutex<Option<CaptureRuntime>>,
    playback: Mutex<Option<PlaybackRuntime>>,
    monitor_playback: Mutex<Option<PlaybackRuntime>>,
    transcript: Arc<Mutex<Vec<TranscriptItem>>>,
    last_config: Mutex<Option<SessionConfig>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(SessionStatus::Idle),
            capture: Mutex::new(None),
            playback: Mutex::new(None),
            monitor_playback: Mutex::new(None),
            transcript: Arc::new(Mutex::new(Vec::new())),
            last_config: Mutex::new(None),
        }
    }

    pub fn app_status(&self) -> AppResult<AppStatus> {
        Ok(AppStatus {
            session_status: self.status()?,
            has_api_key: security::has_api_key(),
            transcript_count: self.transcript.lock().map_err(lock_error)?.len(),
        })
    }

    pub fn status(&self) -> AppResult<SessionStatus> {
        Ok(*self.status.lock().map_err(lock_error)?)
    }

    pub fn set_status(&self, app: &AppHandle, status: SessionStatus) -> AppResult<()> {
        *self.status.lock().map_err(lock_error)? = status;
        app.emit("session-status", status)
            .map_err(|err| AppError::new("event_emit_error", err.to_string()))
    }

    pub fn start_session(&self, app: AppHandle, config: SessionConfig) -> AppResult<()> {
        if self.status()? != SessionStatus::Idle {
            return Err(AppError::new(
                "session_busy",
                "A translation session is already running.",
            ));
        }

        if config.target_language == config.source_language
            && config.source_language.realtime_code() != "auto"
        {
            app.emit(
                "app-error",
                AppError::new(
                    "same_language_pair",
                    "Source and target languages are the same. Translation may be unnecessary.",
                ),
            )
            .map_err(|err| AppError::new("event_emit_error", err.to_string()))?;
        }

        validate_routing_config(&config)?;

        self.set_status(&app, SessionStatus::Starting)?;
        self.transcript.lock().map_err(lock_error)?.clear();
        *self.last_config.lock().map_err(lock_error)? = Some(config.clone());
        self.start_pipeline(app, config)
    }

    pub fn pause_session(&self, app: AppHandle) -> AppResult<()> {
        if self.status()? == SessionStatus::Idle {
            return Ok(());
        }
        *self.capture.lock().map_err(lock_error)? = None;
        self.set_status(&app, SessionStatus::Paused)
    }

    pub fn resume_session(&self, app: AppHandle) -> AppResult<()> {
        let config = self
            .last_config
            .lock()
            .map_err(lock_error)?
            .clone()
            .ok_or_else(|| {
                AppError::new(
                    "session_not_configured",
                    "No previous session config exists.",
                )
            })?;
        if self.status()? != SessionStatus::Paused {
            return Ok(());
        }
        if self.playback.lock().map_err(lock_error)?.is_some() {
            return Err(AppError::new(
                "session_still_pausing",
                "Wait for the current translation session to finish draining before resuming.",
            ));
        }

        self.start_pipeline(app, config)
    }

    pub fn stop_session(&self, app: AppHandle) -> AppResult<()> {
        if self.status()? == SessionStatus::Idle {
            return Ok(());
        }
        self.set_status(&app, SessionStatus::Stopping)?;
        *self.capture.lock().map_err(lock_error)? = None;
        Ok(())
    }

    pub fn export_transcript(&self, request: ExportRequest) -> AppResult<ExportedTranscript> {
        let transcript = self.transcript.lock().map_err(lock_error)?;
        let content = match request.format {
            ExportFormat::Text => render_text(&transcript),
            ExportFormat::Markdown => render_markdown(&transcript),
        };
        let extension = match request.format {
            ExportFormat::Text => "txt",
            ExportFormat::Markdown => "md",
        };

        Ok(ExportedTranscript {
            file_name: format!("baka-trans-transcript.{extension}"),
            content,
        })
    }

    fn start_pipeline(&self, app: AppHandle, config: SessionConfig) -> AppResult<()> {
        let api_key = security::load_api_key()?;
        let playback = audio::start_playback(&config.output_device_id)?;
        let playback_tx = playback.sender();
        let monitor_playback = if config.monitor_original_audio {
            Some(audio::start_playback(&config.monitor_output_device_id)?)
        } else {
            None
        };
        let monitor_tx = monitor_playback.as_ref().map(PlaybackRuntime::sender);
        let (capture, audio_rx) =
            audio::start_capture(app.clone(), &config.input_device_id, monitor_tx)?;
        let transcript_store = Arc::clone(&self.transcript);
        *self.capture.lock().map_err(lock_error)? = Some(capture);
        *self.playback.lock().map_err(lock_error)? = Some(playback);
        *self.monitor_playback.lock().map_err(lock_error)? = monitor_playback;
        self.set_status(&app, SessionStatus::Listening)?;

        tauri::async_runtime::spawn(async move {
            let result = ai::run_realtime_translation(
                app.clone(),
                config,
                api_key,
                audio_rx,
                playback_tx,
                transcript_store,
            )
            .await;
            let state = app.state::<AppState>();
            state.finish_pipeline(&app, result);
        });

        Ok(())
    }

    fn finish_pipeline(&self, app: &AppHandle, result: AppResult<()>) {
        let _ = self.capture.lock().map(|mut capture| *capture = None);
        let _ = self.playback.lock().map(|mut playback| *playback = None);
        let _ = self
            .monitor_playback
            .lock()
            .map(|mut monitor_playback| *monitor_playback = None);

        match result {
            Ok(()) => {
                let current = self.status().unwrap_or(SessionStatus::Idle);
                if current == SessionStatus::Stopping {
                    let _ = self.set_status(app, SessionStatus::Idle);
                } else if current != SessionStatus::Paused {
                    let _ = self.set_status(app, SessionStatus::Idle);
                }
            }
            Err(error) => {
                let _ = app.emit("app-error", error);
                let _ = self.set_status(app, SessionStatus::Error);
            }
        }
    }
}

fn render_text(items: &[TranscriptItem]) -> String {
    items
        .iter()
        .map(|item| {
            format!(
                "[{}] Source: {}\nTranslation: {}\n",
                item.timestamp_ms, item.source_text, item.translated_text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn validate_routing_config(config: &SessionConfig) -> AppResult<()> {
    if config.input_device_id.trim().is_empty() {
        return Err(AppError::new(
            "missing_input_device",
            "Choose a meeting source input before starting.",
        ));
    }

    if config.output_device_id.trim().is_empty() {
        return Err(AppError::new(
            "missing_output_device",
            "Choose a translated audio output before starting.",
        ));
    }

    if config.monitor_original_audio && config.monitor_output_device_id.trim().is_empty() {
        return Err(AppError::new(
            "missing_monitor_output",
            "Choose an original audio monitor output or disable monitoring.",
        ));
    }

    Ok(())
}

fn render_markdown(items: &[TranscriptItem]) -> String {
    let mut content = String::from("# Baka Trans Transcript\n\n");
    for item in items {
        let status = match item.status {
            TranscriptStatus::Partial => "partial",
            TranscriptStatus::Final => "final",
            TranscriptStatus::Error => "error",
        };
        content.push_str(&format!(
            "## {} ({})\n\n**Source:** {}\n\n**Translation:** {}\n\n",
            item.timestamp_ms, status, item.source_text, item.translated_text
        ));
    }
    content
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> AppError {
    AppError::new("state_lock_error", "Application state lock was poisoned.")
}
