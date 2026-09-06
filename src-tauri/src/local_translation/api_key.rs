use crate::error::{AppError, AppResult};
use crate::models::{ApiKeySource, LocalCredentialStatus};
use keyring::Entry;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "dev.baka3k.baka-trans";
const LOCAL_TRANSLATION_KEY_USER: &str = "local-translation-api-key";
const ENV_VAR_NAME: &str = "BAKA_TRANS_LOCAL_API_KEY";
const MAX_KEY_CHARS: usize = 4096;

static CACHED_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn key_cache() -> &'static Mutex<Option<String>> {
    CACHED_KEY.get_or_init(|| Mutex::new(None))
}

pub fn save_local_translation_api_key(key: &str) -> AppResult<()> {
    let trimmed = validate_key_input(key)?;
    local_translation_entry()?
        .set_password(&trimmed)
        .map_err(keychain_error)?;
    *key_cache()
        .lock()
        .map_err(|err| credential_cache_error(err.to_string()))? = Some(trimmed);
    Ok(())
}

pub fn delete_local_translation_api_key() -> AppResult<()> {
    match local_translation_entry()?.delete_credential() {
        Ok(()) => {}
        Err(keyring::Error::NoEntry) => {}
        Err(err) => return Err(keychain_error(err)),
    }
    *key_cache()
        .lock()
        .map_err(|err| credential_cache_error(err.to_string()))? = None;
    Ok(())
}

pub fn local_translation_credential_status() -> AppResult<LocalCredentialStatus> {
    let env_present = env_key().is_some();
    let cached_present = if env_present {
        false
    } else {
        cached_key()?.is_some()
    };
    let keychain_present = if env_present || cached_present {
        false
    } else {
        keychain_key_present()?
    };
    Ok(credential_status_from_parts(
        env_present,
        cached_present,
        keychain_present,
    ))
}

fn validate_key_input(key: &str) -> AppResult<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "credential_input_invalid",
            "The API key cannot be empty.",
        ));
    }
    if trimmed.chars().count() > MAX_KEY_CHARS {
        return Err(AppError::new(
            "credential_input_invalid",
            format!("The API key must be at most {MAX_KEY_CHARS} characters."),
        ));
    }
    Ok(trimmed.to_string())
}

/// Pure resolution rule shared by the status command and its tests: the
/// environment variable wins, then the keychain (the in-memory cache only
/// mirrors the keychain — save, delete, and load keep both in sync), and no
/// source means no key.
fn credential_status_from_parts(
    env_present: bool,
    cached_present: bool,
    keychain_present: bool,
) -> LocalCredentialStatus {
    if env_present {
        return LocalCredentialStatus {
            has_key: true,
            source: Some(ApiKeySource::Environment),
        };
    }
    if cached_present || keychain_present {
        return LocalCredentialStatus {
            has_key: true,
            source: Some(ApiKeySource::Keychain),
        };
    }
    LocalCredentialStatus {
        has_key: false,
        source: None,
    }
}

fn local_translation_entry() -> AppResult<Entry> {
    Entry::new(SERVICE, LOCAL_TRANSLATION_KEY_USER).map_err(keychain_error)
}

fn keychain_key_present() -> AppResult<bool> {
    match local_translation_entry()?.get_password() {
        Ok(secret) => Ok(!secret.trim().is_empty()),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(err) => Err(keychain_error(err)),
    }
}

fn env_key() -> Option<String> {
    std::env::var(ENV_VAR_NAME)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn cached_key() -> AppResult<Option<String>> {
    Ok(key_cache()
        .lock()
        .map_err(|err| credential_cache_error(err.to_string()))?
        .clone()
        .filter(|key| !key.is_empty()))
}

fn keychain_error(err: keyring::Error) -> AppError {
    AppError::new("keychain_error", err.to_string())
}

fn credential_cache_error(message: String) -> AppError {
    AppError::new("credential_cache_error", message)
}

pub fn load_local_translation_api_key() -> AppResult<Option<ApiKeyInfo>> {
    if let Ok(value) = std::env::var(ENV_VAR_NAME) {
        let trimmed = value.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(Some(ApiKeyInfo { key: trimmed }));
        }
    }

    if let Some(cached) = key_cache()
        .lock()
        .map_err(|err| crate::error::AppError::new("credential_cache_error", err.to_string()))?
        .clone()
    {
        if !cached.is_empty() {
            return Ok(Some(ApiKeyInfo { key: cached }));
        }
    }

    let entry = Entry::new(SERVICE, LOCAL_TRANSLATION_KEY_USER)
        .map_err(|err| crate::error::AppError::new("keychain_error", err.to_string()))?;
    match entry.get_password() {
        Ok(secret) => {
            let secret = secret.trim().to_string();
            if secret.is_empty() {
                return Ok(None);
            }
            *key_cache().lock().map_err(|err| {
                crate::error::AppError::new("credential_cache_error", err.to_string())
            })? = Some(secret.clone());
            Ok(Some(ApiKeyInfo { key: secret }))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(crate::error::AppError::new(
            "keychain_error",
            err.to_string(),
        )),
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_key_input_without_touching_the_keychain() {
        assert_eq!(
            validate_key_input("").unwrap_err().code,
            "credential_input_invalid"
        );
        assert_eq!(
            validate_key_input("   \t \n ").unwrap_err().code,
            "credential_input_invalid"
        );
    }

    #[test]
    fn rejects_oversized_key_input_without_touching_the_keychain() {
        let oversized = "k".repeat(MAX_KEY_CHARS + 1);
        assert_eq!(
            validate_key_input(&oversized).unwrap_err().code,
            "credential_input_invalid"
        );
        assert_eq!(
            validate_key_input(&"k".repeat(MAX_KEY_CHARS)).unwrap(),
            "k".repeat(MAX_KEY_CHARS)
        );
    }

    #[test]
    fn trims_surrounding_whitespace_from_valid_key_input() {
        assert_eq!(validate_key_input("  sk-test  ").unwrap(), "sk-test");
    }

    #[test]
    fn credential_status_prefers_the_environment_variable() {
        let status = credential_status_from_parts(true, true, true);
        assert!(status.has_key);
        assert_eq!(status.source, Some(ApiKeySource::Environment));

        let status = credential_status_from_parts(true, false, false);
        assert!(status.has_key);
        assert_eq!(status.source, Some(ApiKeySource::Environment));
    }

    #[test]
    fn credential_status_reports_keychain_when_cached_or_stored() {
        let status = credential_status_from_parts(false, true, false);
        assert!(status.has_key);
        assert_eq!(status.source, Some(ApiKeySource::Keychain));

        let status = credential_status_from_parts(false, false, true);
        assert!(status.has_key);
        assert_eq!(status.source, Some(ApiKeySource::Keychain));
    }

    #[test]
    fn credential_status_reports_no_key_when_every_source_is_absent() {
        let status = credential_status_from_parts(false, false, false);
        assert!(!status.has_key);
        assert_eq!(status.source, None);
    }
}
