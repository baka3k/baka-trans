use crate::error::{AppError, AppResult};
use crate::local_translation::TranslationClient;
use crate::models::{
    Language, LocalTranslationConfig, ManualBoundaryEvent, ManualBoundaryStatus, SessionStatus,
    TranscriptItem, TranscriptStatus, TranscriptUpdateMode, TranslatedAudioLevelEvent,
};
use crate::session::AppState;
use crate::tts;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use whisper_rs::{convert_integer_to_float_audio, FullParams, SamplingStrategy, WhisperContext};

use super::RealtimeControl;

const UTTERANCE_QUEUE_CAPACITY: usize = 4;
const TTS_QUEUE_CAPACITY: usize = 4;
const STOP_DRAIN_TIMEOUT_SECONDS: u64 = 2;
const CANCELLATION_GRACE_SECONDS: u64 = 2;

struct Utterance {
    id: String,
    timestamp_ms: u64,
    samples: Vec<i16>,
}

struct TranslationWorker {
    app: AppHandle,
    config: LocalTranslationConfig,
    context: Arc<WhisperContext>,
    client: TranslationClient,
    transcript_store: Arc<Mutex<Vec<TranscriptItem>>>,
    generation: u64,
    active_generation: Arc<AtomicU64>,
    cancellation: Arc<AtomicBool>,
    activity: Arc<PipelineActivity>,
    tts_tx: mpsc::Sender<TtsRequest>,
    whisper_language: Option<String>,
}

#[derive(Default)]
struct PipelineActivity {
    translation_stage: AtomicU8,
    speech_stage: AtomicU8,
}

const ACTIVITY_INACTIVE: u8 = 0;
const TRANSLATION_TRANSCRIBING: u8 = 1;
const TRANSLATION_TRANSLATING: u8 = 2;
const SPEECH_SYNTHESIZING: u8 = 1;
const SPEECH_PLAYING: u8 = 2;

struct TtsRequest {
    utterance_id: String,
    translated_text: String,
}

struct TtsWorker {
    app: AppHandle,
    config: LocalTranslationConfig,
    playback_tx: std_mpsc::SyncSender<Vec<i16>>,
    generation: u64,
    active_generation: Arc<AtomicU64>,
    cancellation: Arc<AtomicBool>,
    activity: Arc<PipelineActivity>,
}

pub struct LocalTranslationRuntime {
    config: LocalTranslationConfig,
    context: Arc<WhisperContext>,
    playback_tx: std_mpsc::SyncSender<Vec<i16>>,
    source_language: Language,
    target_language: Language,
}

