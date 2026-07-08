use crate::error::{AppError, AppResult};
use crate::models::ApiKeySource;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "dev.baka3k.baka-trans";
const USER: &str = "openai-api-key";
static API_KEY_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn save_api_key(api_key: &str) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("invalid_api_key", "API key cannot be empty."));
    }

    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    cache_api_key(trimmed)?;

    Ok(())
}

pub fn load_api_key() -> AppResult<String> {
    load_api_key_with_info().map(|info| info.key)
}

pub fn api_key_status() -> Option<ApiKeyInfo> {
    load_api_key_with_info().ok()
}

pub struct ApiKeyInfo {
    pub key: String,
    pub source: ApiKeySource,
    pub fingerprint: String,
}

fn load_api_key_with_info() -> AppResult<ApiKeyInfo> {
    if let Ok(value) = std::env::var("OPENAI_API_KEY") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(ApiKeyInfo {
                key: trimmed.to_string(),
                source: ApiKeySource::Environment,
                fingerprint: fingerprint_key(trimmed),
            });
        }
    }

    match keychain_api_key() {
        Ok(key) => {
            cache_api_key(&key)?;
            Ok(ApiKeyInfo {
                fingerprint: fingerprint_key(&key),
                key,
                source: ApiKeySource::Keychain,
            })
        }
        Err(keychain_error) => cached_api_key()?
            .map(|key| ApiKeyInfo {
                fingerprint: fingerprint_key(&key),
                key,
                source: ApiKeySource::Memory,
            })
            .ok_or(keychain_error),
    }
}

pub fn has_api_key() -> bool {
    api_key_status().is_some()
}

fn keychain_api_key() -> AppResult<String> {
    let entry = keyring::Entry::new(SERVICE, USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    let stored = entry
        .get_password()
        .map_err(|_| AppError::missing_api_key())?;
    let trimmed = stored.trim();
    if trimmed.is_empty() {
        return Err(AppError::missing_api_key());
    }

    Ok(trimmed.to_string())
}

fn cache_api_key(api_key: &str) -> AppResult<()> {
    *api_key_cache().lock().map_err(cache_lock_error)? = Some(api_key.to_string());
    Ok(())
}

fn cached_api_key() -> AppResult<Option<String>> {
    Ok(api_key_cache()
        .lock()
        .map_err(cache_lock_error)?
        .as_ref()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty()))
}

fn fingerprint_key(api_key: &str) -> String {
    let chars = api_key.chars().collect::<Vec<_>>();
    let prefix = chars.iter().take(7).collect::<String>();
    let suffix = chars
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    if api_key.len() <= 12 {
        return "short-key".to_string();
    }
    format!("{prefix}...{suffix}")
}

fn api_key_cache() -> &'static Mutex<Option<String>> {
    API_KEY_CACHE.get_or_init(|| Mutex::new(None))
}

fn cache_lock_error<T>(_: std::sync::PoisonError<T>) -> AppError {
    AppError::new("state_lock_error", "API key cache lock was poisoned.")
}
