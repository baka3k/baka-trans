use crate::audio::{self, CaptureRuntime, PlaybackRuntime, TestToneRuntime};
use crate::error::{AppError, AppResult};
use crate::models::{
    AppStatus, AudioOutputChannel, ExportFormat, ExportRequest, ExportedTranscript,
    ManualBoundaryEvent, ManualBoundaryRequest, ManualBoundaryStatus, SessionConfig, SessionStatus,
    TranscriptItem, TranscriptStatus, TranslationProvider,
};
use crate::{ai, local_translation, security};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

const MANUAL_BOUNDARY_DEBOUNCE_MS: u64 = 1_000;

pub struct AppState {
    status: Mutex<SessionStatus>,
    capture: Mutex<Option<CaptureRuntime>>,
    playback: Mutex<Option<PlaybackRuntime>>,
    monitor_playback: Mutex<Option<PlaybackRuntime>>,
    test_tone: Mutex<Option<TestToneRuntime>>,
    local_monitor_capture: Mutex<Option<CaptureRuntime>>,
    local_monitor_playback: Mutex<Option<PlaybackRuntime>>,
    realtime_control: Mutex<Option<mpsc::Sender<ai::RealtimeControl>>>,
    last_manual_boundary_request_ms: Mutex<Option<u64>>,
    transcript: Arc<Mutex<Vec<TranscriptItem>>>,
    last_config: Mutex<Option<SessionConfig>>,
    session_generation: Arc<AtomicU64>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(SessionStatus::Idle),
            capture: Mutex::new(None),
            playback: Mutex::new(None),
            monitor_playback: Mutex::new(None),
            test_tone: Mutex::new(None),
            local_monitor_capture: Mutex::new(None),
            local_monitor_playback: Mutex::new(None),
            realtime_control: Mutex::new(None),
            last_manual_boundary_request_ms: Mutex::new(None),
            transcript: Arc::new(Mutex::new(Vec::new())),
            last_config: Mutex::new(None),
            session_generation: Arc::new(AtomicU64::new(0)),
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

