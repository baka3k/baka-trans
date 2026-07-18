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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranslationProvider {
    OpenaiRealtime,
    #[default]
    GoogleLiveTranslate,
    LocalWhisperOllama,
}

impl TranslationProvider {
    pub fn label(self) -> &'static str {
        match self {
            TranslationProvider::OpenaiRealtime => "OpenAI Realtime Translation",
            TranslationProvider::GoogleLiveTranslate => "Google Live Translation",
            TranslationProvider::LocalWhisperOllama => "Local Whisper + Ollama",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            TranslationProvider::OpenaiRealtime => "OPENAI_API_KEY",
            TranslationProvider::GoogleLiveTranslate => "GEMINI_API_KEY",
            TranslationProvider::LocalWhisperOllama => "",
        }
    }

    pub fn requires_api_key(self) -> bool {
        !matches!(self, TranslationProvider::LocalWhisperOllama)
    }
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
    #[serde(rename = "pt-BR")]
    PtBr,
    #[serde(rename = "pt-PT")]
    PtPt,
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
    #[serde(rename = "zh-Hans")]
    ZhHans,
    #[serde(rename = "zh-Hant")]
    ZhHant,
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
            Language::PtBr => "pt-BR",
            Language::PtPt => "pt-PT",
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
            Language::ZhHans => "zh-Hans",
            Language::ZhHant => "zh-Hant",
        }
    }

    pub fn is_openai_realtime_target_supported(self) -> bool {
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

    pub fn is_google_live_target_supported(self) -> bool {
        self != Language::Auto
    }

    pub fn is_target_supported_by(self, provider: TranslationProvider) -> bool {
        match provider {
            TranslationProvider::OpenaiRealtime => self.is_openai_realtime_target_supported(),
            TranslationProvider::GoogleLiveTranslate => self.is_google_live_target_supported(),
            TranslationProvider::LocalWhisperOllama => self == Language::Vi,
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
    #[serde(default)]
    pub translation_provider: TranslationProvider,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayConfig {
    pub source_language: Language,
    pub target_language: Language,
    pub capture_interval_ms: u64,
    pub minimum_confidence: f32,
    pub opacity: f32,
    pub gemini_model: String,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            source_language: Language::Auto,
            target_language: Language::Vi,
            capture_interval_ms: 800,
            minimum_confidence: 0.45,
            opacity: 0.72,
            gemini_model: "models/gemini-2.5-flash".to_string(),
        }
    }
}

impl OverlayConfig {
    pub fn normalized(self) -> Self {
        let gemini_model = if self.gemini_model.trim().is_empty() {
            "models/gemini-2.5-flash".to_string()
        } else {
            self.gemini_model.trim().to_string()
        };
        Self {
            source_language: self.source_language,
            target_language: self.target_language,
            capture_interval_ms: self.capture_interval_ms.clamp(500, 3_000),
            minimum_confidence: self.minimum_confidence.clamp(0.0, 1.0),
            opacity: self.opacity.clamp(0.35, 0.92),
            gemini_model,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayGeometry {
    pub display_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale_factor: f64,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStatusKind {
    Idle,
    PermissionNeeded,
    Scanning,
    Translating,
    Translated,
    Thinking,
    Complete,
    NoText,
    Paused,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayStatus {
    pub is_open: bool,
    pub is_paused: bool,
    pub status: OverlayStatusKind,
    pub message: String,
    pub config: OverlayConfig,
    pub geometry: Option<OverlayGeometry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTranslationUpdate {
    pub source_text: String,
    pub translated_text: String,
    pub status: OverlayStatusKind,
    pub message: String,
    pub confidence: Option<f32>,
    pub latency_ms: Option<u64>,
    pub provider: String,
    pub model: String,
    pub updated_at_ms: u64,
}

pub const DEFAULT_LOOK_HELP_SYSTEM_PROMPT: &str = "You are Look & Help, a compact assistant for the visible screen region. Explain, summarize, or help with the provided OCR text. Be concise, practical, and do not invent details that are not present.";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookHelpConfig {
    pub provider_profile_id: String,
    pub system_prompt: String,
    pub prompt_panel_visible: bool,
    pub capture_interval_ms: u64,
    pub minimum_confidence: f32,
    pub opacity: f32,
    pub max_ocr_input_chars: usize,
    pub max_output_tokens: Option<u32>,
}

impl Default for LookHelpConfig {
    fn default() -> Self {
        Self {
            provider_profile_id: String::new(),
            system_prompt: DEFAULT_LOOK_HELP_SYSTEM_PROMPT.to_string(),
            prompt_panel_visible: false,
            capture_interval_ms: 900,
            minimum_confidence: 0.45,
            opacity: 0.78,
            max_ocr_input_chars: 6_000,
            max_output_tokens: None,
        }
    }
}

impl LookHelpConfig {
    pub fn normalized(self) -> Self {
        let system_prompt = if self.system_prompt.trim().is_empty() {
            DEFAULT_LOOK_HELP_SYSTEM_PROMPT.to_string()
        } else {
            self.system_prompt.trim().to_string()
        };
        Self {
            provider_profile_id: self.provider_profile_id.trim().to_string(),
            system_prompt,
            prompt_panel_visible: self.prompt_panel_visible,
            capture_interval_ms: self.capture_interval_ms.clamp(600, 5_000),
            minimum_confidence: self.minimum_confidence.clamp(0.0, 1.0),
            opacity: self.opacity.clamp(0.35, 0.94),
            max_ocr_input_chars: self.max_ocr_input_chars.clamp(500, 20_000),
            max_output_tokens: self.max_output_tokens.map(|tokens| tokens.clamp(64, 4_000)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookHelpStatus {
    pub is_open: bool,
    pub is_paused: bool,
    pub status: OverlayStatusKind,
    pub message: String,
    pub config: LookHelpConfig,
    pub geometry: Option<OverlayGeometry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LookHelpUpdate {
    pub source_text: String,
    pub answer_text: String,
    pub status: OverlayStatusKind,
    pub message: String,
    pub latency_ms: Option<u64>,
    pub provider_profile_id: String,
    pub model: String,
    pub prompt_hash: u64,
    pub updated_at_ms: u64,
}

impl SessionConfig {
    pub fn validate_translation_target_language(&self) -> AppResult<()> {
        if self.translation_provider == TranslationProvider::LocalWhisperOllama
            && self.source_language != Language::Ja
        {
            return Err(AppError::new(
                "unsupported_source_language",
                "Local Whisper + Ollama currently requires Japanese as the source language.",
            ));
        }
        if self
            .target_language
            .is_target_supported_by(self.translation_provider)
        {
            return Ok(());
        }

        Err(AppError::new(
            "unsupported_target_language",
            format!(
                "Target language '{}' is not supported by {}.",
                self.target_language.realtime_code(),
                self.translation_provider.label(),
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
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub update_mode: TranscriptUpdateMode,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptUpdateMode {
    #[default]
    Delta,
    Snapshot,
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
pub struct TranslationCredentialStatus {
    pub provider: TranslationProvider,
    pub has_api_key: bool,
    pub api_key_source: Option<ApiKeySource>,
    pub api_key_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyTestResult {
    pub provider: TranslationProvider,
    pub source: ApiKeySource,
    pub fingerprint: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LocalTranslationConfig {
    pub schema_version: u32,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub keep_alive: Option<String>,
    pub model_path: String,
    pub language: String,
    pub threads: u32,
    pub use_gpu: bool,
    pub sample_rate_hz: u32,
    pub minimum_speech_ms: u64,
    pub silence_to_commit_ms: u64,
    pub maximum_utterance_ms: u64,
    pub pre_roll_ms: u64,
    pub speech_threshold: f32,
    pub voice_id: String,
    pub tts_rate: f32,
    pub tts_volume: f32,
    pub tts_output_sample_rate_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LocalTranslationConfigDraft {
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub temperature: f32,
    pub max_output_tokens: u32,
    pub keep_alive: Option<String>,
    pub model_path: String,
    pub language: String,
    pub threads: u32,
    pub use_gpu: bool,
    pub sample_rate_hz: u32,
    pub minimum_speech_ms: u64,
    pub silence_to_commit_ms: u64,
    pub maximum_utterance_ms: u64,
    pub pre_roll_ms: u64,
    pub speech_threshold: f32,
    pub voice_id: String,
    pub tts_rate: f32,
    pub tts_volume: f32,
    pub tts_output_sample_rate_hz: u32,
}

impl From<LocalTranslationConfig> for LocalTranslationConfigDraft {
    fn from(value: LocalTranslationConfig) -> Self {
        Self {
            base_url: value.base_url,
            model: value.model,
            timeout_seconds: value.timeout_seconds,
            temperature: value.temperature,
            max_output_tokens: value.max_output_tokens,
            keep_alive: value.keep_alive,
            model_path: value.model_path,
            language: value.language,
            threads: value.threads,
            use_gpu: value.use_gpu,
            sample_rate_hz: value.sample_rate_hz,
            minimum_speech_ms: value.minimum_speech_ms,
            silence_to_commit_ms: value.silence_to_commit_ms,
            maximum_utterance_ms: value.maximum_utterance_ms,
            pre_roll_ms: value.pre_roll_ms,
            speech_threshold: value.speech_threshold,
            voice_id: value.voice_id,
            tts_rate: value.tts_rate,
            tts_volume: value.tts_volume,
            tts_output_sample_rate_hz: value.tts_output_sample_rate_hz,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalTranslationTestResult {
    pub ok: bool,
    pub message: String,
    pub model: String,
    pub endpoint: String,
    pub whisper_model_readable: bool,
    pub whisper_model_loaded: bool,
    pub ollama_reachable: bool,
    pub ollama_model_accepted: bool,
    pub tts_voice_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalVoice {
    pub id: String,
    pub name: String,
    pub language: String,
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
pub enum MeetingSummaryPromptPreset {
    Balanced,
    Professional,
    Gentle,
    Detailed,
    Timeline,
    Custom,
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
    pub prompt_preset: MeetingSummaryPromptPreset,
    pub custom_system_prompt: String,
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
            translation_provider: TranslationProvider::OpenaiRealtime,
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
            .validate_translation_target_language()
            .is_ok());
    }

    #[test]
    fn rejects_unsupported_realtime_target_language() {
        let error = session_config(Language::Ar)
            .validate_translation_target_language()
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

    #[test]
    fn google_target_language_supports_regional_codes() {
        let mut config = session_config(Language::PtBr);
        config.translation_provider = TranslationProvider::GoogleLiveTranslate;

        assert!(config.validate_translation_target_language().is_ok());
    }

    #[test]
    fn google_target_language_rejects_auto() {
        let mut config = session_config(Language::Auto);
        config.translation_provider = TranslationProvider::GoogleLiveTranslate;

        let error = config
            .validate_translation_target_language()
            .expect_err("Auto should not be allowed as a target language");

        assert_eq!(error.code, "unsupported_target_language");
        assert!(error.message.contains("Google Live Translation"));
    }
}
