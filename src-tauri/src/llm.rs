use crate::error::{AppError, AppResult};
use crate::models::{
    LlmProviderKind, LlmProviderProfile, LlmProviderProfileDraft, LlmProviderTestResult,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use uuid::Uuid;

const SERVICE: &str = "dev.baka3k.baka-trans";
const PROFILE_SECRET_PREFIX: &str = "llm-profile-";
const CONFIG_DIR_NAME: &str = "dev.baka3k.baka-trans";
const PROFILE_FILE_NAME: &str = "llm-profiles.json";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
static PROFILE_SECRET_CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ProfileStore {
    profiles: Vec<StoredLlmProviderProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredLlmProviderProfile {
    id: String,
    name: String,
    kind: LlmProviderKind,
    model: String,
    base_url: Option<String>,
    timeout_seconds: u64,
    max_output_tokens: u32,
    temperature: f32,
    enabled: bool,
}

#[derive(Debug, Clone)]
struct ProfileSecretInfo {
    key: String,
    source: &'static str,
}

impl StoredLlmProviderProfile {
    fn into_profile(self, secret: Option<ProfileSecretInfo>) -> LlmProviderProfile {
        let api_key_fingerprint = secret.as_ref().map(|secret| fingerprint_key(&secret.key));
        LlmProviderProfile {
            id: self.id,
            name: self.name,
            kind: self.kind,
            model: self.model,
            base_url: self.base_url,
            has_api_key: secret.is_some(),
            api_key_source: secret.as_ref().map(|secret| secret.source.to_string()),
            api_key_fingerprint,
            timeout_seconds: self.timeout_seconds,
            max_output_tokens: self.max_output_tokens,
            temperature: self.temperature,
            enabled: self.enabled,
        }
    }
}

impl From<&LlmProviderProfile> for StoredLlmProviderProfile {
    fn from(value: &LlmProviderProfile) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            kind: value.kind,
            model: value.model.clone(),
            base_url: value.base_url.clone(),
            timeout_seconds: value.timeout_seconds,
            max_output_tokens: value.max_output_tokens,
            temperature: value.temperature,
            enabled: value.enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: &'static str,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ChatCompletion {
    pub content: String,
}

pub fn list_profiles() -> AppResult<Vec<LlmProviderProfile>> {
    let store = read_store()?;
    store
        .profiles
        .into_iter()
        .map(|profile| {
            let secret = load_profile_secret(&profile.id)?;
            Ok(profile.into_profile(secret))
        })
        .collect()
}

pub fn save_profile(draft: LlmProviderProfileDraft) -> AppResult<LlmProviderProfile> {
    let mut store = read_store()?;
    let id = draft
        .id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let existing = store.profiles.iter().find(|profile| profile.id == id);
    let base_url = normalize_profile_base_url(draft.kind, draft.base_url.as_deref())?;
    let profile = LlmProviderProfile {
        id: id.clone(),
        name: non_empty_or(
            draft.name.trim(),
            existing
                .map(|profile| profile.name.as_str())
                .unwrap_or("Summary profile"),
        ),
        kind: draft.kind,
        model: require_non_empty(&draft.model, "missing_llm_model", "Enter a summary model.")?,
        base_url,
        has_api_key: false,
        api_key_source: None,
        api_key_fingerprint: None,
        timeout_seconds: draft
            .timeout_seconds
            .or_else(|| existing.map(|profile| profile.timeout_seconds))
            .unwrap_or(45)
            .clamp(5, 300),
        max_output_tokens: draft
            .max_output_tokens
            .or_else(|| existing.map(|profile| profile.max_output_tokens))
            .unwrap_or(1200)
            .clamp(128, 16_384),
        temperature: draft
            .temperature
            .or_else(|| existing.map(|profile| profile.temperature))
            .unwrap_or(0.2)
            .clamp(0.0, 2.0),
        enabled: draft
            .enabled
            .or_else(|| existing.map(|profile| profile.enabled))
            .unwrap_or(true),
    };

    if let Some(api_key) = draft
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|key| !key.is_empty())
    {
        save_profile_secret(&id, api_key)?;
    }

    if let Some(index) = store.profiles.iter().position(|profile| profile.id == id) {
        store.profiles[index] = StoredLlmProviderProfile::from(&profile);
    } else {
        store
            .profiles
            .push(StoredLlmProviderProfile::from(&profile));
    }
    write_store(&store)?;

    get_profile(&id)
}

pub fn delete_profile(profile_id: &str) -> AppResult<()> {
    let mut store = read_store()?;
    let before = store.profiles.len();
    store.profiles.retain(|profile| profile.id != profile_id);
    if before == store.profiles.len() {
        return Err(AppError::new(
            "llm_profile_not_found",
            "The selected summary profile no longer exists.",
        ));
    }
    write_store(&store)?;
    let _ = delete_profile_secret(profile_id);
    Ok(())
}

pub fn get_profile(profile_id: &str) -> AppResult<LlmProviderProfile> {
    list_profiles()?
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| {
            AppError::new(
                "llm_profile_not_found",
                "Choose a summary provider profile before running notes.",
            )
        })
}