impl LocalTranslationRuntime {
    pub fn new(
        config: LocalTranslationConfig,
        context: Arc<WhisperContext>,
        playback_tx: std_mpsc::SyncSender<Vec<i16>>,
        source_language: Language,
        target_language: Language,
    ) -> Self {
        Self {
            config,
            context,
            playback_tx,
            source_language,
            target_language,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnqueueOutcome {
    Queued,
    Dropped,
}

pub async fn run_local_translation(
    app: AppHandle,
    runtime: LocalTranslationRuntime,
    mut audio_rx: mpsc::Receiver<Vec<i16>>,
    mut control_rx: mpsc::Receiver<RealtimeControl>,
    transcript_store: Arc<Mutex<Vec<TranscriptItem>>>,
    generation: u64,
    active_generation: Arc<AtomicU64>,
) -> AppResult<()> {
    let LocalTranslationRuntime {
        config,
        context,
        playback_tx,
        source_language,
        target_language,
    } = runtime;
    let whisper_language =
        crate::local_translation::whisper_language_code(source_language)?.map(str::to_string);
    let client = TranslationClient::new(&config, source_language, target_language)?;
    let (utterance_tx, utterance_rx) = mpsc::channel(UTTERANCE_QUEUE_CAPACITY);
    let cancellation = Arc::new(AtomicBool::new(false));
    let activity = Arc::new(PipelineActivity::default());
    let (tts_tx, tts_rx) = mpsc::channel(TTS_QUEUE_CAPACITY);
    let tts_worker = spawn_tts_worker(
        TtsWorker {
            app: app.clone(),
            config: config.clone(),
            playback_tx,
            generation,
            active_generation: active_generation.clone(),
            cancellation: cancellation.clone(),
            activity: activity.clone(),
        },
        tts_rx,
    );
    let mut tts_worker = Some(tts_worker);
    let worker = spawn_translation_worker(
        TranslationWorker {
            app: app.clone(),
            config: config.clone(),
            context,
            client,
            transcript_store: transcript_store.clone(),
            generation,
            active_generation: active_generation.clone(),
            cancellation: cancellation.clone(),
            activity,
            tts_tx,
            whisper_language,
        },
        utterance_rx,
    );
    let mut worker = Some(worker);
    let mut segmenter = PcmSegmenter::new(&config);
    emit_local_pipeline_stage(&app, "listening")?;
    loop {
        tokio::select! {
            maybe_audio = audio_rx.recv() => {
                match maybe_audio {
                    Some(samples) => {
                        if let Some(segment) = segmenter.push(samples) {
                            let _ = enqueue_segment(
                                &app,
                                &utterance_tx,
                                segment,
                                &transcript_store,
                                generation,
                                &active_generation,
                            )?;
                        }
                    }
                    None => {
                        if let Some(segment) = segmenter.flush() {
                            let _ = enqueue_segment(
                                &app,
                                &utterance_tx,
                                segment,
                                &transcript_store,
                                generation,
                                &active_generation,
                            )?;
                        }
                        drop(utterance_tx);
                        drain_worker(&mut worker, &cancellation).await?;
                        drain_worker(&mut tts_worker, &cancellation).await?;
                        return Ok(());
                    }
                }
            }
            maybe_control = control_rx.recv() => {
                match maybe_control {
                    Some(RealtimeControl::ForceBoundary(_request)) => {
                        if let Some(segment) = segmenter.flush() {
                            match enqueue_segment(
                                &app,
                                &utterance_tx,
                                segment,
                                &transcript_store,
                                generation,
                                &active_generation,
                            ) {
                                Ok(EnqueueOutcome::Queued) => emit_manual_boundary(
                                    &app,
                                    ManualBoundaryStatus::Committed,
                                    "Local utterance committed",
                                    Some(now_ms()),
                                )?,
                                Ok(EnqueueOutcome::Dropped) => emit_manual_boundary(
                                    &app,
                                    ManualBoundaryStatus::Error,
                                    "Local translation backlog is full",
                                    None,
                                )?,
                                Err(error) => {
                                    emit_manual_boundary(
                                        &app,
                                        ManualBoundaryStatus::Error,
                                        &error.message,
                                        None,
                                    )?;
                                    return Err(error);
                                }
                            }
                        } else {
                            emit_manual_boundary(
                                &app,
                                ManualBoundaryStatus::IgnoredEmptyBuffer,
                                "No speech is buffered",
                                None,
                            )?;
                        }
                    }
                    Some(RealtimeControl::Stop) => {
                        cancellation.store(true, Ordering::SeqCst);
                        drop(utterance_tx);
                        drain_worker(&mut worker, &cancellation).await?;
                        drain_worker(&mut tts_worker, &cancellation).await?;
                        return Ok(());
                    }
                    None => {
                        cancellation.store(true, Ordering::SeqCst);
                        drop(utterance_tx);
                        drain_worker(&mut worker, &cancellation).await?;
                        drain_worker(&mut tts_worker, &cancellation).await?;
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn spawn_translation_worker(
    worker: TranslationWorker,
    mut utterance_rx: mpsc::Receiver<Utterance>,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let TranslationWorker {
            app,
            config,
            context,
            client,
            transcript_store,
            generation,
            active_generation,
            cancellation,
            activity,
            tts_tx,
            whisper_language,
        } = worker;
        while let Some(utterance) = utterance_rx.recv().await {
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }
            activity
                .translation_stage
                .store(TRANSLATION_TRANSCRIBING, Ordering::SeqCst);
            let started = Instant::now();
            settle_pipeline_activity(&app, generation, &activity)?;
            let utterance_id = utterance.id.clone();
            let timestamp_ms = utterance.timestamp_ms;
            let threads = config.threads;
            let language = whisper_language.clone();
            let whisper_context = context.clone();
            let inference_cancellation = cancellation.clone();
            let transcription = tauri::async_runtime::spawn_blocking(move || {
                transcribe(
                    &whisper_context,
                    utterance.samples,
                    threads,
                    language.as_deref(),
                    inference_cancellation,
                )
            })
            .await
            .map_err(|err| AppError::new("local_whisper_join_error", err.to_string()))?;
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }

            let source_text = match transcription {
                Ok(text) => text,
                Err(error) => {
                    emit_snapshot(
                        &app,
                        &transcript_store,
                        generation,
                        &active_generation,
                        TranscriptItem {
                            id: utterance_id,
                            timestamp_ms,
                            source_text: String::new(),
                            translated_text: String::new(),
                            status: TranscriptStatus::Error,
                            latency_ms: Some(elapsed_ms(started)),
                            revision: 1,
                            update_mode: TranscriptUpdateMode::Snapshot,
                            error_message: Some(error.message),
                        },
                    )?;
                    activity
                        .translation_stage
                        .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
                    settle_pipeline_activity(&app, generation, &activity)?;
                    continue;
                }
            };
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }

            let pending = TranscriptItem {
                id: utterance_id.clone(),
                timestamp_ms,
                source_text: source_text.clone(),
                translated_text: String::new(),
                status: TranscriptStatus::Partial,
                latency_ms: None,
                revision: 1,
                update_mode: TranscriptUpdateMode::Snapshot,
                error_message: None,
            };
            emit_snapshot(
                &app,
                &transcript_store,
                generation,
                &active_generation,
                pending.clone(),
            )?;
            activity
                .translation_stage
                .store(TRANSLATION_TRANSLATING, Ordering::SeqCst);
            settle_pipeline_activity(&app, generation, &activity)?;
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }

            let translation = client.translate(&source_text).await;
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }
            match translation {
                Ok((translated_text, _translation_latency_ms)) => {
                    let speech_text = translated_text.clone();
                    emit_snapshot(
                        &app,
                        &transcript_store,
                        generation,
                        &active_generation,
                        TranscriptItem {
                            translated_text,
                            status: TranscriptStatus::Final,
                            latency_ms: Some(elapsed_ms(started)),
                            revision: 2,
                            ..pending
                        },
                    )?;
                    match tts_tx.try_send(TtsRequest {
                        utterance_id: utterance_id.clone(),
                        translated_text: speech_text,
                    }) {
                        Ok(()) => {}
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            let _ = app.emit(
                                "app-error",
                                AppError::new(
                                    "local_tts_backlog_full",
                                    "Local speech is falling behind. The translated text was kept, but this sentence will not be spoken.",
                                ),
                            );
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            if is_worker_active(generation, &active_generation, &cancellation) {
                                let _ = app.emit(
                                    "app-error",
                                    AppError::new(
                                        "local_tts_worker_closed",
                                        "The local speech worker stopped unexpectedly.",
                                    ),
                                );
                            }
                        }
                    }
                }
                Err(error) => {
                    emit_snapshot(
                        &app,
                        &transcript_store,
                        generation,
                        &active_generation,
                        TranscriptItem {
                            status: TranscriptStatus::Error,
                            latency_ms: Some(elapsed_ms(started)),
                            revision: 2,
                            error_message: Some(error.message),
                            ..pending
                        },
                    )?;
                }
            }
            activity
                .translation_stage
                .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
            settle_pipeline_activity(&app, generation, &activity)?;
        }
        activity
            .translation_stage
            .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
        Ok(())
    })
}

fn spawn_tts_worker(
    worker: TtsWorker,
    mut tts_rx: mpsc::Receiver<TtsRequest>,
) -> JoinHandle<AppResult<()>> {
    tokio::spawn(async move {
        let TtsWorker {
            app,
            config,
            playback_tx,
            generation,
            active_generation,
            cancellation,
            activity,
        } = worker;
        while let Some(request) = tts_rx.recv().await {
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }
            activity
                .speech_stage
                .store(SPEECH_SYNTHESIZING, Ordering::SeqCst);
            settle_pipeline_activity(&app, generation, &activity)?;
            let synthesis = tts::synthesize(
                Some(&app),
                &request.translated_text,
                &config,
                cancellation.clone(),
            )
            .await;
            if !is_worker_active(generation, &active_generation, &cancellation) {
                break;
            }
            match synthesis {
                Ok(audio) => {
                    let sample_count = audio.pcm16_mono.len();
                    let translated_level = translated_audio_level(&audio.pcm16_mono);
                    if let Err(error) = playback_tx.try_send(audio.pcm16_mono) {
                        let message = match error {
                            std_mpsc::TrySendError::Full(_) => {
                                "Translated audio output is overloaded. This sentence remains in the transcript but was not played."
                            }
                            std_mpsc::TrySendError::Disconnected(_) => {
                                "The selected translated audio output disconnected."
                            }
                        };
                        let _ = app.emit(
                            "app-error",
                            AppError::new(
                                "local_tts_playback_error",
                                format!("{message} Utterance {}.", request.utterance_id),
                            ),
                        );
                    } else {
                        app.emit("translated-audio-level", translated_level)
                            .map_err(|err| AppError::new("event_emit_error", err.to_string()))?;
                        activity
                            .speech_stage
                            .store(SPEECH_PLAYING, Ordering::SeqCst);
                        settle_pipeline_activity(&app, generation, &activity)?;
                        let playback_ms = (sample_count as u64 * 1_000)
                            .saturating_div(u64::from(tts::LOCAL_TTS_SAMPLE_RATE));
                        tokio::time::sleep(Duration::from_millis(playback_ms)).await;
                        app.emit(
                            "translated-audio-level",
                            TranslatedAudioLevelEvent {
                                sample_count: 0,
                                rms: 0.0,
                                peak: 0.0,
                            },
                        )
                        .map_err(|err| AppError::new("event_emit_error", err.to_string()))?;
                    }
                }
                Err(error) if error.code == "local_tts_cancelled" => {
                    activity
                        .speech_stage
                        .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
                    break;
                }
                Err(error) => {
                    let _ = app.emit("app-error", error);
                }
            }
            activity
                .speech_stage
                .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
            settle_pipeline_activity(&app, generation, &activity)?;
        }
        activity
            .speech_stage
            .store(ACTIVITY_INACTIVE, Ordering::SeqCst);
        Ok(())
    })
}

