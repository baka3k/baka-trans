use crate::error::{AppError, AppResult};
use crate::models::{ApiKeySource, TranslationCredentialStatus, TranslationProvider};
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "dev.baka3k.baka-trans";
const OPENAI_USER: &str = "openai-api-key";
const GOOGLE_USER: &str = "google-gemini-api-key";
static OPENAI_API_KEY_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static GOOGLE_API_KEY_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

pub fn save_api_key(api_key: &str) -> AppResult<()> {
    save_translation_api_key(TranslationProvider::OpenaiRealtime, api_key)
}

pub fn save_translation_api_key(provider: TranslationProvider, api_key: &str) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("invalid_api_key", "API key cannot be empty."));
    }

    let entry = keyring::Entry::new(SERVICE, keychain_user(provider))
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    cache_api_key(provider, trimmed)?;

    Ok(())
}

pub fn load_translation_api_key(provider: TranslationProvider) -> AppResult<String> {
    load_translation_api_key_info(provider).map(|info| info.key)
}

pub fn api_key_status() -> Option<ApiKeyInfo> {
    translation_api_key_status(TranslationProvider::OpenaiRealtime)
}

pub fn translation_api_key_status(provider: TranslationProvider) -> Option<ApiKeyInfo> {
    load_translation_api_key_info(provider).ok()
}

pub fn translation_credential_status(provider: TranslationProvider) -> TranslationCredentialStatus {
    let info = translation_api_key_status(provider);
    TranslationCredentialStatus {
        provider,
        has_api_key: info.is_some(),
        api_key_source: info.as_ref().map(|info| info.source),
        api_key_fingerprint: info.map(|info| info.fingerprint),
    }
}

pub struct ApiKeyInfo {
    pub key: String,
    pub source: ApiKeySource,
    pub fingerprint: String,
}

pub fn load_translation_api_key_info(provider: TranslationProvider) -> AppResult<ApiKeyInfo> {
    if let Ok(value) = std::env::var(provider.env_var()) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(ApiKeyInfo {
                key: trimmed.to_string(),
                source: ApiKeySource::Environment,
                fingerprint: fingerprint_key(trimmed),
            });
        }
    }

    match keychain_api_key(provider) {
        Ok(key) => {
            cache_api_key(provider, &key)?;
            Ok(ApiKeyInfo {
                fingerprint: fingerprint_key(&key),
                key,
                source: ApiKeySource::Keychain,
            })
        }
        Err(keychain_error) => cached_api_key(provider)?
            .map(|key| ApiKeyInfo {
                fingerprint: fingerprint_key(&key),
                key,
                source: ApiKeySource::Memory,
            })
            .ok_or(keychain_error),
    }
}

pub fn has_api_key() -> bool {
    has_translation_api_key(TranslationProvider::OpenaiRealtime)
}

pub fn has_translation_api_key(provider: TranslationProvider) -> bool {
    translation_api_key_status(provider).is_some()
}

fn keychain_api_key(provider: TranslationProvider) -> AppResult<String> {
    let entry = keyring::Entry::new(SERVICE, keychain_user(provider))
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    let stored = entry
        .get_password()
        .map_err(|_| missing_api_key(provider))?;
    let trimmed = stored.trim();
    if trimmed.is_empty() {
        return Err(missing_api_key(provider));
    }

    Ok(trimmed.to_string())
}

fn cache_api_key(provider: TranslationProvider, api_key: &str) -> AppResult<()> {
    *api_key_cache(provider).lock().map_err(cache_lock_error)? = Some(api_key.to_string());
    Ok(())
}

fn cached_api_key(provider: TranslationProvider) -> AppResult<Option<String>> {
    Ok(api_key_cache(provider)
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

fn keychain_user(provider: TranslationProvider) -> &'static str {
    match provider {
        TranslationProvider::OpenaiRealtime => OPENAI_USER,
        TranslationProvider::GoogleLiveTranslate => GOOGLE_USER,
    }
}

fn api_key_cache(provider: TranslationProvider) -> &'static Mutex<Option<String>> {
    match provider {
        TranslationProvider::OpenaiRealtime => {
            OPENAI_API_KEY_CACHE.get_or_init(|| Mutex::new(None))
        }
        TranslationProvider::GoogleLiveTranslate => {
            GOOGLE_API_KEY_CACHE.get_or_init(|| Mutex::new(None))
        }
    }
}

fn missing_api_key(provider: TranslationProvider) -> AppError {
    AppError::new(
        "missing_api_key",
        format!(
            "Save a {} key in Settings or set {} for development.",
            provider.label(),
            provider.env_var(),
        ),
    )
}

fn cache_lock_error<T>(_: std::sync::PoisonError<T>) -> AppError {
    AppError::new("state_lock_error", "API key cache lock was poisoned.")
}
