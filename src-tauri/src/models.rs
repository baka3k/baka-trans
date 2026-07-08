use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub is_default: bool,
    pub min_sample_rate: Option<u32>,
    pub max_sample_rate: Option<u32>,
    pub max_channels: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    pub inputs: Vec<AudioDeviceInfo>,
    pub outputs: Vec<AudioDeviceInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Auto,
    En,
    Ja,
    Vi,
}

impl Language {
    pub fn realtime_code(self) -> &'static str {
        match self {
            Language::Auto => "auto",
            Language::En => "en",
            Language::Ja => "ja",
            Language::Vi => "vi",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationStyle {
    Literal,
    Natural,
    TechnicalMeetingSafe,
}

impl TranslationStyle {
    pub fn instructions(self) -> &'static str {
        match self {
            TranslationStyle::Literal => "Translate speech literally and preserve wording where possible.",
            TranslationStyle::Natural => "Translate speech naturally for a listener while preserving meaning.",
            TranslationStyle::TechnicalMeetingSafe => {
                "Translate for a technical business meeting. Preserve names, product terms, code terms, numbers, and decisions."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub source_language: Language,
    pub target_language: Language,
    pub translation_style: TranslationStyle,
    pub input_device_id: String,
    pub output_device_id: String,
    pub monitor_output_device_id: String,
    pub monitor_original_audio: bool,
    pub voice_id: String,
    pub fallback_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Idle,
    Starting,
    Listening,
    Translating,
    Speaking,
    Paused,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptItem {
    pub id: String,
    pub timestamp_ms: u64,
    pub source_text: String,
    pub translated_text: String,
    pub status: TranscriptStatus,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptStatus {
    Partial,
    Final,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub session_status: SessionStatus,
    pub has_api_key: bool,
    pub transcript_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelEvent {
    pub input_device_id: String,
    pub rms: f32,
    pub peak: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub format: ExportFormat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Text,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedTranscript {
    pub file_name: String,
    pub content: String,
}
