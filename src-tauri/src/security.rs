use crate::error::{AppError, AppResult};

const SERVICE: &str = "dev.baka3k.baka-trans";
const USER: &str = "openai-api-key";

pub fn save_api_key(api_key: &str) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("invalid_api_key", "API key cannot be empty."));
    }

    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))
}

pub fn load_api_key() -> AppResult<String> {
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .get_password()
        .map_err(|_| AppError::missing_api_key())
}

pub fn has_api_key() -> bool {
    load_api_key().is_ok()
}
