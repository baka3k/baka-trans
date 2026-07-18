use crate::error::{AppError, AppResult};
use crate::models::{
    AudioDeviceInfo, AudioDevices, AudioLevelEvent, AudioOutputChannel, DeviceKind,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};
use std::collections::VecDeque;
use std::f32::consts::PI;
use std::sync::mpsc as std_mpsc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

pub const OPENAI_REALTIME_SAMPLE_RATE: u32 = 24_000;
pub const GOOGLE_LIVE_INPUT_SAMPLE_RATE: u32 = 16_000;
pub const GOOGLE_LIVE_OUTPUT_SAMPLE_RATE: u32 = 24_000;
const AUDIO_LEVEL_EVENT_INTERVAL: Duration = Duration::from_millis(50);

#[cfg(target_os = "windows")]
const LOOPBACK_DEVICE_PREFIX: &str = "loopback";

pub struct CaptureRuntime {
    stop_tx: std_mpsc::Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
}

pub struct PlaybackRuntime {
    audio_tx: std_mpsc::SyncSender<Vec<i16>>,
    stop_tx: std_mpsc::Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
}

pub struct TestToneRuntime {
    stop_tx: std_mpsc::Sender<()>,
    join_handle: Option<thread::JoinHandle<()>>,
}

struct AudioLevelEventThrottle {
    interval: Duration,
    last_emitted_at: Option<Instant>,
}

impl AudioLevelEventThrottle {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_emitted_at: None,
        }
    }

    fn should_emit(&mut self) -> bool {
        self.should_emit_at(Instant::now())
    }

    fn should_emit_at(&mut self, now: Instant) -> bool {
        if self
            .last_emitted_at
            .is_some_and(|last| now.saturating_duration_since(last) < self.interval)
        {
            return false;
        }
        self.last_emitted_at = Some(now);
        true
    }
}

impl PlaybackRuntime {
    pub fn sender(&self) -> std_mpsc::SyncSender<Vec<i16>> {
        self.audio_tx.clone()
    }
}

impl Drop for PlaybackRuntime {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl Drop for CaptureRuntime {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl Drop for TestToneRuntime {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

pub fn list_devices() -> AppResult<AudioDevices> {
    let host = cpal::default_host();
    let default_input = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let default_output = host
        .default_output_device()
        .and_then(|device| device.name().ok());

    let mut inputs = host
        .input_devices()?
        .enumerate()
        .map(|(index, device)| {
            device_info(index, device, DeviceKind::Input, default_input.as_deref())
        })
        .collect::<Vec<_>>();

    let outputs = host
        .output_devices()?
        .enumerate()
        .map(|(index, device)| {
            device_info(index, device, DeviceKind::Output, default_output.as_deref())
        })
        .collect::<Vec<_>>();

    #[cfg(target_os = "windows")]
    {
        let loopback_outputs = host
            .output_devices()?
            .enumerate()
            .map(|(index, device)| loopback_device_info(index, device, default_output.as_deref()))
            .collect::<Vec<_>>();
        inputs.splice(0..0, loopback_outputs);
    }

    Ok(AudioDevices { inputs, outputs })
}

pub fn start_capture(
    app: AppHandle,
    input_device_id: &str,
    monitor_tx: Option<std_mpsc::SyncSender<Vec<i16>>>,
) -> AppResult<(CaptureRuntime, mpsc::Receiver<Vec<i16>>)> {
    start_capture_at_sample_rate(
        app,
        input_device_id,
        monitor_tx,
        OPENAI_REALTIME_SAMPLE_RATE,
    )
}

pub fn start_capture_at_sample_rate(
    app: AppHandle,
    input_device_id: &str,
    monitor_tx: Option<std_mpsc::SyncSender<Vec<i16>>>,
    target_sample_rate: u32,
) -> AppResult<(CaptureRuntime, mpsc::Receiver<Vec<i16>>)> {
    let (tx, rx) = mpsc::channel::<Vec<i16>>(24);
    let (stop_tx, stop_rx) = std_mpsc::channel::<()>();
    let (ready_tx, ready_rx) = std_mpsc::channel::<AppResult<()>>();
    let device_id = input_device_id.to_string();
    let thread_device_id = device_id.clone();

    let join_handle = thread::spawn(move || {
        run_capture_thread(
            app,
            tx,
            monitor_tx,
            thread_device_id,
            target_sample_rate,
            stop_rx,
            ready_tx,
        )
    });

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok((
            CaptureRuntime {
                stop_tx,
                join_handle: Some(join_handle),
            },
            rx,
        )),
        Ok(Err(error)) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(AppError::new(
                "audio_capture_error",
                "Timed out while starting the input stream.",
            ))
        }
    }
}

pub fn start_test_tone(
    output_device_id: &str,
    output_channel: AudioOutputChannel,
) -> AppResult<TestToneRuntime> {
    let (stop_tx, stop_rx) = std_mpsc::channel::<()>();
    let (ready_tx, ready_rx) = std_mpsc::channel::<AppResult<()>>();
    let device_id = output_device_id.to_string();

    let join_handle =
        thread::spawn(move || run_test_tone_thread(device_id, output_channel, stop_rx, ready_tx));

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(TestToneRuntime {
            stop_tx,
            join_handle: Some(join_handle),
        }),
        Ok(Err(error)) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(AppError::new(
                "audio_playback_error",
                "Timed out while starting the test tone.",
            ))
        }
    }
}

