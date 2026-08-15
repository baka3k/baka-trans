use crate::error::{AppError, AppResult};
use crate::models::Language;
use reqwest::redirect::Policy;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::net::IpAddr;
use std::time::{Duration, Instant};
use url::Url;

const MAX_SOURCE_TEXT_CHARS: usize = 10_000;
const MAX_RESPONSE_BODY_BYTES: usize = 1_048_576;

#[derive(Clone, Debug)]
pub struct OpenAiCompatibleClient {
    client: Client,
    endpoint: String,
    model: String,
    temperature: f32,
    max_output_tokens: u32,
    system_prompt: String,
    api_key: Option<String>,
}

impl OpenAiCompatibleClient {
    pub fn new(
        base_url: &str,
        model: &str,
        timeout_seconds: u64,
        temperature: f32,
        max_output_tokens: u32,
        api_key: Option<String>,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        let endpoint = normalize_openai_chat_completions_url(base_url)?;
        if target_language == Language::Auto {
            return Err(AppError::new(
                "unsupported_target_language",
                "Choose an explicit target language for local translation.",
            ));
        }
        let is_loopback = is_loopback_url(&endpoint)?;
        if !is_loopback && !endpoint.starts_with("https://") {
            return Err(AppError::new(
                "local_openai_tls_required",
                "HTTPS is required for non-loopback OpenAI-compatible endpoints.",
            ));
        }
        if api_key.is_some() && !is_loopback && endpoint.starts_with("http://") {
            return Err(AppError::new(
                "local_openai_key_over_http",
                "An API key cannot be sent over an unencrypted HTTP connection.",
            ));
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(
                timeout_seconds.max(5).min(300),
            ))
            .redirect(Policy::none())
            .build()
            .map_err(|err| AppError::new("local_openai_client_error", err.to_string()))?;

        Ok(Self {
            client,
            endpoint,
            model: model.to_string(),
            temperature,
            max_output_tokens,
            system_prompt: translation_system_prompt(source_language, target_language),
            api_key,
        })
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn translate(&self, source_text: &str) -> AppResult<(String, u64)> {
        let source_text = source_text.trim();
        if source_text.is_empty() {
            return Err(AppError::new(
                "local_translation_empty_source",
                "Whisper returned no text to translate.",
            ));
        }
        if source_text.chars().count() > MAX_SOURCE_TEXT_CHARS {
            return Err(AppError::new(
                "local_openai_text_too_long",
                format!(
                    "Source text exceeds the maximum of {MAX_SOURCE_TEXT_CHARS} characters."
                ),
            ));
        }

        let payload = json!({
            "model": self.model,
            "stream": false,
            "messages": [
                { "role": "system", "content": self.system_prompt },
                { "role": "user", "content": source_text }
            ],
            "temperature": self.temperature,
            "max_tokens": self.max_output_tokens,
        });

        let started = Instant::now();
        let mut request = self.client.post(&self.endpoint).json(&payload);

        let is_loopback = is_loopback_url(&self.endpoint).unwrap_or(false);
        if let Some(ref key) = self.api_key {
            if is_loopback || self.endpoint.starts_with("https://") {
                request = request.bearer_auth(key);
            }
        }

        let response = request
            .send()
            .await
            .map_err(|err| AppError::new("local_openai_request_error", redact_error(err.to_string())))?;

        let status = response.status();
        let body_bytes = response
            .bytes()
            .await
            .map_err(|err| AppError::new("local_openai_response_error", redact_error(err.to_string())))?;

        if body_bytes.len() > MAX_RESPONSE_BODY_BYTES {
            return Err(AppError::new(
                "local_openai_response_too_large",
                format!(
                    "The provider response exceeds the maximum of {} bytes.",
                    MAX_RESPONSE_BODY_BYTES
                ),
            ));
        }

        let body = String::from_utf8_lossy(&body_bytes);
        let content = parse_openai_chat_completion_response(status, &body)?;
        let latency_ms = started.elapsed().as_millis().min(86_400_000) as u64;
        Ok((content, latency_ms))
    }
}

pub fn normalize_openai_chat_completions_url(base_url: &str) -> AppResult<String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::new(
            "local_openai_base_url_missing",
            "Enter an OpenAI-compatible base URL.",
        ));
    }

    let mut url = Url::parse(trimmed).map_err(|err| {
        AppError::new(
            "local_openai_base_url_invalid",
            format!("Enter a valid URL: {err}"),
        )
    })?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(AppError::new(
            "local_openai_base_url_invalid",
            "OpenAI-compatible URL must use http or https.",
        ));
    }

    if !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::new(
            "local_openai_url_credentials",
            "OpenAI-compatible URL must not contain embedded credentials.",
        ));
    }

    if url.fragment().is_some() {
        return Err(AppError::new(
            "local_openai_url_fragment",
            "OpenAI-compatible URL must not contain a fragment.",
        ));
    }

    url.set_query(None);
    url.set_fragment(None);

    let path = url.path().trim_end_matches('/');
    if path.ends_with("/chat/completions") {
        return Ok(url.to_string().trim_end_matches('/').to_string());
    }

    if path.ends_with("/v1") {
        url.set_path(&format!("{path}/chat/completions"));
    } else if path.is_empty() || path == "/" {
        url.set_path("/v1/chat/completions");
    } else {
        url.set_path(&format!("{path}/v1/chat/completions"));
    }

    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn is_loopback_url(url_str: &str) -> AppResult<bool> {
    let url = Url::parse(url_str).map_err(|err| {
        AppError::new(
            "local_openai_base_url_invalid",
            format!("Invalid URL: {err}"),
        )
    })?;
    let host = url.host_str().unwrap_or("");
    if matches!(host, "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::1]") {
        return Ok(true);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip.is_loopback());
    }
    Ok(false)
}

