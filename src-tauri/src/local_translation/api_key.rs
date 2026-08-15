use crate::error::{AppError, AppResult};
use keyring::Entry;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "dev.baka3k.baka-trans";
const LOCAL_TRANSLATION_KEY_USER: &str = "local-translation-api-key";
const ENV_VAR_NAME: &str = "BAKA_TRANS_LOCAL_API_KEY";

static CACHED_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn key_cache() -> &'static Mutex<Option<String>> {
    CACHED_KEY.get_or_init(|| Mutex::new(None))
}

pub fn save_local_translation_api_key(api_key: &str) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "local_translation_api_key_empty",
            "API key cannot be empty.",
        ));
    }
    let entry = Entry::new(SERVICE, LOCAL_TRANSLATION_KEY_USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    *key_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))? =
        Some(trimmed.to_string());
    Ok(())
}

pub fn load_local_translation_api_key() -> AppResult<Option<ApiKeyInfo>> {
    if let Ok(value) = std::env::var(ENV_VAR_NAME) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(Some(ApiKeyInfo {
                key: trimmed,
                source: "environment",
            }));
        }
    }

    if let Some(cached) = key_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))?
        .clone()
    {
        if !cached.is_empty() {
            return Ok(Some(ApiKeyInfo {
                key: cached,
                source: "memory",
            }));
        }
    }

    let entry = Entry::new(SERVICE, LOCAL_TRANSLATION_KEY_USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    match entry.get_password() {
        Ok(secret) => {
            let secret = secret.trim().to_string();
            if secret.is_empty() {
                return Ok(None);
            }
            *key_cache()
                .lock()
                .map_err(|err| AppError::new("credential_cache_error", err.to_string()))? =
                Some(secret.clone());
            Ok(Some(ApiKeyInfo {
                key: secret,
                source: "keychain",
            }))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(AppError::new("keychain_error", err.to_string())),
    }
}

pub fn delete_local_translation_api_key() -> AppResult<()> {
    let entry = Entry::new(SERVICE, LOCAL_TRANSLATION_KEY_USER)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    *key_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))? =
        None;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(AppError::new("keychain_error", err.to_string())),
    }
}

pub fn has_local_translation_api_key() -> AppResult<bool> {
    Ok(load_local_translation_api_key()?.is_some())
}

pub fn local_translation_api_key_fingerprint() -> AppResult<Option<String>> {
    Ok(load_local_translation_api_key()?.map(|info| fingerprint_key(&info.key)))
}

pub fn local_translation_api_key_source() -> AppResult<Option<String>> {
    Ok(load_local_translation_api_key()?.map(|info| info.source.to_string()))
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key: String,
    pub source: &'static str,
}

fn fingerprint_key(api_key: &str) -> String {
    let chars = api_key.chars().collect::<Vec<_>>();
    let prefix: String = chars.iter().take(7).collect();
    let suffix: String = chars.iter().rev().take(4).rev().collect();
    if api_key.len() <= 12 {
        return "short-key".to_string();
    }
    format!("{prefix}...{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_long_keys() {
        let fp = fingerprint_key("sk-1234567890abcdef");
        assert!(fp.starts_with("sk-1234"));
        assert!(fp.ends_with("cdef"));
        assert!(fp.contains("..."));
    }

    #[test]
    fn short_keys_get_generic_fingerprint() {
        assert_eq!(fingerprint_key("sk-1234"), "short-key");
    }
}