fn run_test_tone_thread(
    output_device_id: String,
    output_channel: AudioOutputChannel,
    stop_rx: std_mpsc::Receiver<()>,
    ready_tx: std_mpsc::Sender<AppResult<()>>,
) {
    let device = match find_device(&output_device_id, DeviceKind::Output) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    let supported = match device.default_output_config() {
        Ok(config) => config,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };
    let channels = supported.channels() as usize;
    let sample_rate = supported.sample_rate().0 as f32;
    let config: StreamConfig = supported.clone().into();
    let mut clock = 0f32;
    let err_fn = |err| eprintln!("Output stream error: {err}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                fill_tone(data, channels, output_channel, sample_rate, &mut clock)
            },
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_output_stream(
            &config,
            move |data: &mut [i16], _| {
                let mut buffer = vec![0.0; data.len()];
                fill_tone(
                    &mut buffer,
                    channels,
                    output_channel,
                    sample_rate,
                    &mut clock,
                );
                for (out, sample) in data.iter_mut().zip(buffer) {
                    *out = (sample * i16::MAX as f32) as i16;
                }
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_output_stream(
            &config,
            move |data: &mut [u16], _| {
                let mut buffer = vec![0.0; data.len()];
                fill_tone(
                    &mut buffer,
                    channels,
                    output_channel,
                    sample_rate,
                    &mut clock,
                );
                for (out, sample) in data.iter_mut().zip(buffer) {
                    *out = (((sample + 1.0) * 0.5) * u16::MAX as f32) as u16;
                }
            },
            err_fn,
            None,
        ),
        other => {
            let _ = ready_tx.send(Err(AppError::new(
                "unsupported_audio_format",
                format!("Unsupported output sample format: {other:?}"),
            )));
            return;
        }
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };

    if let Err(error) = stream.play() {
        let _ = ready_tx.send(Err(error.into()));
        return;
    }

    let _ = ready_tx.send(Ok(()));
    let _ = stop_rx.recv();
    drop(stream);
}

pub fn start_playback_with_channel(
    app: AppHandle,
    output_device_id: &str,
    output_channel: AudioOutputChannel,
) -> AppResult<PlaybackRuntime> {
    start_playback_with_channel_at_sample_rate(
        app,
        output_device_id,
        output_channel,
        OPENAI_REALTIME_SAMPLE_RATE,
    )
}

pub fn start_playback_with_channel_at_sample_rate(
    app: AppHandle,
    output_device_id: &str,
    output_channel: AudioOutputChannel,
    source_sample_rate: u32,
) -> AppResult<PlaybackRuntime> {
    let (audio_tx, audio_rx) = std_mpsc::sync_channel::<Vec<i16>>(24);
    let (stop_tx, stop_rx) = std_mpsc::channel::<()>();
    let (ready_tx, ready_rx) = std_mpsc::channel::<AppResult<()>>();
    let device_id = output_device_id.to_string();

    let join_handle = thread::spawn(move || {
        run_playback_thread(
            app,
            device_id,
            output_channel,
            source_sample_rate,
            audio_rx,
            stop_rx,
            ready_tx,
        )
    });

    match ready_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(())) => Ok(PlaybackRuntime {
            audio_tx,
            stop_tx,
            join_handle: Some(join_handle),
        }),
        Ok(Err(error)) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = stop_tx.send(());
            let _ = join_handle.join();
            Err(AppError::new(
                "audio_playback_error",
                "Timed out while starting the output stream.",
            ))
        }
    }
}