pub async fn test_profile(profile_id: &str) -> AppResult<LlmProviderTestResult> {
    let profile = get_profile(profile_id)?;
    let completion = chat_completion(
        &profile,
        vec![
            ChatMessage {
                role: "system",
                content: "Return a compact JSON object.".to_string(),
            },
            ChatMessage {
                role: "user",
                content: "Return exactly: {\"ok\":true}".to_string(),
            },
        ],
        true,
    )
    .await?;

    Ok(LlmProviderTestResult {
        profile_id: profile.id,
        ok: true,
        message: format!(
            "Profile responded with {} characters.",
            completion.content.len()
        ),
        model: profile.model,
        base_url: normalize_chat_completions_url(
            profile
                .base_url
                .as_deref()
                .unwrap_or_else(|| default_base_url(profile.kind)),
        )?,
    })
}

pub async fn chat_completion(
    profile: &LlmProviderProfile,
    messages: Vec<ChatMessage>,
    json_output: bool,
) -> AppResult<ChatCompletion> {
    if !profile.enabled {
        return Err(AppError::new(
            "llm_profile_disabled",
            "Enable the selected summary profile before using it.",
        ));
    }

    let endpoint = normalize_chat_completions_url(
        profile
            .base_url
            .as_deref()
            .unwrap_or_else(|| default_base_url(profile.kind)),
    )?;
    let secret = load_profile_secret(&profile.id)?.map(|secret| secret.key);
    if requires_api_key(profile.kind) && secret.is_none() {
        return Err(AppError::new(
            "missing_llm_api_key",
            "Save an API key for this summary provider profile.",
        ));
    }

    let payload_messages = messages
        .into_iter()
        .map(|message| json!({ "role": message.role, "content": message.content }))
        .collect::<Vec<_>>();
    let mut payload = json!({
        "model": profile.model,
        "messages": payload_messages,
        "temperature": profile.temperature,
        "max_tokens": profile.max_output_tokens,
    });
    if json_output {
        payload["response_format"] = json!({ "type": "json_object" });
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(profile.timeout_seconds.max(5)))
        .build()
        .map_err(|err| AppError::new("llm_client_error", err.to_string()))?;
    let mut request = client.post(endpoint).json(&payload);
    if let Some(secret) = secret {
        request = request.bearer_auth(secret);
    }

    let response = request
        .send()
        .await
        .map_err(|err| AppError::new("llm_request_error", err.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::new("llm_response_error", err.to_string()))?;

    if status != StatusCode::OK {
        return Err(AppError::new(
            "llm_provider_error",
            format!("Provider returned {status}: {}", compact_error_body(&body)),
        ));
    }

    parse_chat_completion_body(&body)
}

pub fn normalize_chat_completions_url(base_url: &str) -> AppResult<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::new(
            "missing_llm_base_url",
            "Enter a provider base URL.",
        ));
    }
    let normalized = match trimmed.strip_prefix("http://ollama.com") {
        Some("") => "https://ollama.com".to_string(),
        Some(path) if path.starts_with('/') => format!("https://ollama.com{path}"),
        _ => trimmed.to_string(),
    };
    if normalized.ends_with("/chat/completions") {
        return Ok(normalized);
    }
    Ok(format!("{normalized}/chat/completions"))
}