fn settle_pipeline_activity(
    app: &AppHandle,
    generation: u64,
    activity: &PipelineActivity,
) -> AppResult<()> {
    let (status, stage) = pipeline_activity_state(activity);
    app.state::<AppState>()
        .set_pipeline_status_if_active(app, generation, status)?;
    emit_local_pipeline_stage(app, stage)
}

fn pipeline_activity_state(activity: &PipelineActivity) -> (SessionStatus, &'static str) {
    match activity.speech_stage.load(Ordering::SeqCst) {
        SPEECH_SYNTHESIZING => return (SessionStatus::Speaking, "synthesizing"),
        SPEECH_PLAYING => return (SessionStatus::Speaking, "speaking"),
        _ => {}
    }
    match activity.translation_stage.load(Ordering::SeqCst) {
        TRANSLATION_TRANSCRIBING => (SessionStatus::Listening, "transcribing"),
        TRANSLATION_TRANSLATING => (SessionStatus::Translating, "translating"),
        _ => (SessionStatus::Listening, "listening"),
    }
}

fn translated_audio_level(samples: &[i16]) -> TranslatedAudioLevelEvent {
    if samples.is_empty() {
        return TranslatedAudioLevelEvent {
            sample_count: 0,
            rms: 0.0,
            peak: 0.0,
        };
    }
    let scale = f32::from(i16::MAX);
    let mut square_sum = 0.0_f32;
    let mut peak = 0.0_f32;
    for sample in samples {
        let normalized = f32::from(*sample).abs() / scale;
        peak = peak.max(normalized);
        square_sum += normalized * normalized;
    }
    TranslatedAudioLevelEvent {
        sample_count: samples.len(),
        rms: (square_sum / samples.len() as f32).sqrt(),
        peak,
    }
}

