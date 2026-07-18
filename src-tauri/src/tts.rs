use crate::error::{AppError, AppResult};
use crate::models::{LocalTranslationConfig, LocalVoice};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const LOCAL_TTS_SAMPLE_RATE: u32 = 24_000;

#[derive(Debug, Clone, PartialEq)]
pub struct SynthesizedAudio {
    pub pcm16_mono: Vec<i16>,
    pub sample_rate_hz: u32,
}

pub fn list_voices() -> AppResult<Vec<LocalVoice>> {
    platform::list_voices()
}

pub fn voice_is_available(voice_id: &str) -> AppResult<bool> {
    let voice_id = voice_id.trim();
    Ok(!voice_id.is_empty() && list_voices()?.iter().any(|voice| voice.id == voice_id))
}

pub async fn synthesize(
    text: &str,
    config: &LocalTranslationConfig,
    cancelled: Arc<AtomicBool>,
) -> AppResult<SynthesizedAudio> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::new(
            "local_tts_empty_text",
            "There is no translated text to speak.",
        ));
    }
    if cancelled.load(Ordering::Acquire) {
        return Err(cancelled_error());
    }
    platform::synthesize(text, config, cancelled).await
}

fn cancelled_error() -> AppError {
    AppError::new(
        "local_tts_cancelled",
        "Local speech synthesis was cancelled.",
    )
}

fn decode_wav_pcm16(bytes: &[u8]) -> AppResult<SynthesizedAudio> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AppError::new(
            "local_tts_audio_format_error",
            "The system voice returned an unsupported audio container.",
        ));
    }

    let mut cursor = 12usize;
    let mut format = None;
    let mut data = None;
    while cursor.saturating_add(8) <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size = u32::from_le_bytes(
            bytes[cursor + 4..cursor + 8]
                .try_into()
                .map_err(|_| AppError::new("local_tts_audio_format_error", "Invalid WAV chunk."))?,
        ) as usize;
        cursor += 8;
        let end = cursor.checked_add(chunk_size).ok_or_else(|| {
            AppError::new("local_tts_audio_format_error", "Invalid WAV chunk size.")
        })?;
        if end > bytes.len() {
            return Err(AppError::new(
                "local_tts_audio_format_error",
                "The system voice returned a truncated WAV stream.",
            ));
        }
        if chunk_id == b"fmt " {
            format = Some(parse_wav_format(&bytes[cursor..end])?);
        } else if chunk_id == b"data" {
            data = Some(&bytes[cursor..end]);
        }
        cursor = end + (chunk_size & 1);
    }

    let (channels, sample_rate_hz, bits_per_sample) = format.ok_or_else(|| {
        AppError::new(
            "local_tts_audio_format_error",
            "The WAV stream has no format chunk.",
        )
    })?;
    if bits_per_sample != 16 || !(channels == 1 || channels == 2) || sample_rate_hz == 0 {
        return Err(AppError::new(
            "local_tts_audio_format_error",
            "Local speech requires PCM16 mono or stereo audio.",
        ));
    }
    let data = data.ok_or_else(|| {
        AppError::new(
            "local_tts_audio_format_error",
            "The WAV stream has no audio data.",
        )
    })?;
    if data.is_empty() {
        return Err(AppError::new(
            "local_tts_empty_audio",
            "The system voice returned an empty audio buffer.",
        ));
    }

    let raw = data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    let mono = if channels == 1 {
        raw
    } else {
        raw.chunks_exact(2)
            .map(|frame| ((i32::from(frame[0]) + i32::from(frame[1])) / 2) as i16)
            .collect()
    };
    let pcm16_mono = resample_linear(&mono, sample_rate_hz, LOCAL_TTS_SAMPLE_RATE);
    if pcm16_mono.is_empty() {
        return Err(AppError::new(
            "local_tts_empty_audio",
            "The system voice returned no playable audio samples.",
        ));
    }
    Ok(SynthesizedAudio {
        pcm16_mono,
        sample_rate_hz: LOCAL_TTS_SAMPLE_RATE,
    })
}