pub fn parse_json_object(text: &str) -> AppResult<Value> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    let without_fence = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim);
    if let Some(candidate) = without_fence {
        if let Ok(value) = serde_json::from_str::<Value>(candidate) {
            return Ok(value);
        }
    }

    let Some(start) = trimmed.find('{') else {
        return Err(AppError::new(
            "summary_agent_parse_error",
            "The summary model did not return JSON.",
        ));
    };
    let Some(end) = trimmed.rfind('}') else {
        return Err(AppError::new(
            "summary_agent_parse_error",
            "The summary model returned incomplete JSON.",
        ));
    };
    serde_json::from_str::<Value>(&trimmed[start..=end]).map_err(|err| {
        AppError::new(
            "summary_agent_parse_error",
            format!("The summary model returned malformed JSON: {err}"),
        )
    })
}

fn parse_chat_completion_body(body: &str) -> AppResult<ChatCompletion> {
    let value: Value = serde_json::from_str(body)
        .map_err(|err| AppError::new("llm_response_parse_error", err.to_string()))?;
    let content = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    if content.is_empty() {
        return Err(AppError::new(
            "llm_empty_response",
            "The provider returned an empty chat completion.",
        ));
    }
    Ok(ChatCompletion { content })
}

fn normalize_profile_base_url(
    kind: LlmProviderKind,
    base_url: Option<&str>,
) -> AppResult<Option<String>> {
    let value = base_url.map(str::trim).filter(|value| !value.is_empty());
    match kind {
        LlmProviderKind::Openai => Ok(Some(value.unwrap_or(OPENAI_BASE_URL).to_string())),
        LlmProviderKind::Ollama => Ok(Some(value.unwrap_or(OLLAMA_BASE_URL).to_string())),
        LlmProviderKind::OpenaiCompatible | LlmProviderKind::AdkLitellm => value
            .map(|value| Ok(Some(value.to_string())))
            .unwrap_or_else(|| {
                Err(AppError::new(
                    "missing_llm_base_url",
                    "Enter a base URL for this provider profile.",
                ))
            }),
    }
}

fn default_base_url(kind: LlmProviderKind) -> &'static str {
    match kind {
        LlmProviderKind::Openai => OPENAI_BASE_URL,
        LlmProviderKind::Ollama => OLLAMA_BASE_URL,
        LlmProviderKind::OpenaiCompatible | LlmProviderKind::AdkLitellm => "",
    }
}

fn requires_api_key(kind: LlmProviderKind) -> bool {
    matches!(
        kind,
        LlmProviderKind::Openai | LlmProviderKind::OpenaiCompatible
    )
}

fn read_store() -> AppResult<ProfileStore> {
    let path = profile_store_path()?;
    if !path.exists() {
        return Ok(ProfileStore::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|err| AppError::new("llm_config_read_error", err.to_string()))?;
    serde_json::from_str(&raw).map_err(|err| {
        AppError::new(
            "llm_config_parse_error",
            format!("Could not parse {}: {err}", path.display()),
        )
    })
}

fn write_store(store: &ProfileStore) -> AppResult<()> {
    let path = profile_store_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| AppError::new("llm_config_write_error", err.to_string()))?;
    }
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| AppError::new("llm_config_write_error", err.to_string()))?;
    std::fs::write(path, raw)
        .map_err(|err| AppError::new("llm_config_write_error", err.to_string()))
}

fn profile_store_path() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA").ok_or_else(|| {
            AppError::new(
                "llm_config_path_error",
                "Could not resolve APPDATA for summary profile storage.",
            )
        })?;
        Ok(PathBuf::from(app_data)
            .join(CONFIG_DIR_NAME)
            .join(PROFILE_FILE_NAME))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME").ok_or_else(|| {
            AppError::new(
                "llm_config_path_error",
                "Could not resolve HOME for summary profile storage.",
            )
        })?;
        Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(CONFIG_DIR_NAME)
            .join(PROFILE_FILE_NAME))
    }
}

fn save_profile_secret(profile_id: &str, api_key: &str) -> AppResult<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "invalid_llm_api_key",
            "LLM API key cannot be empty.",
        ));
    }
    let entry = keyring::Entry::new(SERVICE, &profile_secret_user(profile_id))
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    entry
        .set_password(trimmed)
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    cache_profile_secret(profile_id, trimmed)
}