pub fn pcm16_to_le_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn handle_input(
    data: &[f32],
    channels: usize,
    sample_rate: u32,
    tx: &mpsc::Sender<Vec<i16>>,
    monitor_tx: Option<&std_mpsc::SyncSender<Vec<i16>>>,
    level_event_throttle: &mut AudioLevelEventThrottle,
    app: &AppHandle,
    input_device_id: &str,
    target_sample_rate: u32,
) {
    if data.is_empty() || channels == 0 {
        return;
    }

    let mono = downmix_to_mono(data, channels);
    let (rms, peak) = level(&mono);
    let resampled = resample_linear(&mono, sample_rate, target_sample_rate);
    let pcm = f32_to_pcm16(&resampled);
    if let Some(monitor_tx) = monitor_tx {
        let _ = tx.try_send(pcm.clone());
        let _ = monitor_tx.try_send(pcm);
    } else {
        let _ = tx.try_send(pcm);
    }
    if level_event_throttle.should_emit() {
        let _ = app.emit(
            "audio-level",
            AudioLevelEvent {
                input_device_id: input_device_id.to_string(),
                rms,
                peak,
            },
        );
    }
}

fn run_capture_thread(
    app: AppHandle,
    tx: mpsc::Sender<Vec<i16>>,
    monitor_tx: Option<std_mpsc::SyncSender<Vec<i16>>>,
    device_id: String,
    target_sample_rate: u32,
    stop_rx: std_mpsc::Receiver<()>,
    ready_tx: std_mpsc::Sender<AppResult<()>>,
) {
    #[cfg(target_os = "windows")]
    let is_loopback = device_id.starts_with(&format!("{LOOPBACK_DEVICE_PREFIX}:"));
    #[cfg(not(target_os = "windows"))]
    let is_loopback = false;

    let device_kind = if is_loopback {
        DeviceKind::Output
    } else {
        DeviceKind::Input
    };
    let device = match find_device(&device_id, device_kind) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    let supported = match if is_loopback {
        device.default_output_config()
    } else {
        device.default_input_config()
    } {
        Ok(config) => config,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let config: StreamConfig = supported.clone().into();
    let error_app = app.clone();
    let err_fn = move |err| {
        let _ = error_app.emit(
            "app-error",
            AppError::new("audio_capture_error", format!("Input stream error: {err}")),
        );
    };

    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            let app = app.clone();
            let tx = tx.clone();
            let device_id = device_id.clone();
            let mut level_event_throttle = AudioLevelEventThrottle::new(AUDIO_LEVEL_EVENT_INTERVAL);
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    handle_input(
                        data,
                        channels,
                        sample_rate,
                        &tx,
                        monitor_tx.as_ref(),
                        &mut level_event_throttle,
                        &app,
                        &device_id,
                        target_sample_rate,
                    )
                },
                err_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let app = app.clone();
            let tx = tx.clone();
            let device_id = device_id.clone();
            let mut level_event_throttle = AudioLevelEventThrottle::new(AUDIO_LEVEL_EVENT_INTERVAL);
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let samples = data
                        .iter()
                        .map(|v| *v as f32 / i16::MAX as f32)
                        .collect::<Vec<_>>();
                    handle_input(
                        &samples,
                        channels,
                        sample_rate,
                        &tx,
                        monitor_tx.as_ref(),
                        &mut level_event_throttle,
                        &app,
                        &device_id,
                        target_sample_rate,
                    );
                },
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let app = app.clone();
            let tx = tx.clone();
            let device_id = device_id.clone();
            let mut level_event_throttle = AudioLevelEventThrottle::new(AUDIO_LEVEL_EVENT_INTERVAL);
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let samples = data
                        .iter()
                        .map(|v| (*v as f32 / u16::MAX as f32) * 2.0 - 1.0)
                        .collect::<Vec<_>>();
                    handle_input(
                        &samples,
                        channels,
                        sample_rate,
                        &tx,
                        monitor_tx.as_ref(),
                        &mut level_event_throttle,
                        &app,
                        &device_id,
                        target_sample_rate,
                    );
                },
                err_fn,
                None,
            )
        }
        other => {
            let _ = ready_tx.send(Err(AppError::new(
                "unsupported_audio_format",
                format!("Unsupported input sample format: {other:?}"),
            )));
            return;
        }
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };

    if let Err(error) = stream.play() {
        let _ = ready_tx.send(Err(error.into()));
        return;
    }

    let _ = ready_tx.send(Ok(()));
    let _ = stop_rx.recv();
    drop(stream);
}

