use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn missing_api_key() -> Self {
        Self::new(
            "missing_api_key",
            "Save an OpenAI API key in Settings or set OPENAI_API_KEY for development.",
        )
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<cpal::DevicesError> for AppError {
    fn from(value: cpal::DevicesError) -> Self {
        Self::new("audio_devices_error", value.to_string())
    }
}

impl From<cpal::SupportedStreamConfigsError> for AppError {
    fn from(value: cpal::SupportedStreamConfigsError) -> Self {
        Self::new("audio_format_error", value.to_string())
    }
}

impl From<cpal::BuildStreamError> for AppError {
    fn from(value: cpal::BuildStreamError) -> Self {
        Self::new("audio_stream_error", value.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for AppError {
    fn from(value: cpal::DefaultStreamConfigError) -> Self {
        Self::new("audio_format_error", value.to_string())
    }
}

impl From<cpal::PlayStreamError> for AppError {
    fn from(value: cpal::PlayStreamError) -> Self {
        Self::new("audio_stream_error", value.to_string())
    }
}
