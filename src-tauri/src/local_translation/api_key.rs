use crate::error::AppResult;
use keyring::Entry;
use std::sync::{Mutex, OnceLock};

const SERVICE: &str = "dev.baka3k.baka-trans";
const LOCAL_TRANSLATION_KEY_USER: &str = "local-translation-api-key";
const ENV_VAR_NAME: &str = "BAKA_TRANS_LOCAL_API_KEY";

static CACHED_KEY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

fn key_cache() -> &'static Mutex<Option<String>> {
    CACHED_KEY.get_or_init(|| Mutex::new(None))
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