fn run_playback_thread(
    app: AppHandle,
    device_id: String,
    output_channel: AudioOutputChannel,
    source_sample_rate: u32,
    audio_rx: std_mpsc::Receiver<Vec<i16>>,
    stop_rx: std_mpsc::Receiver<()>,
    ready_tx: std_mpsc::Sender<AppResult<()>>,
) {
    let device = match find_device(&device_id, DeviceKind::Output) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return;
        }
    };
    let supported = match device.default_output_config() {
        Ok(config) => config,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };
    let channels = supported.channels() as usize;
    let output_sample_rate = supported.sample_rate().0;
    let config: StreamConfig = supported.clone().into();
    let err_fn = move |err| {
        let _ = app.emit(
            "app-error",
            AppError::new(
                "audio_playback_error",
                format!("Output stream error: {err}"),
            ),
        );
    };

    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            let mut queue = VecDeque::new();
            device.build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    fill_output_f32(
                        data,
                        channels,
                        output_channel,
                        source_sample_rate,
                        output_sample_rate,
                        &audio_rx,
                        &mut queue,
                    )
                },
                err_fn,
                None,
            )
        }
        SampleFormat::I16 => {
            let mut queue = VecDeque::new();
            device.build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    fill_output_i16(
                        data,
                        channels,
                        output_channel,
                        source_sample_rate,
                        output_sample_rate,
                        &audio_rx,
                        &mut queue,
                    )
                },
                err_fn,
                None,
            )
        }
        SampleFormat::U16 => {
            let mut queue = VecDeque::new();
            device.build_output_stream(
                &config,
                move |data: &mut [u16], _| {
                    fill_output_u16(
                        data,
                        channels,
                        output_channel,
                        source_sample_rate,
                        output_sample_rate,
                        &audio_rx,
                        &mut queue,
                    )
                },
                err_fn,
                None,
            )
        }
        other => {
            let _ = ready_tx.send(Err(AppError::new(
                "unsupported_audio_format",
                format!("Unsupported output sample format: {other:?}"),
            )));
            return;
        }
    };

    let stream = match stream {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(error.into()));
            return;
        }
    };

    if let Err(error) = stream.play() {
        let _ = ready_tx.send(Err(error.into()));
        return;
    }

    let _ = ready_tx.send(Ok(()));
    let _ = stop_rx.recv();
    drop(stream);
}

fn fill_output_f32(
    data: &mut [f32],
    channels: usize,
    output_channel: AudioOutputChannel,
    source_sample_rate: u32,
    output_sample_rate: u32,
    rx: &std_mpsc::Receiver<Vec<i16>>,
    queue: &mut VecDeque<i16>,
) {
    refill_output_queue(
        rx,
        queue,
        data.len() / channels,
        source_sample_rate,
        output_sample_rate,
    );
    for frame in data.chunks_mut(channels) {
        let sample = queue.pop_front().unwrap_or_default() as f32 / i16::MAX as f32;
        write_frame_f32(frame, output_channel, sample);
    }
}

fn fill_output_i16(
    data: &mut [i16],
    channels: usize,
    output_channel: AudioOutputChannel,
    source_sample_rate: u32,
    output_sample_rate: u32,
    rx: &std_mpsc::Receiver<Vec<i16>>,
    queue: &mut VecDeque<i16>,
) {
    refill_output_queue(
        rx,
        queue,
        data.len() / channels,
        source_sample_rate,
        output_sample_rate,
    );
    for frame in data.chunks_mut(channels) {
        let sample = queue.pop_front().unwrap_or_default();
        write_frame_i16(frame, output_channel, sample);
    }
}

fn fill_output_u16(
    data: &mut [u16],
    channels: usize,
    output_channel: AudioOutputChannel,
    source_sample_rate: u32,
    output_sample_rate: u32,
    rx: &std_mpsc::Receiver<Vec<i16>>,
    queue: &mut VecDeque<i16>,
) {
    refill_output_queue(
        rx,
        queue,
        data.len() / channels,
        source_sample_rate,
        output_sample_rate,
    );
    for frame in data.chunks_mut(channels) {
        let sample = queue.pop_front().unwrap_or_default();
        write_frame_u16(frame, output_channel, pcm16_to_u16(sample));
    }
}