fn emit_local_pipeline_stage(app: &AppHandle, stage: &'static str) -> AppResult<()> {
    app.emit("local-pipeline-stage", stage)
        .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

fn enqueue_segment(
    app: &AppHandle,
    utterance_tx: &mpsc::Sender<Utterance>,
    samples: Vec<i16>,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
    generation: u64,
    active_generation: &Arc<AtomicU64>,
) -> AppResult<EnqueueOutcome> {
    let utterance = Utterance {
        id: Uuid::new_v4().to_string(),
        timestamp_ms: now_ms(),
        samples,
    };
    match utterance_tx.try_send(utterance) {
        Ok(()) => Ok(EnqueueOutcome::Queued),
        Err(mpsc::error::TrySendError::Full(utterance)) => {
            let error = AppError::new(
                "local_translation_backlog_full",
                "Local translation is overloaded. Shorten utterances or use a faster model.",
            );
            emit_snapshot(
                app,
                transcript_store,
                generation,
                active_generation,
                TranscriptItem {
                    id: utterance.id,
                    timestamp_ms: utterance.timestamp_ms,
                    source_text: String::new(),
                    translated_text: String::new(),
                    status: TranscriptStatus::Error,
                    latency_ms: None,
                    revision: 1,
                    update_mode: TranscriptUpdateMode::Snapshot,
                    error_message: Some(error.message.clone()),
                },
            )?;
            let _ = app.emit("app-error", error.clone());
            Ok(EnqueueOutcome::Dropped)
        }
        Err(mpsc::error::TrySendError::Closed(_)) => Err(AppError::new(
            "local_translation_worker_closed",
            "The local translation worker stopped unexpectedly.",
        )),
    }
}

fn emit_snapshot(
    app: &AppHandle,
    transcript_store: &Arc<Mutex<Vec<TranscriptItem>>>,
    generation: u64,
    active_generation: &Arc<AtomicU64>,
    item: TranscriptItem,
) -> AppResult<()> {
    if !is_generation_active(generation, active_generation) {
        return Ok(());
    }
    {
        let mut transcript = transcript_store.lock().map_err(|_| {
            AppError::new("state_lock_error", "Transcript store lock was poisoned.")
        })?;
        if !upsert_snapshot(&mut transcript, item.clone()) {
            return Ok(());
        }
    }
    if !is_generation_active(generation, active_generation) {
        return Ok(());
    }
    app.emit("transcript-update", item)
        .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

fn upsert_snapshot(transcript: &mut Vec<TranscriptItem>, item: TranscriptItem) -> bool {
    if let Some(existing) = transcript
        .iter_mut()
        .find(|existing| existing.id == item.id)
    {
        if item.revision <= existing.revision {
            return false;
        }
        *existing = item;
    } else {
        transcript.push(item);
    }
    true
}

fn is_generation_active(generation: u64, active_generation: &Arc<AtomicU64>) -> bool {
    active_generation.load(Ordering::SeqCst) == generation
}

fn is_worker_active(
    generation: u64,
    active_generation: &Arc<AtomicU64>,
    cancellation: &Arc<AtomicBool>,
) -> bool {
    !cancellation.load(Ordering::SeqCst) && is_generation_active(generation, active_generation)
}

fn transcribe(
    context: &WhisperContext,
    samples: Vec<i16>,
    threads: u32,
    language: Option<&str>,
    cancellation: Arc<AtomicBool>,
) -> AppResult<String> {
    if samples.is_empty() {
        return Err(AppError::new(
            "local_whisper_empty_audio",
            "No speech audio was available for Whisper.",
        ));
    }
    let mut audio = vec![0.0_f32; samples.len()];
    convert_integer_to_float_audio(&samples, &mut audio).map_err(|err| {
        AppError::new(
            "local_whisper_audio_conversion_error",
            format!("Could not prepare PCM for Whisper: {err}"),
        )
    })?;
    let mut state = context.create_state().map_err(|err| {
        AppError::new(
            "local_whisper_state_error",
            format!("Could not create Whisper inference state: {err}"),
        )
    })?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(language);
    params.set_translate(false);
    params.set_no_context(true);
    params.set_no_timestamps(true);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_n_threads(threads as i32);
    let abort_callback: Box<dyn FnMut() -> bool> =
        Box::new(move || cancellation.load(Ordering::SeqCst));
    params.set_abort_callback_safe::<Option<Box<dyn FnMut() -> bool>>, Box<dyn FnMut() -> bool>>(
        Some(abort_callback),
    );
    state.full(params, &audio).map_err(|err| {
        AppError::new(
            "local_whisper_inference_error",
            format!("Whisper inference failed: {err}"),
        )
    })?;
    let text = state
        .as_iter()
        .map(|segment| {
            segment
                .to_str_lossy()
                .map(|value| value.into_owned())
                .map_err(|err| {
                    AppError::new(
                        "local_whisper_text_error",
                        format!("Could not read Whisper text: {err}"),
                    )
                })
        })
        .collect::<AppResult<Vec<_>>>()?
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return Err(AppError::new(
            "local_whisper_no_speech",
            "Whisper did not detect speech in this utterance.",
        ));
    }
    Ok(text)
}

struct PcmSegmenter {
    minimum_speech_samples: usize,
    silence_to_commit_samples: usize,
    maximum_utterance_samples: usize,
    pre_roll_samples: usize,
    speech_threshold: f32,
    pre_roll: VecDeque<i16>,
    buffer: Vec<i16>,
    speech_samples: usize,
    silence_samples: usize,
    active: bool,
}

impl PcmSegmenter {
    fn new(config: &LocalTranslationConfig) -> Self {
        let sample_rate = config.sample_rate_hz as usize;
        Self {
            minimum_speech_samples: ms_to_samples(config.minimum_speech_ms, sample_rate),
            silence_to_commit_samples: ms_to_samples(config.silence_to_commit_ms, sample_rate),
            maximum_utterance_samples: ms_to_samples(config.maximum_utterance_ms, sample_rate),
            pre_roll_samples: ms_to_samples(config.pre_roll_ms, sample_rate),
            speech_threshold: config.speech_threshold,
            pre_roll: VecDeque::new(),
            buffer: Vec::new(),
            speech_samples: 0,
            silence_samples: 0,
            active: false,
        }
    }

    fn push(&mut self, samples: Vec<i16>) -> Option<Vec<i16>> {
        if samples.is_empty() {
            return None;
        }
        let speech = chunk_has_speech(&samples, self.speech_threshold);
        if !self.active {
            if !speech {
                self.push_pre_roll(&samples);
                return None;
            }
            self.active = true;
            self.buffer.extend(self.pre_roll.drain(..));
            self.buffer.extend_from_slice(&samples);
            self.speech_samples = samples.len();
            self.silence_samples = 0;
        } else {
            self.buffer.extend_from_slice(&samples);
            if speech {
                self.speech_samples = self.speech_samples.saturating_add(samples.len());
                self.silence_samples = 0;
            } else {
                self.silence_samples = self.silence_samples.saturating_add(samples.len());
            }
        }

        if self.buffer.len() >= self.maximum_utterance_samples {
            return self.take_if_valid();
        }
        if self.silence_samples >= self.silence_to_commit_samples {
            if self.speech_samples >= self.minimum_speech_samples {
                return self.take_if_valid();
            }
            self.reset_active();
        }
        None
    }

    fn flush(&mut self) -> Option<Vec<i16>> {
        if self.active && self.speech_samples >= self.minimum_speech_samples {
            return self.take_if_valid();
        }
        self.reset_active();
        None
    }

    fn take_if_valid(&mut self) -> Option<Vec<i16>> {
        if self.speech_samples < self.minimum_speech_samples {
            self.reset_active();
            return None;
        }
        let segment = std::mem::take(&mut self.buffer);
        self.reset_active();
        Some(segment)
    }

    fn reset_active(&mut self) {
        self.buffer.clear();
        self.speech_samples = 0;
        self.silence_samples = 0;
        self.active = false;
        self.pre_roll.clear();
    }

    fn push_pre_roll(&mut self, samples: &[i16]) {
        if self.pre_roll_samples == 0 {
            return;
        }
        self.pre_roll.extend(samples.iter().copied());
        while self.pre_roll.len() > self.pre_roll_samples {
            self.pre_roll.pop_front();
        }
    }
}

fn ms_to_samples(milliseconds: u64, sample_rate: usize) -> usize {
    (milliseconds as usize)
        .saturating_mul(sample_rate)
        .saturating_div(1_000)
}

fn rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let energy = samples
        .iter()
        .map(|sample| {
            let value = *sample as f64 / i16::MAX as f64;
            value * value
        })
        .sum::<f64>()
        / samples.len() as f64;
    energy.sqrt() as f32
}

