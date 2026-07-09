use crate::error::{AppError, AppResult};
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
    Ar,
    Af,
    Az,
    Be,
    Bn,
    Bs,
    Bg,
    Ca,
    Zh,
    Hr,
    Cs,
    Da,
    Nl,
    Dz,
    En,
    Eo,
    Et,
    Eu,
    Fa,
    Fi,
    Fil,
    Fr,
    Gl,
    De,
    El,
    Gu,
    Ht,
    Haw,
    He,
    Hi,
    Hu,
    Hy,
    Id,
    It,
    Ja,
    Jv,
    Ka,
    Kk,
    Ko,
    Ku,
    La,
    Lv,
    Lt,
    Mk,
    Ms,
    Ml,
    Mi,
    Mn,
    My,
    Ne,
    No,
    Nn,
    Pl,
    Pt,
    Pa,
    Ro,
    Ru,
    Sr,
    Sn,
    Sk,
    Sl,
    Sq,
    Es,
    Sw,
    Sv,
    Tl,
    Te,
    Th,
    Tr,
    Uk,
    Uz,
    Vi,
    Cy,
    Yo,
}

impl Language {
    pub fn realtime_code(self) -> &'static str {
        match self {
            Language::Auto => "auto",
            Language::Ar => "ar",
            Language::Af => "af",
            Language::Az => "az",
            Language::Be => "be",
            Language::Bn => "bn",
            Language::Bs => "bs",
            Language::Bg => "bg",
            Language::Ca => "ca",
            Language::Zh => "zh",
            Language::Hr => "hr",
            Language::Cs => "cs",
            Language::Da => "da",
            Language::Nl => "nl",
            Language::Dz => "dz",
            Language::En => "en",
            Language::Eo => "eo",
            Language::Et => "et",
            Language::Eu => "eu",
            Language::Fa => "fa",
            Language::Fi => "fi",
            Language::Fil => "fil",
            Language::Fr => "fr",
            Language::Gl => "gl",
            Language::De => "de",
            Language::El => "el",
            Language::Gu => "gu",
            Language::Ht => "ht",
            Language::Haw => "haw",
            Language::He => "he",
            Language::Hi => "hi",
            Language::Hu => "hu",
            Language::Hy => "hy",
            Language::Id => "id",
            Language::It => "it",
            Language::Ja => "ja",
            Language::Jv => "jv",
            Language::Ka => "ka",
            Language::Kk => "kk",
            Language::Ko => "ko",
            Language::Ku => "ku",
            Language::La => "la",
            Language::Lv => "lv",
            Language::Lt => "lt",
            Language::Mk => "mk",
            Language::Ms => "ms",
            Language::Ml => "ml",
            Language::Mi => "mi",
            Language::Mn => "mn",
            Language::My => "my",
            Language::Ne => "ne",
            Language::No => "no",
            Language::Nn => "nn",
            Language::Pl => "pl",
            Language::Pt => "pt",
            Language::Pa => "pa",
            Language::Ro => "ro",
            Language::Ru => "ru",
            Language::Sr => "sr",
            Language::Sn => "sn",
            Language::Sk => "sk",
            Language::Sl => "sl",
            Language::Sq => "sq",
            Language::Es => "es",
            Language::Sw => "sw",
            Language::Sv => "sv",
            Language::Tl => "tl",
            Language::Te => "te",
            Language::Th => "th",
            Language::Tr => "tr",
            Language::Uk => "uk",
            Language::Uz => "uz",
            Language::Vi => "vi",
            Language::Cy => "cy",
            Language::Yo => "yo",
        }
    }

    pub fn is_realtime_target_supported(self) -> bool {
        matches!(
            self,
            Language::Es
                | Language::Pt
                | Language::Fr
                | Language::Ja
                | Language::Ru
                | Language::Zh
                | Language::De
                | Language::Ko
                | Language::Hi
                | Language::Id
                | Language::Vi
                | Language::It
                | Language::En
        )
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

impl SessionConfig {
    pub fn validate_realtime_target_language(&self) -> AppResult<()> {
        if self.target_language.is_realtime_target_supported() {
            return Ok(());
        }

        Err(AppError::new(
            "unsupported_target_language",
            format!(
                "Target language '{}' is not supported by OpenAI Realtime Translation.",
                self.target_language.realtime_code(),
            ),
        ))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn session_config(target_language: Language) -> SessionConfig {
        SessionConfig {
            source_language: Language::Auto,
            target_language,
            translation_style: TranslationStyle::TechnicalMeetingSafe,
            input_device_id: "input".to_string(),
            output_device_id: "output".to_string(),
            translation_output_channel: AudioOutputChannel::All,
            monitor_output_device_id: String::new(),
            monitor_output_channel: AudioOutputChannel::All,
            monitor_original_audio: false,
            voice_id: "marin".to_string(),
            fallback_enabled: false,
        }
    }

    #[test]
    fn validates_realtime_target_language() {
        assert!(session_config(Language::Es)
            .validate_realtime_target_language()
            .is_ok());
    }

    #[test]
    fn rejects_unsupported_realtime_target_language() {
        let error = session_config(Language::Ar)
            .validate_realtime_target_language()
            .expect_err("Arabic should not be allowed as a target language");

        assert_eq!(error.code, "unsupported_target_language");
        assert!(
            error
                .message
                .contains("Target language 'ar' is not supported"),
            "{}",
            error.message,
        );
    }
}