fn parse_openai_chat_completion_response(status: StatusCode, body: &str) -> AppResult<String> {
    let parsed: Result<Value, _> = serde_json::from_str(body);

    if status != StatusCode::OK {
        let detail = match &parsed {
            Ok(value) => value
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown provider error")
                .to_string(),
            Err(_) => "The provider returned a non-JSON error response.".to_string(),
        };
        return Err(AppError::new(
            "local_openai_provider_error",
            format!("Provider returned {status}: {}", compact_redacted(&detail)),
        ));
    }

    let value = parsed.map_err(|err| {
        AppError::new(
            "local_openai_response_parse_error",
            format!("Provider returned malformed JSON: {err}"),
        )
    })?;

    if let Some(error) = value.get("error") {
        let detail = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(AppError::new(
            "local_openai_provider_error",
            format!("Provider error: {}", compact_redacted(detail)),
        ));
    }

    let content = value
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or_default()
        .trim()
        .to_string();

    if content.is_empty() {
        return Err(AppError::new(
            "local_openai_empty_response",
            "The provider returned an empty chat completion.",
        ));
    }
    Ok(content)
}

fn translation_system_prompt(source_language: Language, target_language: Language) -> String {
    let source = if source_language == Language::Auto {
        "the automatically detected source language".to_string()
    } else {
        format!("language code '{}'", source_language.realtime_code())
    };
    format!(
        "Translate from {source} to language code '{}'. Return only the translation. Preserve names, numbers, and technical terms. Do not explain.",
        target_language.realtime_code()
    )
}

fn redact_error(message: String) -> String {
    let mut redacted = message;
    for prefix in &["sk-", "Bearer ", "api-key=", "key=", "token="] {
        if let Some(pos) = redacted.find(prefix) {
            let after = pos + prefix.len();
            let end = redacted[after..]
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '&')
                .map(|i| after + i)
                .unwrap_or(redacted.len());
            if end > after {
                redacted.replace_range(after..end, "[REDACTED]");
            }
        }
    }
    redacted
}