fn parse_wav_format(bytes: &[u8]) -> AppResult<(u16, u32, u16)> {
    if bytes.len() < 16 {
        return Err(AppError::new(
            "local_tts_audio_format_error",
            "The WAV format chunk is incomplete.",
        ));
    }
    let format_tag = u16::from_le_bytes([bytes[0], bytes[1]]);
    let channels = u16::from_le_bytes([bytes[2], bytes[3]]);
    let sample_rate_hz = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let bits_per_sample = u16::from_le_bytes([bytes[14], bytes[15]]);
    let pcm = format_tag == 1
        || (format_tag == 0xfffe
            && bytes.len() >= 40
            && bytes[24..40] == [1, 0, 0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113]);
    if !pcm {
        return Err(AppError::new(
            "local_tts_audio_format_error",
            "The system voice did not return uncompressed PCM audio.",
        ));
    }
    Ok((channels, sample_rate_hz, bits_per_sample))
}

fn resample_linear(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    if samples.is_empty() || source_rate == 0 || target_rate == 0 {
        return Vec::new();
    }
    if source_rate == target_rate {
        return samples.to_vec();
    }
    let output_len =
        ((samples.len() as u64 * target_rate as u64) / source_rate as u64).max(1) as usize;
    let ratio = source_rate as f64 / target_rate as f64;
    (0..output_len)
        .map(|index| {
            let position = index as f64 * ratio;
            let left = position.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = (position - left as f64) as f32;
            let sample = samples[left] as f32 * (1.0 - fraction) + samples[right] as f32 * fraction;
            sample.round().clamp(i16::MIN as f32, i16::MAX as f32) as i16
        })
        .collect()
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use windows::core::HSTRING;
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;
    use windows::Storage::Streams::DataReader;

    fn windows_error(code: &'static str, context: &str, error: windows::core::Error) -> AppError {
        AppError::new(code, format!("{context}: {error}"))
    }

    pub fn list_voices() -> AppResult<Vec<LocalVoice>> {
        let installed = SpeechSynthesizer::AllVoices().map_err(|error| {
            windows_error(
                "local_tts_voice_list_error",
                "Could not list Windows voices",
                error,
            )
        })?;
        let size = installed.Size().map_err(|error| {
            windows_error(
                "local_tts_voice_list_error",
                "Could not read Windows voices",
                error,
            )
        })?;
        let mut voices = Vec::with_capacity(size as usize);
        for index in 0..size {
            let voice = installed.GetAt(index).map_err(|error| {
                windows_error(
                    "local_tts_voice_list_error",
                    "Could not read a Windows voice",
                    error,
                )
            })?;
            voices.push(LocalVoice {
                id: voice
                    .Id()
                    .map_err(|error| {
                        windows_error(
                            "local_tts_voice_list_error",
                            "Could not read the voice id",
                            error,
                        )
                    })?
                    .to_string(),
                name: voice
                    .DisplayName()
                    .map_err(|error| {
                        windows_error(
                            "local_tts_voice_list_error",
                            "Could not read the voice name",
                            error,
                        )
                    })?
                    .to_string(),
                language: voice
                    .Language()
                    .map_err(|error| {
                        windows_error(
                            "local_tts_voice_list_error",
                            "Could not read the voice language",
                            error,
                        )
                    })?
                    .to_string(),
            });
        }
        voices.sort_by_key(|voice| {
            (
                !voice.language.to_ascii_lowercase().starts_with("vi"),
                voice.name.clone(),
            )
        });
        Ok(voices)
    }

    pub async fn synthesize(
        text: &str,
        config: &LocalTranslationConfig,
        cancelled: Arc<AtomicBool>,
    ) -> AppResult<SynthesizedAudio> {
        let synthesizer = SpeechSynthesizer::new().map_err(|error| {
            windows_error(
                "local_tts_start_error",
                "Could not start Windows speech synthesis",
                error,
            )
        })?;
        let voice = {
            let installed = SpeechSynthesizer::AllVoices().map_err(|error| {
                windows_error(
                    "local_tts_voice_list_error",
                    "Could not list Windows voices",
                    error,
                )
            })?;
            let mut selected = None;
            let selected_id = HSTRING::from(&config.voice_id);
            let size = installed.Size().map_err(|error| {
                windows_error(
                    "local_tts_voice_list_error",
                    "Could not read Windows voices",
                    error,
                )
            })?;
            for index in 0..size {
                let voice = installed.GetAt(index).map_err(|error| {
                    windows_error(
                        "local_tts_voice_list_error",
                        "Could not read a Windows voice",
                        error,
                    )
                })?;
                if voice.Id().map_err(|error| {
                    windows_error(
                        "local_tts_voice_list_error",
                        "Could not read the voice id",
                        error,
                    )
                })? == selected_id
                {
                    selected = Some(voice);
                    break;
                }
            }
            selected.ok_or_else(|| {
                AppError::new(
                    "local_tts_voice_missing",
                    "The selected Windows voice is no longer installed.",
                )
            })?
        };
        synthesizer.SetVoice(&voice).map_err(|error| {
            windows_error(
                "local_tts_voice_error",
                "Could not select the Windows voice",
                error,
            )
        })?;
        let options = synthesizer.Options().map_err(|error| {
            windows_error(
                "local_tts_options_error",
                "Could not configure Windows speech",
                error,
            )
        })?;
        options
            .SetSpeakingRate(config.tts_rate as f64)
            .map_err(|error| {
                windows_error(
                    "local_tts_options_error",
                    "Could not set speaking rate",
                    error,
                )
            })?;
        options
            .SetAudioVolume(config.tts_volume as f64)
            .map_err(|error| {
                windows_error(
                    "local_tts_options_error",
                    "Could not set voice volume",
                    error,
                )
            })?;
        let stream = synthesizer
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))
            .map_err(|error| {
                windows_error(
                    "local_tts_synthesis_error",
                    "Could not start speech synthesis",
                    error,
                )
            })?
            .await
            .map_err(|error| {
                windows_error(
                    "local_tts_synthesis_error",
                    "Windows speech synthesis failed",
                    error,
                )
            })?;
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        let size = stream.Size().map_err(|error| {
            windows_error(
                "local_tts_stream_error",
                "Could not read synthesized audio size",
                error,
            )
        })?;
        let count = u32::try_from(size).map_err(|_| {
            AppError::new("local_tts_stream_error", "Synthesized speech is too large.")
        })?;
        let reader = {
            let input = stream.GetInputStreamAt(0).map_err(|error| {
                windows_error(
                    "local_tts_stream_error",
                    "Could not open synthesized audio",
                    error,
                )
            })?;
            DataReader::CreateDataReader(&input).map_err(|error| {
                windows_error(
                    "local_tts_stream_error",
                    "Could not create synthesized audio reader",
                    error,
                )
            })?
        };
        let loaded = reader
            .LoadAsync(count)
            .map_err(|error| {
                windows_error(
                    "local_tts_stream_error",
                    "Could not load synthesized audio",
                    error,
                )
            })?
            .await
            .map_err(|error| {
                windows_error(
                    "local_tts_stream_error",
                    "Could not load synthesized audio",
                    error,
                )
            })?;
        let mut bytes = vec![0u8; loaded as usize];
        reader.ReadBytes(&mut bytes).map_err(|error| {
            windows_error(
                "local_tts_stream_error",
                "Could not read synthesized audio",
                error,
            )
        })?;
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_error());
        }
        decode_wav_pcm16(&bytes)
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use uuid::Uuid;

    pub fn list_voices() -> AppResult<Vec<LocalVoice>> {
        let output = Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .map_err(|error| {
                AppError::new(
                    "local_tts_voice_list_error",
                    format!("Could not list macOS voices: {error}"),
                )
            })?;
        if !output.status.success() {
            return Err(AppError::new(
                "local_tts_voice_list_error",
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let mut voices = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts = line.split_whitespace().collect::<Vec<_>>();
                let language_index = parts.iter().position(|part| part.contains('_'))?;
                let name = parts[..language_index].join(" ");
                let language = parts[language_index].replace('_', "-");
                (!name.is_empty()).then_some(LocalVoice {
                    id: name.clone(),
                    name,
                    language,
                })
            })
            .collect::<Vec<_>>();
        voices.sort_by_key(|voice| {
            (
                !voice.language.to_ascii_lowercase().starts_with("vi"),
                voice.name.clone(),
            )
        });
        Ok(voices)
    }

    pub async fn synthesize(
        text: &str,
        config: &LocalTranslationConfig,
        cancelled: Arc<AtomicBool>,
    ) -> AppResult<SynthesizedAudio> {
        let text = text.to_string();
        let voice = config.voice_id.clone();
        let rate = (175.0 * config.tts_rate).round().clamp(80.0, 350.0) as u32;
        let volume = config.tts_volume;
        tauri::async_runtime::spawn_blocking(move || {
            let path = std::env::temp_dir().join(format!("baka-trans-{}.wav", Uuid::new_v4()));
            let mut child = Command::new("say")
                .args(["-v", &voice, "-r", &rate.to_string(), "-o"])
                .arg(&path)
                .args(["--file-format=WAVE", "--data-format=LEI16@24000", &text])
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| {
                    AppError::new(
                        "local_tts_synthesis_error",
                        format!("Could not start macOS speech synthesis: {error}"),
                    )
                })?;
            loop {
                if cancelled.load(Ordering::Acquire) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&path);
                    return Err(cancelled_error());
                }
                if let Some(status) = child.try_wait().map_err(|error| {
                    AppError::new("local_tts_synthesis_error", error.to_string())
                })? {
                    if !status.success() {
                        let _ = std::fs::remove_file(&path);
                        return Err(AppError::new(
                            "local_tts_synthesis_error",
                            "macOS speech synthesis failed.",
                        ));
                    }
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }
            let bytes = std::fs::read(&path).map_err(|error| {
                AppError::new(
                    "local_tts_stream_error",
                    format!("Could not read macOS speech audio: {error}"),
                )
            })?;
            let _ = std::fs::remove_file(&path);
            let mut audio = decode_wav_pcm16(&bytes)?;
            if volume < 1.0 {
                for sample in &mut audio.pcm16_mono {
                    *sample = (*sample as f32 * volume).round() as i16;
                }
            }
            Ok(audio)
        })
        .await
        .map_err(|error| AppError::new("local_tts_join_error", error.to_string()))?
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod platform {
    use super::*;

    pub fn list_voices() -> AppResult<Vec<LocalVoice>> {
        Err(AppError::new(
            "local_tts_unsupported",
            "Local speech synthesis is supported on Windows and macOS.",
        ))
    }

    pub async fn synthesize(
        _text: &str,
        _config: &LocalTranslationConfig,
        _cancelled: Arc<AtomicBool>,
    ) -> AppResult<SynthesizedAudio> {
        Err(AppError::new(
            "local_tts_unsupported",
            "Local speech synthesis is supported on Windows and macOS.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm16_wav(channels: u16, sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let byte_rate = sample_rate * u32::from(channels) * 2;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        bytes.extend_from_slice(&byte_rate.to_le_bytes());
        bytes.extend_from_slice(&(channels * 2).to_le_bytes());
        bytes.extend_from_slice(&16u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn decodes_downmixes_and_resamples_pcm16_wav() {
        let wav = pcm16_wav(2, 48_000, &[1000, 3000, -1000, 1000, 2000, 4000, 0, 2000]);
        let audio = decode_wav_pcm16(&wav).expect("valid wav");
        assert_eq!(audio.sample_rate_hz, LOCAL_TTS_SAMPLE_RATE);
        assert_eq!(audio.pcm16_mono, vec![2000, 3000]);
    }

    #[test]
    fn rejects_empty_and_non_wav_audio() {
        let empty = pcm16_wav(1, 24_000, &[]);
        assert_eq!(
            decode_wav_pcm16(&empty).unwrap_err().code,
            "local_tts_empty_audio"
        );
        assert_eq!(
            decode_wav_pcm16(b"not a wav").unwrap_err().code,
            "local_tts_audio_format_error"
        );
    }

    #[test]
    fn resampling_preserves_non_empty_signal() {
        let samples = resample_linear(&[0, 1000, 0, -1000], 16_000, 24_000);
        assert_eq!(samples.len(), 6);
        assert!(samples.iter().any(|sample| *sample != 0));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    #[ignore = "requires an installed Windows system voice"]
    async fn windows_system_voice_synthesis_smoke_test() {
        let voice = list_voices()
            .expect("Windows voices should be readable")
            .into_iter()
            .find(|voice| voice.language.to_ascii_lowercase().starts_with("vi"))
            .or_else(|| {
                list_voices()
                    .ok()
                    .and_then(|voices| voices.into_iter().next())
            })
            .expect("at least one Windows voice is required");
        let config = LocalTranslationConfig {
            voice_id: voice.id,
            ..LocalTranslationConfig::default()
        };
        let audio = synthesize("Xin chào.", &config, Arc::new(AtomicBool::new(false)))
            .await
            .expect("Windows synthesis should return PCM");
        assert_eq!(audio.sample_rate_hz, LOCAL_TTS_SAMPLE_RATE);
        assert!(!audio.pcm16_mono.is_empty());
    }
}
