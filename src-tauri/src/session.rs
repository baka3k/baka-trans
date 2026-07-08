use crate::audio::{self, CaptureRuntime, PlaybackRuntime};
use crate::error::{AppError, AppResult};
use crate::models::{
    AppStatus, ExportFormat, ExportRequest, ExportedTranscript, ManualBoundaryEvent,
    ManualBoundaryRequest, ManualBoundaryStatus, SessionConfig, SessionStatus, TranscriptItem,
    TranscriptStatus,
};
use crate::{ai, security};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

const MANUAL_BOUNDARY_DEBOUNCE_MS: u64 = 1_000;

pub struct AppState {
    status: Mutex<SessionStatus>,
    capture: Mutex<Option<CaptureRuntime>>,
    playback: Mutex<Option<PlaybackRuntime>>,
    monitor_playback: Mutex<Option<PlaybackRuntime>>,
    realtime_control: Mutex<Option<mpsc::Sender<ai::RealtimeControl>>>,
    last_manual_boundary_request_ms: Mutex<Option<u64>>,
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
            realtime_control: Mutex::new(None),
            last_manual_boundary_request_ms: Mutex::new(None),
            transcript: Arc::new(Mutex::new(Vec::new())),
            last_config: Mutex::new(None),
        }
    }

    pub fn app_status(&self) -> AppResult<AppStatus> {
        let api_key = security::api_key_status();
        Ok(AppStatus {
            session_status: self.status()?,
            has_api_key: api_key.is_some(),
            api_key_source: api_key.as_ref().map(|info| info.source),
            api_key_fingerprint: api_key.map(|info| info.fingerprint),
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
        let current_status = self.status()?;
        if !matches!(current_status, SessionStatus::Idle | SessionStatus::Error) {
            return Err(AppError::new(
                "session_busy",
                "A translation session is already running.",
            ));
        }
        if current_status == SessionStatus::Error {
            self.clear_runtime();
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
        if let Err(error) = self.start_pipeline(app.clone(), config) {
            let _ = self.capture.lock().map(|mut capture| *capture = None);
            let _ = self.playback.lock().map(|mut playback| *playback = None);
            let _ = self
                .monitor_playback
                .lock()
                .map(|mut monitor_playback| *monitor_playback = None);
            let _ = self
                .realtime_control
                .lock()
                .map(|mut realtime_control| *realtime_control = None);
            let _ = self
                .last_manual_boundary_request_ms
                .lock()
                .map(|mut last_request| *last_request = None);
            let _ = self.set_status(&app, SessionStatus::Idle);
            return Err(error);
        }

        Ok(())
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
        let playback_active = self.playback.lock().map_err(lock_error)?.is_some();
        let monitor_active = self.monitor_playback.lock().map_err(lock_error)?.is_some();
        if playback_active || monitor_active {
            return Err(AppError::new(
                "session_still_pausing",
                "Wait for the current translation session to finish draining before resuming.",
            ));
        }

        self.start_pipeline(app, config)
    }

    pub fn stop_session(&self, app: AppHandle) -> AppResult<()> {
        match self.status()? {
            SessionStatus::Idle => return Ok(()),
            SessionStatus::Error => {
                self.clear_runtime();
                return self.set_status(&app, SessionStatus::Idle);
            }
            SessionStatus::Stopping => return Ok(()),
            _ => {}
        }
        self.set_status(&app, SessionStatus::Stopping)?;
        *self.capture.lock().map_err(lock_error)? = None;
        let control = self.realtime_control.lock().map_err(lock_error)?.clone();
        if let Some(control) = control {
            let _ = control.try_send(ai::RealtimeControl::Stop);
        } else {
            self.clear_runtime();
            self.set_status(&app, SessionStatus::Idle)?;
        }
        Ok(())
    }

    pub fn force_translate_boundary(
        &self,
        app: AppHandle,
        request: ManualBoundaryRequest,
    ) -> AppResult<()> {
        let status = self.status()?;
        if !matches!(
            status,
            SessionStatus::Listening | SessionStatus::Translating | SessionStatus::Speaking
        ) {
            return Err(AppError::new(
                "manual_boundary_inactive",
                "Start or resume translation before forcing a boundary.",
            ));
        }

        let now = now_ms();
        {
            let mut last_request = self
                .last_manual_boundary_request_ms
                .lock()
                .map_err(lock_error)?;
            if last_request
                .is_some_and(|last| now.saturating_sub(last) < MANUAL_BOUNDARY_DEBOUNCE_MS)
            {
                emit_manual_boundary(
                    &app,
                    ManualBoundaryStatus::RateLimited,
                    "Still translating",
                    None,
                )?;
                return Ok(());
            }
            *last_request = Some(now);
        }

        let tx = self
            .realtime_control
            .lock()
            .map_err(lock_error)?
            .clone()
            .ok_or_else(|| {
                AppError::new(
                    "manual_boundary_unavailable",
                    "Realtime translation is not ready for manual boundaries.",
                )
            })?;

        emit_manual_boundary(&app, ManualBoundaryStatus::Pending, "Boundary sent", None)?;
        tx.try_send(ai::RealtimeControl::ForceBoundary(request))
            .map_err(|err| {
                let error = AppError::new("manual_boundary_send_error", err.to_string());
                let _ =
                    emit_manual_boundary(&app, ManualBoundaryStatus::Error, &error.message, None);
                error
            })
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
        let playback = audio::start_playback_with_channel(
            &config.output_device_id,
            config.translation_output_channel,
        )?;
        let playback_tx = playback.sender();
        let monitor_playback = if config.monitor_original_audio {
            Some(audio::start_playback_with_channel(
                &config.monitor_output_device_id,
                config.monitor_output_channel,
            )?)
        } else {
            None
        };
        let monitor_tx = monitor_playback.as_ref().map(PlaybackRuntime::sender);
        let (capture, audio_rx) =
            audio::start_capture(app.clone(), &config.input_device_id, monitor_tx)?;
        let (control_tx, control_rx) = mpsc::channel(8);
        let transcript_store = Arc::clone(&self.transcript);
        *self.capture.lock().map_err(lock_error)? = Some(capture);
        *self.playback.lock().map_err(lock_error)? = Some(playback);
        *self.monitor_playback.lock().map_err(lock_error)? = monitor_playback;
        *self.realtime_control.lock().map_err(lock_error)? = Some(control_tx);
        *self
            .last_manual_boundary_request_ms
            .lock()
            .map_err(lock_error)? = None;
        self.set_status(&app, SessionStatus::Listening)?;

        tauri::async_runtime::spawn(async move {
            let result = ai::run_realtime_translation(
                app.clone(),
                config,
                api_key,
                audio_rx,
                control_rx,
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
        self.clear_runtime();

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
                let current = self.status().unwrap_or(SessionStatus::Idle);
                if current == SessionStatus::Stopping {
                    let _ = self.set_status(app, SessionStatus::Idle);
                } else {
                    let _ = app.emit("app-error", error);
                    let _ = self.set_status(app, SessionStatus::Error);
                }
            }
        }
    }

    fn clear_runtime(&self) {
        let _ = self.capture.lock().map(|mut capture| *capture = None);
        let _ = self.playback.lock().map(|mut playback| *playback = None);
        let _ = self
            .monitor_playback
            .lock()
            .map(|mut monitor_playback| *monitor_playback = None);
        let _ = self
            .realtime_control
            .lock()
            .map(|mut realtime_control| *realtime_control = None);
        let _ = self
            .last_manual_boundary_request_ms
            .lock()
            .map(|mut last_request| *last_request = None);
    }
}

fn emit_manual_boundary(
    app: &AppHandle,
    status: ManualBoundaryStatus,
    message: impl Into<String>,
    committed_at_ms: Option<u64>,
) -> AppResult<()> {
    app.emit(
        "manual-boundary-status",
        ManualBoundaryEvent {
            status,
            message: message.into(),
            committed_at_ms,
        },
    )
    .map_err(|err| AppError::new("event_emit_error", err.to_string()))
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

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