fn peak(samples: &[i16]) -> f32 {
    samples
        .iter()
        .map(|sample| f32::from(sample.unsigned_abs()) / f32::from(i16::MAX as u16))
        .fold(0.0_f32, f32::max)
}

fn chunk_has_speech(samples: &[i16], speech_threshold: f32) -> bool {
    rms(samples) >= speech_threshold || peak(samples) >= speech_threshold * 2.0
}

async fn drain_worker(
    worker: &mut Option<JoinHandle<AppResult<()>>>,
    cancellation: &Arc<AtomicBool>,
) -> AppResult<()> {
    drain_worker_with_timeout(
        worker,
        cancellation,
        Duration::from_secs(STOP_DRAIN_TIMEOUT_SECONDS),
        Duration::from_secs(CANCELLATION_GRACE_SECONDS),
    )
    .await
}

async fn drain_worker_with_timeout(
    worker: &mut Option<JoinHandle<AppResult<()>>>,
    cancellation: &Arc<AtomicBool>,
    drain_timeout: Duration,
    cancellation_grace: Duration,
) -> AppResult<()> {
    let Some(mut worker) = worker.take() else {
        return Ok(());
    };
    match timeout(drain_timeout, &mut worker).await {
        Ok(joined) => {
            joined.map_err(|err| AppError::new("local_translation_join_error", err.to_string()))?
        }
        Err(_) => {
            cancellation.store(true, Ordering::SeqCst);
            match timeout(cancellation_grace, &mut worker).await {
                Ok(joined) => joined.map_err(|err| {
                    AppError::new("local_translation_join_error", err.to_string())
                })?,
                Err(_) => {
                    worker.abort();
                    let _ = worker.await;
                    Ok(())
                }
            }
        }
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

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(86_400_000) as u64
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

    fn config() -> LocalTranslationConfig {
        LocalTranslationConfig {
            minimum_speech_ms: 100,
            silence_to_commit_ms: 200,
            maximum_utterance_ms: 1_000,
            pre_roll_ms: 100,
            speech_threshold: 0.01,
            ..LocalTranslationConfig::default()
        }
    }

    fn samples(milliseconds: usize, value: i16) -> Vec<i16> {
        vec![value; milliseconds * 16]
    }

    #[test]
    fn commits_after_trailing_silence_with_pre_roll() {
        let mut segmenter = PcmSegmenter::new(&config());
        assert!(segmenter.push(samples(100, 0)).is_none());
        assert!(segmenter.push(samples(100, 2_000)).is_none());
        let segment = segmenter.push(samples(200, 0)).unwrap();
        assert_eq!(segment.len(), samples(400, 0).len());
    }

    #[test]
    fn ignores_short_noise_bursts() {
        let mut segmenter = PcmSegmenter::new(&config());
        assert!(segmenter.push(samples(50, 2_000)).is_none());
        assert!(segmenter.push(samples(200, 0)).is_none());
        assert!(segmenter.flush().is_none());
    }

    #[test]
    fn commits_peak_led_speech_that_is_visible_on_the_input_meter() {
        let mut segmenter = PcmSegmenter::new(&config());
        let mut speech = samples(100, 0);
        for index in (0..speech.len()).step_by(160) {
            speech[index] = 800;
        }
        assert!(rms(&speech) < 0.01);
        assert!(peak(&speech) > 0.02);

        assert!(segmenter.push(speech.clone()).is_none());
        assert!(segmenter.push(speech).is_none());
        assert!(segmenter.push(samples(200, 0)).is_some());
    }

    #[test]
    fn manual_flush_commits_valid_speech() {
        let mut segmenter = PcmSegmenter::new(&config());
        assert!(segmenter.push(samples(120, 2_000)).is_none());
        assert!(segmenter.flush().is_some());
        assert!(segmenter.flush().is_none());
    }

    #[test]
    fn maximum_duration_forces_a_boundary() {
        let mut segmenter = PcmSegmenter::new(&config());
        let segment = segmenter.push(samples(1_000, 2_000)).unwrap();
        assert_eq!(segment.len(), 16_000);
    }

    #[test]
    fn snapshot_upsert_ignores_stale_revisions_and_keeps_order() {
        let first = TranscriptItem {
            id: "first".to_string(),
            timestamp_ms: 1,
            source_text: "一".to_string(),
            translated_text: String::new(),
            status: TranscriptStatus::Partial,
            latency_ms: None,
            revision: 1,
            update_mode: TranscriptUpdateMode::Snapshot,
            error_message: None,
        };
        let second = TranscriptItem {
            id: "second".to_string(),
            ..first.clone()
        };
        let mut transcript = vec![first.clone(), second];
        assert!(!upsert_snapshot(&mut transcript, first.clone()));
        assert!(upsert_snapshot(
            &mut transcript,
            TranscriptItem {
                translated_text: "Một".to_string(),
                status: TranscriptStatus::Final,
                revision: 2,
                ..first
            }
        ));
        assert_eq!(transcript.len(), 2);
        assert_eq!(transcript[0].translated_text, "Một");
        assert_eq!(transcript[1].id, "second");
    }

    #[test]
    fn generation_change_cancels_late_mutations() {
        let generation = Arc::new(AtomicU64::new(7));
        assert!(is_generation_active(7, &generation));
        generation.store(8, Ordering::SeqCst);
        assert!(!is_generation_active(7, &generation));
    }

    #[test]
    fn cancellation_flag_deactivates_worker() {
        let generation = Arc::new(AtomicU64::new(7));
        let cancellation = Arc::new(AtomicBool::new(false));
        assert!(is_worker_active(7, &generation, &cancellation));
        cancellation.store(true, Ordering::SeqCst);
        assert!(!is_worker_active(7, &generation, &cancellation));
    }

    #[test]
    fn translated_audio_level_reports_pcm_energy() {
        let level = translated_audio_level(&[0, i16::MAX, -i16::MAX]);
        assert_eq!(level.sample_count, 3);
        assert!((level.peak - 1.0).abs() < f32::EPSILON);
        assert!((level.rms - (2.0_f32 / 3.0).sqrt()).abs() < 0.0001);

        let silence = translated_audio_level(&[]);
        assert_eq!(silence.sample_count, 0);
        assert_eq!(silence.peak, 0.0);
        assert_eq!(silence.rms, 0.0);
    }

    #[test]
    fn speech_activity_has_priority_over_overlapping_translation() {
        let activity = PipelineActivity::default();
        activity
            .translation_stage
            .store(TRANSLATION_TRANSCRIBING, Ordering::SeqCst);
        assert_eq!(
            pipeline_activity_state(&activity),
            (SessionStatus::Listening, "transcribing")
        );

        activity
            .speech_stage
            .store(SPEECH_PLAYING, Ordering::SeqCst);
        assert_eq!(
            pipeline_activity_state(&activity),
            (SessionStatus::Speaking, "speaking")
        );

        activity
            .translation_stage
            .store(TRANSLATION_TRANSLATING, Ordering::SeqCst);
        assert_eq!(
            pipeline_activity_state(&activity),
            (SessionStatus::Speaking, "speaking")
        );
    }

    #[tokio::test]
    async fn fast_worker_completes_during_graceful_drain() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let mut worker = Some(tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(())
        }));

        drain_worker_with_timeout(
            &mut worker,
            &cancellation,
            Duration::from_millis(100),
            Duration::from_millis(100),
        )
        .await
        .unwrap();

        assert!(worker.is_none());
        assert!(!cancellation.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn slow_worker_is_cooperatively_cancelled_after_drain_deadline() {
        let cancellation = Arc::new(AtomicBool::new(false));
        let worker_cancellation = cancellation.clone();
        let mut worker = Some(tokio::spawn(async move {
            while !worker_cancellation.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
            Ok(())
        }));

        drain_worker_with_timeout(
            &mut worker,
            &cancellation,
            Duration::from_millis(10),
            Duration::from_millis(100),
        )
        .await
        .unwrap();

        assert!(worker.is_none());
        assert!(cancellation.load(Ordering::SeqCst));
    }

    #[tokio::test]
    #[ignore = "requires BAKA_TRANS_WHISPER_MODEL, BAKA_TRANS_OLLAMA_MODEL, BAKA_TRANS_JAPANESE_PCM, and local Ollama"]
    async fn local_whisper_ollama_end_to_end_smoke_test() {
        let model_path = std::env::var("BAKA_TRANS_WHISPER_MODEL").unwrap();
        let ollama_model = std::env::var("BAKA_TRANS_OLLAMA_MODEL").unwrap();
        let pcm_path = std::env::var("BAKA_TRANS_JAPANESE_PCM").unwrap();
        let bytes = std::fs::read(pcm_path).unwrap();
        assert_eq!(
            bytes.len() % 2,
            0,
            "PCM fixture must contain little-endian i16 samples"
        );
        let pcm = bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let context = crate::local_translation::load_whisper_context(&model_path, false).unwrap();
        let source = transcribe(
            &context,
            pcm,
            4,
            Some("ja"),
            Arc::new(AtomicBool::new(false)),
        )
        .unwrap();
        assert!(!source.is_empty());
        let local_config = LocalTranslationConfig {
            model: ollama_model,
            model_path,
            ..LocalTranslationConfig::default()
        };
        let (translated, _) = crate::local_translation::OllamaClient::new(
            &local_config,
            Language::Ja,
            Language::Vi,
        )
        .unwrap()
        .translate(&source)
        .await
        .unwrap();
        assert!(!translated.is_empty());
        println!("Japanese: {source}\nVietnamese: {translated}");
    }
}