    fn next_session_generation(&self) -> u64 {
        self.session_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn invalidate_session_generation(&self) {
        self.session_generation.fetch_add(1, Ordering::SeqCst);
    }

    fn is_session_generation_current(&self, generation: u64) -> bool {
        self.session_generation.load(Ordering::SeqCst) == generation
    }

    pub async fn start_session(&self, app: AppHandle, config: SessionConfig) -> AppResult<()> {
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
        if self.local_monitor_active()? {
            return Err(AppError::new(
                "local_monitor_active",
                "Stop the mic monitor test before starting translation.",
            ));
        }
        self.clear_test_tone();

        config.validate_translation_target_language()?;

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
        if let Err(error) = self.start_pipeline(app.clone(), config).await {
            if error.code == "session_start_cancelled" {
                return Ok(());
            }
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

    pub fn start_local_monitor(
        &self,
        app: AppHandle,
        input_device_id: &str,
        output_device_id: &str,
        output_channel: AudioOutputChannel,
    ) -> AppResult<()> {
        if self.status()? != SessionStatus::Idle {
            return Err(AppError::new(
                "session_busy",
                "Stop translation before testing live mic monitoring.",
            ));
        }
        if self.local_monitor_active()? {
            return Ok(());
        }
        self.clear_test_tone();
        if input_device_id.trim().is_empty() {
            return Err(AppError::new(
                "missing_input_device",
                "Choose a microphone before starting mic monitor.",
            ));
        }
        if output_device_id.trim().is_empty() {
            return Err(AppError::new(
                "missing_output_device",
                "Choose a speaker or headphones before starting mic monitor.",
            ));
        }

        let playback =
            audio::start_playback_with_channel(app.clone(), output_device_id, output_channel)?;
        let monitor_tx = Some(playback.sender());
        let (capture, _audio_rx) = audio::start_capture(app, input_device_id, monitor_tx)?;

        *self.local_monitor_capture.lock().map_err(lock_error)? = Some(capture);
        *self.local_monitor_playback.lock().map_err(lock_error)? = Some(playback);
        Ok(())
    }

    pub fn start_test_tone(
        &self,
        output_device_id: &str,
        output_channel: AudioOutputChannel,
    ) -> AppResult<()> {
        if self.status()? != SessionStatus::Idle {
            return Err(AppError::new(
                "session_busy",
                "Stop translation before testing audio output.",
            ));
        }
        if self.local_monitor_active()? {
            return Err(AppError::new(
                "local_monitor_active",
                "Stop the mic monitor before testing audio output.",
            ));
        }
        if output_device_id.trim().is_empty() {
            return Err(AppError::new(
                "missing_output_device",
                "Choose a speaker or headphones before testing audio output.",
            ));
        }

        self.clear_test_tone();
        let tone = audio::start_test_tone(output_device_id, output_channel)?;
        *self.test_tone.lock().map_err(lock_error)? = Some(tone);
        Ok(())
    }

    pub fn stop_test_tone(&self) -> AppResult<()> {
        self.clear_test_tone();
        Ok(())
    }

    pub fn stop_local_monitor(&self) -> AppResult<()> {
        self.clear_local_monitor();
        Ok(())
    }

    pub fn pause_session(&self, app: AppHandle) -> AppResult<()> {
        if self.status()? == SessionStatus::Idle {
            return Ok(());
        }
        *self.capture.lock().map_err(lock_error)? = None;
        self.set_status(&app, SessionStatus::Paused)
    }

    pub async fn resume_session(&self, app: AppHandle) -> AppResult<()> {
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
        let control_active = self.realtime_control.lock().map_err(lock_error)?.is_some();
        if playback_active || monitor_active || control_active {
            return Err(AppError::new(
                "session_still_pausing",
                "Wait for the current translation session to finish draining before resuming.",
            ));
        }

        self.set_status(&app, SessionStatus::Starting)?;
        if let Err(error) = self.start_pipeline(app.clone(), config).await {
            if error.code == "session_start_cancelled" {
                return Ok(());
            }
            let _ = self.set_status(&app, SessionStatus::Paused);
            return Err(error);
        }
        Ok(())
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
        let local_session = self
            .last_config
            .lock()
            .map_err(lock_error)?
            .as_ref()
            .is_some_and(|config| config.translation_provider == TranslationProvider::LocalWhisper);
        if local_session {
            *self.playback.lock().map_err(lock_error)? = None;
            *self.monitor_playback.lock().map_err(lock_error)? = None;
        }
        let control = self.realtime_control.lock().map_err(lock_error)?.clone();
        if let Some(control) = control {
            let _ = control.try_send(ai::RealtimeControl::Stop);
        } else {
            self.invalidate_session_generation();
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

    pub fn transcript_snapshot(&self) -> AppResult<Vec<TranscriptItem>> {
        Ok(self.transcript.lock().map_err(lock_error)?.clone())
    }

    async fn start_pipeline(&self, app: AppHandle, config: SessionConfig) -> AppResult<()> {
        let generation = self.next_session_generation();
        let provider = config.translation_provider;
        let local_config = if provider == TranslationProvider::LocalWhisper {
            let local_config = local_translation::validated_runtime_config()?;
            local_translation::validate_local_session_languages(
                config.source_language,
                config.target_language,
                local_config.tts_provider,
            )?;
            Some(local_config)
        } else {
            None
        };
        let api_key = if provider.requires_api_key() {
            Some(security::load_translation_api_key(provider)?)
        } else {
            None
        };
        let local_context_result = if let Some(local_config) = &local_config {
            let model_path = local_config.model_path.clone();
            let use_gpu = local_config.use_gpu;
            Some(
                tauri::async_runtime::spawn_blocking(move || {
                    local_translation::load_whisper_context(&model_path, use_gpu)
                })
                .await,
            )
        } else {
            None
        };
        if !self.is_session_generation_current(generation)
            || self.status()? != SessionStatus::Starting
        {
            return Err(AppError::new(
                "session_start_cancelled",
                "Translation startup was cancelled before the local model was ready.",
            ));
        }
        let local_context = local_context_result
            .map(|result| {
                result
                    .map_err(|err| AppError::new("local_whisper_join_error", err.to_string()))?
                    .map(Arc::new)
            })
            .transpose()?;
        let capture_sample_rate = capture_sample_rate(provider);
        let playback = Some(audio::start_playback_with_channel_at_sample_rate(
            app.clone(),
            &config.output_device_id,
            config.translation_output_channel,
            translated_output_sample_rate(provider),
        )?);
        let playback_tx = playback.as_ref().map(PlaybackRuntime::sender);
        let monitor_playback = if should_start_original_monitor(&config) {
            Some(audio::start_playback_with_channel_at_sample_rate(
                app.clone(),
                &config.monitor_output_device_id,
                config.monitor_output_channel,
                capture_sample_rate,
            )?)
        } else {
            None
        };
        let monitor_tx = monitor_playback.as_ref().map(PlaybackRuntime::sender);
        let (capture, audio_rx) = audio::start_capture_at_sample_rate(
            app.clone(),
            &config.input_device_id,
            monitor_tx,
            capture_sample_rate,
        )?;
        let (control_tx, control_rx) = mpsc::channel(8);
        let transcript_store = Arc::clone(&self.transcript);
        let active_generation = self.session_generation.clone();
        *self.capture.lock().map_err(lock_error)? = Some(capture);
        *self.playback.lock().map_err(lock_error)? = playback;
        *self.monitor_playback.lock().map_err(lock_error)? = monitor_playback;
        *self.realtime_control.lock().map_err(lock_error)? = Some(control_tx);
        *self
            .last_manual_boundary_request_ms
            .lock()
            .map_err(lock_error)? = None;
        self.set_status(&app, SessionStatus::Listening)?;

        tauri::async_runtime::spawn(async move {
            let result = match provider {
                TranslationProvider::OpenaiRealtime => {
                    ai::run_realtime_translation(
                        app.clone(),
                        config,
                        api_key.expect("cloud provider API key was validated"),
                        audio_rx,
                        control_rx,
                        playback_tx.expect("cloud provider playback was created"),
                        transcript_store,
                    )
                    .await
                }
                TranslationProvider::GoogleLiveTranslate => {
                    ai::run_google_live_translation(
                        app.clone(),
                        config,
                        api_key.expect("cloud provider API key was validated"),
                        audio_rx,
                        control_rx,
                        playback_tx.expect("cloud provider playback was created"),
                        transcript_store,
                    )
                    .await
                }
                TranslationProvider::LocalWhisper => {
                    ai::run_local_translation(
                        app.clone(),
                        ai::LocalTranslationRuntime::new(
                            local_config.expect("local config was validated"),
                            local_context.expect("local Whisper context was loaded"),
                            playback_tx.expect("local provider playback was created"),
                            config.source_language,
                            config.target_language,
                        ),
                        audio_rx,
                        control_rx,
                        transcript_store,
                        generation,
                        active_generation,
                    )
                    .await
                }
            };
            let state = app.state::<AppState>();
            state.finish_pipeline(&app, result, generation);
        });

        Ok(())
    }

    pub(crate) fn set_pipeline_status_if_active(
        &self,
        app: &AppHandle,
        generation: u64,
        next_status: SessionStatus,
    ) -> AppResult<()> {
        let mut current = self.status.lock().map_err(lock_error)?;
        if !self.is_session_generation_current(generation)
            || !matches!(
                *current,
                SessionStatus::Listening | SessionStatus::Translating | SessionStatus::Speaking
            )
        {
            return Ok(());
        }
        *current = next_status;
        drop(current);
        app.emit("session-status", next_status)
            .map_err(|err| AppError::new("event_emit_error", err.to_string()))
    }

    fn finish_pipeline(&self, app: &AppHandle, result: AppResult<()>, generation: u64) {
        if !self.is_session_generation_current(generation) {
            return;
        }

        self.clear_runtime();

        match result {
            Ok(()) => {
                let current = self.status().unwrap_or(SessionStatus::Idle);
                if current != SessionStatus::Paused {
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

    fn local_monitor_active(&self) -> AppResult<bool> {
        Ok(self
            .local_monitor_capture
            .lock()
            .map_err(lock_error)?
            .is_some()
            || self
                .local_monitor_playback
                .lock()
                .map_err(lock_error)?
                .is_some())
    }

    fn clear_local_monitor(&self) {
        let _ = self
            .local_monitor_capture
            .lock()
            .map(|mut capture| *capture = None);
        let _ = self
            .local_monitor_playback
            .lock()
            .map(|mut playback| *playback = None);
    }

    fn clear_test_tone(&self) {
        let _ = self.test_tone.lock().map(|mut test_tone| *test_tone = None);
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

fn should_start_original_monitor(config: &SessionConfig) -> bool {
    config.monitor_original_audio
        && !has_output_monitor_conflict(
            &config.output_device_id,
            config.translation_output_channel,
            &config.monitor_output_device_id,
            config.monitor_output_channel,
        )
}

fn has_output_monitor_conflict(
    output_device_id: &str,
    translation_channel: AudioOutputChannel,
    monitor_output_device_id: &str,
    monitor_channel: AudioOutputChannel,
) -> bool {
    same_audio_device(output_device_id, monitor_output_device_id)
        && !uses_opposite_channels(translation_channel, monitor_channel)
}

fn same_audio_device(left: &str, right: &str) -> bool {
    let left = left.trim();
    let right = right.trim();
    !left.is_empty() && left == right
}

fn uses_opposite_channels(
    translation_channel: AudioOutputChannel,
    monitor_channel: AudioOutputChannel,
) -> bool {
    matches!(
        (translation_channel, monitor_channel),
        (AudioOutputChannel::Left, AudioOutputChannel::Right)
            | (AudioOutputChannel::Right, AudioOutputChannel::Left)
    )
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

fn capture_sample_rate(provider: TranslationProvider) -> u32 {
    match provider {
        TranslationProvider::OpenaiRealtime => audio::OPENAI_REALTIME_SAMPLE_RATE,
        TranslationProvider::GoogleLiveTranslate => audio::GOOGLE_LIVE_INPUT_SAMPLE_RATE,
        TranslationProvider::LocalWhisper => 16_000,
    }
}

fn translated_output_sample_rate(provider: TranslationProvider) -> u32 {
    match provider {
        TranslationProvider::OpenaiRealtime => audio::OPENAI_REALTIME_SAMPLE_RATE,
        TranslationProvider::GoogleLiveTranslate => audio::GOOGLE_LIVE_OUTPUT_SAMPLE_RATE,
        TranslationProvider::LocalWhisper => crate::tts::LOCAL_TTS_SAMPLE_RATE,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Language, TranslationStyle};

    fn session_config() -> SessionConfig {
        SessionConfig {
            translation_provider: TranslationProvider::OpenaiRealtime,
            source_language: Language::Auto,
            target_language: Language::En,
            translation_style: TranslationStyle::TechnicalMeetingSafe,
            input_device_id: "input:0:BlackHole 2ch".to_string(),
            output_device_id: "output:0:Headphones".to_string(),
            translation_output_channel: AudioOutputChannel::All,
            monitor_output_device_id: String::new(),
            monitor_output_channel: AudioOutputChannel::All,
            monitor_original_audio: false,
            voice_id: "marin".to_string(),
            fallback_enabled: false,
        }
    }

    #[test]
    fn stale_start_token_cannot_claim_a_newer_start() {
        let state = AppState::new();
        let start_a = state.next_session_generation();
        state.invalidate_session_generation();
        let start_b = state.next_session_generation();

        assert!(!state.is_session_generation_current(start_a));
        assert!(state.is_session_generation_current(start_b));
    }

    #[test]
    fn allows_same_output_device_without_opposite_channels() {
        let mut config = session_config();
        config.monitor_original_audio = true;
        config.monitor_output_device_id = config.output_device_id.clone();
        config.translation_output_channel = AudioOutputChannel::All;
        config.monitor_output_channel = AudioOutputChannel::Left;

        assert!(validate_routing_config(&config).is_ok());
        assert!(!should_start_original_monitor(&config));
    }

    #[test]
    fn allows_same_output_device_with_opposite_channels() {
        let mut config = session_config();
        config.monitor_original_audio = true;
        config.monitor_output_device_id = config.output_device_id.clone();
        config.translation_output_channel = AudioOutputChannel::Left;
        config.monitor_output_channel = AudioOutputChannel::Right;

        assert!(validate_routing_config(&config).is_ok());
        assert!(should_start_original_monitor(&config));
    }

    #[test]
    fn allows_separate_monitor_output_device_with_same_name() {
        let mut config = session_config();
        config.monitor_original_audio = true;
        config.monitor_output_device_id = "output:1:Headphones".to_string();

        assert!(validate_routing_config(&config).is_ok());
        assert!(should_start_original_monitor(&config));
    }
}