fn load_profile_secret(profile_id: &str) -> AppResult<Option<ProfileSecretInfo>> {
    let entry = keyring::Entry::new(SERVICE, &profile_secret_user(profile_id))
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    match entry.get_password() {
        Ok(secret) => {
            let secret = secret.trim().to_string();
            if secret.is_empty() {
                return cached_profile_secret_info(profile_id);
            }
            cache_profile_secret(profile_id, &secret)?;
            Ok(Some(ProfileSecretInfo {
                key: secret,
                source: "keychain",
            }))
        }
        Err(keyring::Error::NoEntry) => cached_profile_secret_info(profile_id),
        Err(err) => cached_profile_secret_info(profile_id)?.map_or_else(
            || Err(AppError::new("keychain_error", err.to_string())),
            |secret| Ok(Some(secret)),
        ),
    }
}

fn delete_profile_secret(profile_id: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(SERVICE, &profile_secret_user(profile_id))
        .map_err(|err| AppError::new("keychain_error", err.to_string()))?;
    clear_cached_profile_secret(profile_id)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(AppError::new("keychain_error", err.to_string())),
    }
}

fn profile_secret_user(profile_id: &str) -> String {
    format!("{PROFILE_SECRET_PREFIX}{profile_id}")
}

fn profile_secret_cache() -> &'static Mutex<HashMap<String, String>> {
    PROFILE_SECRET_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cache_profile_secret(profile_id: &str, api_key: &str) -> AppResult<()> {
    profile_secret_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))?
        .insert(profile_id.to_string(), api_key.to_string());
    Ok(())
}

fn cached_profile_secret_info(profile_id: &str) -> AppResult<Option<ProfileSecretInfo>> {
    Ok(profile_secret_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))?
        .get(profile_id)
        .cloned()
        .map(|key| ProfileSecretInfo {
            key,
            source: "memory",
        }))
}

fn clear_cached_profile_secret(profile_id: &str) -> AppResult<()> {
    profile_secret_cache()
        .lock()
        .map_err(|err| AppError::new("credential_cache_error", err.to_string()))?
        .remove(profile_id);
    Ok(())
}

fn require_non_empty(value: &str, code: &str, message: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(code, message));
    }
    Ok(trimmed.to_string())
}

fn non_empty_or(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
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

fn compact_error_body(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        cache_profile_secret, cached_profile_secret_info, clear_cached_profile_secret,
        normalize_chat_completions_url, parse_chat_completion_body, parse_json_object,
    };

    #[test]
    fn normalizes_base_url_to_chat_completions() {
        let endpoint = normalize_chat_completions_url("http://localhost:11434/v1/").unwrap();
        assert_eq!(endpoint, "http://localhost:11434/v1/chat/completions");
    }

    #[test]
    fn keeps_full_chat_completions_endpoint() {
        let endpoint =
            normalize_chat_completions_url("https://example.test/v1/chat/completions").unwrap();
        assert_eq!(endpoint, "https://example.test/v1/chat/completions");
    }

    #[test]
    fn upgrades_ollama_cloud_base_url_to_https() {
        let endpoint = normalize_chat_completions_url("http://ollama.com/v1").unwrap();
        assert_eq!(endpoint, "https://ollama.com/v1/chat/completions");
    }

    #[test]
    fn parses_chat_completion_content() {
        let body = r#"{"choices":[{"message":{"content":"{\"ok\":true}"}}]}"#;
        let parsed = parse_chat_completion_body(body).unwrap();
        assert_eq!(parsed.content, "{\"ok\":true}");
    }

    #[test]
    fn repairs_json_from_fenced_response() {
        let parsed = parse_json_object("```json\n{\"summary\":\"done\"}\n```").unwrap();
        assert_eq!(parsed["summary"], "done");
    }

    #[test]
    fn caches_summary_profile_secrets_for_read_back() {
        let profile_id = "summary-profile-cache-test";
        clear_cached_profile_secret(profile_id).unwrap();

        cache_profile_secret(profile_id, "sk-summary-cache-value").unwrap();
        let cached = cached_profile_secret_info(profile_id).unwrap().unwrap();

        assert_eq!(cached.key, "sk-summary-cache-value");
        assert_eq!(cached.source, "memory");

        clear_cached_profile_secret(profile_id).unwrap();
        assert!(cached_profile_secret_info(profile_id).unwrap().is_none());
    }
}
