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
pub enum AudioOutputChannel {
    All,
    Left,
    Right,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub source_language: Language,
    pub target_language: Language,
    pub translation_style: TranslationStyle,
    pub input_device_id: String,
    pub output_device_id: String,
    pub translation_output_channel: AudioOutputChannel,
    pub monitor_output_device_id: String,
    pub monitor_output_channel: AudioOutputChannel,
    pub monitor_original_audio: bool,
    pub voice_id: String,
    pub fallback_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManualBoundaryReason {
    UserButton,
    KeyboardShortcut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualBoundaryRequest {
    pub reason: ManualBoundaryReason,
    pub requested_at_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManualBoundaryStatus {
    Idle,
    Pending,
    Committed,
    IgnoredEmptyBuffer,
    RateLimited,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualBoundaryEvent {
    pub status: ManualBoundaryStatus,
    pub message: String,
    pub committed_at_ms: Option<u64>,
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
    pub api_key_source: Option<ApiKeySource>,
    pub api_key_fingerprint: Option<String>,
    pub transcript_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyTestResult {
    pub source: ApiKeySource,
    pub fingerprint: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeySource {
    Environment,
    Keychain,
    Memory,
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
pub struct TranslatedAudioLevelEvent {
    pub sample_count: usize,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LlmProviderKind {
    Openai,
    OpenaiCompatible,
    Ollama,
    AdkLitellm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderProfile {
    pub id: String,
    pub name: String,
    pub kind: LlmProviderKind,
    pub model: String,
    pub base_url: Option<String>,
    pub has_api_key: bool,
    pub api_key_source: Option<String>,
    pub api_key_fingerprint: Option<String>,
    pub timeout_seconds: u64,
    pub max_output_tokens: u32,
    pub temperature: f32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderProfileDraft {
    pub id: Option<String>,
    pub name: String,
    pub kind: LlmProviderKind,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub max_output_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmProviderTestResult {
    pub profile_id: String,
    pub ok: bool,
    pub message: String,
    pub model: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingSummaryTrigger {
    Manual,
    EndOfSession,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptScope {
    Source,
    Translated,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummarySections {
    pub summary: bool,
    pub decisions: bool,
    pub action_items: bool,
    pub blockers: bool,
    pub important_points: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryConfig {
    pub provider_profile_id: String,
    pub trigger: MeetingSummaryTrigger,
    pub transcript_scope: TranscriptScope,
    pub output_language: String,
    pub sections: MeetingSummarySections,
    pub max_transcript_chars: usize,
    pub rolling_memory_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    pub text: String,
    pub owner: Option<String>,
    pub due_date: Option<String>,
    pub source_item_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeetingSummaryStatus {
    Running,
    Complete,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryResult {
    pub id: String,
    pub created_at_ms: u64,
    pub source_item_ids: Vec<String>,
    pub summary: String,
    pub decisions: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub blockers: Vec<String>,
    pub important_points: Vec<String>,
    pub model: String,
    pub provider_profile_id: String,
    pub status: MeetingSummaryStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummaryStatusEvent {
    pub status: MeetingSummaryStatus,
    pub message: String,
}