fn refill_output_queue(
    rx: &std_mpsc::Receiver<Vec<i16>>,
    queue: &mut VecDeque<i16>,
    target_frames: usize,
    source_sample_rate: u32,
    output_sample_rate: u32,
) {
    while queue.len() < target_frames * 3 {
        match rx.try_recv() {
            Ok(chunk) => queue.extend(resample_pcm16_chunk(
                &chunk,
                source_sample_rate,
                output_sample_rate,
            )),
            Err(_) => break,
        }
    }
}

fn resample_pcm16_chunk(samples: &[i16], from_rate: u32, to_rate: u32) -> Vec<i16> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let f32_samples = samples
        .iter()
        .map(|sample| *sample as f32 / i16::MAX as f32)
        .collect::<Vec<_>>();
    let resampled = resample_linear(&f32_samples, from_rate, to_rate);
    f32_to_pcm16(&resampled)
}

fn downmix_to_mono(data: &[f32], channels: usize) -> Vec<f32> {
    data.chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f32 / to_rate as f32;
    let output_len = ((samples.len() as f32) / ratio).ceil() as usize;
    (0..output_len)
        .map(|index| {
            let src_pos = index as f32 * ratio;
            let left = src_pos.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let frac = src_pos - left as f32;
            samples[left] * (1.0 - frac) + samples[right] * frac
        })
        .collect()
}

fn f32_to_pcm16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect()
}

fn level(samples: &[f32]) -> (f32, f32) {
    let mut peak = 0.0f32;
    let mut sum = 0.0f32;
    for sample in samples {
        let abs = sample.abs();
        peak = peak.max(abs);
        sum += sample * sample;
    }
    let rms = if samples.is_empty() {
        0.0
    } else {
        (sum / samples.len() as f32).sqrt()
    };
    (rms, peak)
}

fn fill_tone(
    data: &mut [f32],
    channels: usize,
    output_channel: AudioOutputChannel,
    sample_rate: f32,
    clock: &mut f32,
) {
    for frame in data.chunks_mut(channels) {
        let value = (*clock * 440.0 * 2.0 * PI / sample_rate).sin() * 0.16;
        *clock += 1.0;
        write_frame_f32(frame, output_channel, value);
    }
}

fn write_frame_f32(frame: &mut [f32], output_channel: AudioOutputChannel, sample: f32) {
    write_frame_by_channel(frame, output_channel, sample, 0.0);
}

fn write_frame_i16(frame: &mut [i16], output_channel: AudioOutputChannel, sample: i16) {
    write_frame_by_channel(frame, output_channel, sample, 0);
}

fn write_frame_u16(frame: &mut [u16], output_channel: AudioOutputChannel, sample: u16) {
    write_frame_by_channel(frame, output_channel, sample, pcm16_to_u16(0));
}

fn write_frame_by_channel<T: Copy>(
    frame: &mut [T],
    output_channel: AudioOutputChannel,
    sample: T,
    silence: T,
) {
    match output_channel {
        AudioOutputChannel::All => {
            for out in frame {
                *out = sample;
            }
        }
        AudioOutputChannel::Left => {
            for out in frame.iter_mut() {
                *out = silence;
            }
            if let Some(out) = frame.first_mut() {
                *out = sample;
            }
        }
        AudioOutputChannel::Right => {
            for out in frame.iter_mut() {
                *out = silence;
            }
            let index = if frame.len() > 1 { 1 } else { 0 };
            if let Some(out) = frame.get_mut(index) {
                *out = sample;
            }
        }
    }
}

fn pcm16_to_u16(sample: i16) -> u16 {
    ((sample as i32 + i16::MAX as i32 + 1) as f32 / (u16::MAX as f32 + 1.0) * u16::MAX as f32)
        as u16
}