fn compact_redacted(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_bare_origin_to_chat_completions() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://api.example.com").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_v1_base_to_chat_completions() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://api.example.com/v1").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn normalizes_v1_with_trailing_slash() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://api.example.com/v1/").unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn preserves_existing_chat_completions_path() {
        assert_eq!(
            normalize_openai_chat_completions_url(
                "https://api.example.com/v1/chat/completions"
            )
            .unwrap(),
            "https://api.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn preserves_custom_path_prefix() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://api.example.com/custom/v1").unwrap(),
            "https://api.example.com/custom/v1/chat/completions"
        );
    }

    #[test]
    fn rejects_empty_url() {
        assert_eq!(
            normalize_openai_chat_completions_url("").unwrap_err().code,
            "local_openai_base_url_missing"
        );
    }

    #[test]
    fn rejects_non_http_scheme() {
        assert_eq!(
            normalize_openai_chat_completions_url("ftp://example.com")
                .unwrap_err()
                .code,
            "local_openai_base_url_invalid"
        );
    }

    #[test]
    fn rejects_url_with_credentials() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://user:pass@api.example.com")
                .unwrap_err()
                .code,
            "local_openai_url_credentials"
        );
    }

    #[test]
    fn rejects_url_with_fragment() {
        assert_eq!(
            normalize_openai_chat_completions_url("https://api.example.com/v1#section")
                .unwrap_err()
                .code,
            "local_openai_url_fragment"
        );
    }

    #[test]
    fn strips_query_parameters() {
        let result =
            normalize_openai_chat_completions_url("https://api.example.com/v1?token=secret")
                .unwrap();
        assert!(!result.contains("token"));
        assert!(result.ends_with("/chat/completions"));
    }

    #[test]
    fn identifies_loopback_addresses() {
        assert!(is_loopback_url("http://localhost:8080/v1").unwrap());
        assert!(is_loopback_url("http://127.0.0.1:11434/v1").unwrap());
        assert!(is_loopback_url("http://[::1]:8080/v1").unwrap());
        assert!(!is_loopback_url("https://api.example.com/v1").unwrap());
        assert!(!is_loopback_url("https://10.0.0.1:8080/v1").unwrap());
    }

    #[test]
    fn client_rejects_http_for_non_loopback() {
        let result = OpenAiCompatibleClient::new(
            "http://api.example.com",
            "model",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        );
        assert_eq!(result.unwrap_err().code, "local_openai_tls_required");
    }

    #[test]
    fn client_allows_http_for_loopback() {
        let result = OpenAiCompatibleClient::new(
            "http://localhost:8080",
            "model",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn client_rejects_api_key_over_http_non_loopback() {
        let result = OpenAiCompatibleClient::new(
            "https://api.example.com",
            "model",
            30,
            0.0,
            256,
            Some("sk-test-key".to_string()),
            Language::Ja,
            Language::Vi,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn parses_successful_chat_completion() {
        let body = r#"{"choices":[{"message":{"content":"Xin chào"}}]}"#;
        assert_eq!(
            parse_openai_chat_completion_response(StatusCode::OK, body).unwrap(),
            "Xin chào"
        );
    }

    #[test]
    fn rejects_empty_completion_content() {
        let body = r#"{"choices":[{"message":{"content":"  "}}]}"#;
        assert_eq!(
            parse_openai_chat_completion_response(StatusCode::OK, body)
                .unwrap_err()
                .code,
            "local_openai_empty_response"
        );
    }

    #[test]
    fn parses_provider_error_response() {
        let body = r#"{"error":{"message":"Invalid model: unknown-model"}}"#;
        let err = parse_openai_chat_completion_response(StatusCode::BAD_REQUEST, body).unwrap_err();
        assert_eq!(err.code, "local_openai_provider_error");
        assert!(err.message.contains("Invalid model"));
    }

    #[test]
    fn handles_malformed_json() {
        let err =
            parse_openai_chat_completion_response(StatusCode::OK, "not json").unwrap_err();
        assert_eq!(err.code, "local_openai_response_parse_error");
    }

    #[test]
    fn handles_non_json_error_status() {
        let err = parse_openai_chat_completion_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error",
        )
        .unwrap_err();
        assert_eq!(err.code, "local_openai_provider_error");
    }

    #[test]
    fn redacts_secrets_from_error_messages() {
        let msg = redact_error("Authentication failed for sk-abc123def456ghi".to_string());
        assert!(!msg.contains("abc123def456"));
        assert!(msg.contains("[REDACTED]"));

        let msg2 = redact_error("Bearer token-xyz789 rejected".to_string());
        assert!(!msg2.contains("token-xyz789"));
        assert!(msg2.contains("[REDACTED]"));
    }

    #[test]
    fn rejects_oversized_source_text() {
        let long_text: String = "a".repeat(MAX_SOURCE_TEXT_CHARS + 1);
        let client = OpenAiCompatibleClient::new(
            "http://localhost:8080",
            "model",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        )
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(client.translate(&long_text));
        assert_eq!(
            result.unwrap_err().code,
            "local_openai_text_too_long"
        );
    }

    #[test]
    fn rejects_empty_source_text() {
        let client = OpenAiCompatibleClient::new(
            "http://localhost:8080",
            "model",
            30,
            0.0,
            256,
            None,
            Language::Ja,
            Language::Vi,
        )
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(client.translate(""));
        assert_eq!(
            result.unwrap_err().code,
            "local_translation_empty_source"
        );
    }
}