fn device_info(
    index: usize,
    device: Device,
    kind: DeviceKind,
    default_name: Option<&str>,
) -> AudioDeviceInfo {
    let name = device
        .name()
        .unwrap_or_else(|_| "Unknown audio device".to_string());
    let mut min_sample_rate = None;
    let mut max_sample_rate = None;
    let mut max_channels = None;

    match kind {
        DeviceKind::Input => {
            if let Ok(configs) = device.supported_input_configs() {
                for config in configs {
                    update_config_stats(
                        &mut min_sample_rate,
                        &mut max_sample_rate,
                        &mut max_channels,
                        config.min_sample_rate().0,
                        config.max_sample_rate().0,
                        config.channels(),
                    );
                }
            }
        }
        DeviceKind::Output => {
            if let Ok(configs) = device.supported_output_configs() {
                for config in configs {
                    update_config_stats(
                        &mut min_sample_rate,
                        &mut max_sample_rate,
                        &mut max_channels,
                        config.min_sample_rate().0,
                        config.max_sample_rate().0,
                        config.channels(),
                    );
                }
            }
        }
    }

    let prefix = match kind {
        DeviceKind::Input => "input",
        DeviceKind::Output => "output",
    };

    AudioDeviceInfo {
        id: format!("{prefix}:{index}:{name}"),
        name: name.clone(),
        kind,
        is_default: default_name == Some(name.as_str()),
        min_sample_rate,
        max_sample_rate,
        max_channels,
    }
}

#[cfg(target_os = "windows")]
fn loopback_device_info(
    index: usize,
    device: Device,
    default_name: Option<&str>,
) -> AudioDeviceInfo {
    let name = device
        .name()
        .unwrap_or_else(|_| "Unknown output device".to_string());
    let mut min_sample_rate = None;
    let mut max_sample_rate = None;
    let mut max_channels = None;
    if let Ok(configs) = device.supported_output_configs() {
        for config in configs {
            update_config_stats(
                &mut min_sample_rate,
                &mut max_sample_rate,
                &mut max_channels,
                config.min_sample_rate().0,
                config.max_sample_rate().0,
                config.channels(),
            );
        }
    }

    AudioDeviceInfo {
        id: format!("{LOOPBACK_DEVICE_PREFIX}:{index}:{name}"),
        name: format!("Teams audio (system output) — {name}"),
        kind: DeviceKind::Input,
        is_default: default_name == Some(name.as_str()),
        min_sample_rate,
        max_sample_rate,
        max_channels,
    }
}

fn update_config_stats(
    min_sample_rate: &mut Option<u32>,
    max_sample_rate: &mut Option<u32>,
    max_channels: &mut Option<u16>,
    min_rate: u32,
    max_rate: u32,
    channels: u16,
) {
    *min_sample_rate = Some(match min_sample_rate {
        Some(current) => (*current).min(min_rate),
        None => min_rate,
    });
    *max_sample_rate = Some(match max_sample_rate {
        Some(current) => (*current).max(max_rate),
        None => max_rate,
    });
    *max_channels = Some(match max_channels {
        Some(current) => (*current).max(channels),
        None => channels,
    });
}

fn find_device(device_id: &str, kind: DeviceKind) -> AppResult<Device> {
    let host = cpal::default_host();
    let expected_name = device_id.splitn(3, ':').nth(2).unwrap_or(device_id);
    let devices = match kind {
        DeviceKind::Input => host.input_devices()?,
        DeviceKind::Output => host.output_devices()?,
    };

    for device in devices {
        if device.name().ok().as_deref() == Some(expected_name) {
            return Ok(device);
        }
    }

    Err(AppError::new(
        "audio_device_not_found",
        format!("Audio device not found: {expected_name}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn audio_level_events_are_rate_limited() {
        let started = Instant::now();
        let mut throttle = AudioLevelEventThrottle::new(Duration::from_millis(50));

        assert!(throttle.should_emit_at(started));
        assert!(!throttle.should_emit_at(started + Duration::from_millis(10)));
        assert!(!throttle.should_emit_at(started + Duration::from_millis(49)));
        assert!(throttle.should_emit_at(started + Duration::from_millis(50)));
        assert!(!throttle.should_emit_at(started + Duration::from_millis(75)));
        assert!(throttle.should_emit_at(started + Duration::from_millis(100)));
    }

    #[test]
    fn resamples_to_expected_length() {
        let input = vec![0.0; 48_000];
        let output = resample_linear(&input, 48_000, 24_000);
        assert_eq!(output.len(), 24_000);
    }

    #[test]
    fn downmixes_stereo() {
        let mono = downmix_to_mono(&[1.0, -1.0, 0.5, 0.5], 2);
        assert_eq!(mono, vec![0.0, 0.5]);
    }
}
