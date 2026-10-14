use crate::error::{AppError, AppResult};
use crate::models::Language;
use reqwest::redirect::Policy;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_MODEL: &str = "gemini-2.5-flash-lite";
pub(crate) const API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";
const REQUEST_TIMEOUT_SECONDS: u64 = 30;
const MAX_RESPONSE_BODY_BYTES: usize = 1_048_576;
/// Interpretation constants are engine behavior, not user settings: a cabin
/// interpreter should stay close to the source with low randomness, and the
/// batch response is bounded by the window size (5 × 60 chars in, ~2× out).
const TRANSLATION_TEMPERATURE: f32 = 0.3;
const TRANSLATION_MAX_OUTPUT_TOKENS: u32 = 4_096;

/// Gemini translation client for the local Whisper workspace. One
/// `generateContent` call carries a whole JSON array of Whisper sentences so
/// the model can stitch ASR fragments with cabin-interpreter context.
#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    model: String,
    system_prompt: String,
    api_key: String,
    api_base: String,
}

impl std::fmt::Debug for GeminiClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never include the API key in debug output.
        formatter
            .debug_struct("GeminiClient")
            .field("model", &self.model)
            .finish()
    }
}

impl GeminiClient {
    pub fn new(
        model: &str,
        api_key: String,
        source_language: Language,
        target_language: Language,
    ) -> AppResult<Self> {
        if api_key.trim().is_empty() {
            return Err(AppError::new(
                "local_gemini_api_key_missing",
                gemini_key_missing_message(),
            ));
        }
        if target_language == Language::Auto {
            return Err(AppError::new(
                "unsupported_target_language",
                "Choose an explicit target language for local translation.",
            ));
        }
        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
            .redirect(Policy::none())
            .build()
            .map_err(|err| {
                AppError::new(
                    "local_gemini_request_error",
                    redact_message(&err.to_string(), &api_key),
                )
            })?;
        Ok(Self {
            client,
            model: normalize_gemini_model(model),
            system_prompt: translation_system_prompt(source_language, target_language),
            api_key: api_key.trim().to_string(),
            api_base: API_BASE.to_string(),
        })
    }

    /// Test-only origin override so crate tests can point the client at a
    /// loopback capture server instead of Google's API.
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, api_base: String) -> Self {
        self.api_base = api_base;
        self
    }
    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn endpoint(&self) -> String {
        generate_content_endpoint(&self.api_base, &self.model)
    }

    /// Single-sentence translation. It rides the same JSON-array contract as
    /// the batch path so production exercises exactly one request shape.
    pub async fn translate(&self, source_text: &str) -> AppResult<(String, u64)> {
        let batch = self.translate_batch(&[source_text.to_string()]).await;
        batch.into_iter().next().unwrap_or_else(|| {
            Err(AppError::new(
                "local_gemini_empty_response",
                "Gemini returned no translation.",
            ))
        })
    }

    /// Translate a window of sentences with one request. Returns one outcome
    /// per input in arrival order: request-level failures repeat the same
    /// error for every item, and a length mismatch maps returned indexes 1:1 —
    /// missing indexes get `local_gemini_batch_length_mismatch` errors while
    /// extra results are truncated.
    pub async fn translate_batch(&self, texts: &[String]) -> Vec<AppResult<(String, u64)>> {
        if texts.is_empty() {
            return Vec::new();
        }
        let trimmed: Vec<String> = texts.iter().map(|text| text.trim().to_string()).collect();
        if trimmed.iter().any(|text| text.is_empty()) {
            // An empty element would break the 1:1 index mapping, so the whole
            // window fails with the same error the per-sentence engines use.
            let error = AppError::new(
                "local_translation_empty_source",
                "Whisper returned no text to translate.",
            );
            return vec![Err(error); trimmed.len()];
        }

        let payload = json!({
            "system_instruction": { "parts": [{ "text": self.system_prompt }] },
            "contents": [{ "parts": [{ "text": serde_json::to_string(&trimmed).unwrap_or_default() }] }],
            "generationConfig": {
                "temperature": TRANSLATION_TEMPERATURE,
                "maxOutputTokens": TRANSLATION_MAX_OUTPUT_TOKENS,
                "response_mime_type": "application/json",
                // 2.5-family models can spend time thinking by default; an
                // explicit zero budget keeps interpretation latency flat.
                "thinkingConfig": { "thinkingBudget": 0 },
            },
        });

        let started = Instant::now();
        let send_result = self
            .client
            .post(self.endpoint())
            // Header auth, not the overlay's `?key=` query param: query
            // parameters leak into URLs and logs.
            .header("x-goog-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await;
        let response = match send_result {
            Ok(response) => response,
            Err(err) => {
                return vec![
                    Err(AppError::new(
                        "local_gemini_request_error",
                        redact_message(&err.to_string(), &self.api_key),
                    ));
                    trimmed.len()
                ]
            }
        };
        let status = response.status();
        let body = match response.bytes().await {
            Ok(bytes) if bytes.len() <= MAX_RESPONSE_BODY_BYTES => {
                String::from_utf8_lossy(&bytes).into_owned()
            }
            Ok(_) => {
                return vec![Err(AppError::new(
                    "local_gemini_response_too_large",
                    format!(
                        "The Gemini response exceeds the maximum of {MAX_RESPONSE_BODY_BYTES} bytes."
                    ),
                )); trimmed.len()]
            }
            Err(err) => {
                return vec![Err(AppError::new(
                    "local_gemini_response_error",
                    redact_message(&err.to_string(), &self.api_key),
                )); trimmed.len()]
            }
        };
        let latency_ms = started.elapsed().as_millis().min(86_400_000) as u64;

        let text = match parse_generate_content_text(status, &body, &self.api_key) {
            Ok(text) => text,
            Err(error) => return vec![Err(error); trimmed.len()],
        };
        let translations = match parse_translation_array(&text) {
            Ok(translations) => translations,
            Err(error) => return vec![Err(error); trimmed.len()],
        };

        trimmed
            .iter()
            .enumerate()
            .map(|(index, _)| match translations.get(index) {
                Some(translated) => Ok((translated.clone(), latency_ms)),
                None => Err(AppError::new(
                    "local_gemini_batch_length_mismatch",
                    format!(
                        "Gemini returned {} translations for {} sentences.",
                        translations.len(),
                        trimmed.len()
                    ),
                )),
            })
            .collect()
    }
}

/// Cheap key probe used by the engine-test button: lists models with header
/// auth, which validates the credential without paying for a translation.
pub(crate) async fn probe_key(api_key: &str) -> AppResult<()> {
    probe_key_at(&format!("{API_BASE}/models"), api_key).await
}

async fn probe_key_at(models_url: &str, api_key: &str) -> AppResult<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECONDS))
        .redirect(Policy::none())
        .build()
        .map_err(|err| {
            AppError::new(
                "local_gemini_request_error",
                redact_message(&err.to_string(), api_key),
            )
        })?;
    let response = client
        .get(models_url)
        .header("x-goog-api-key", api_key.trim())
        .send()
        .await
        .map_err(|err| {
            AppError::new(
                "local_gemini_request_error",
                redact_message(&err.to_string(), api_key),
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        AppError::new(
            "local_gemini_response_error",
            redact_message(&err.to_string(), api_key),
        )
    })?;
    if !status.is_success() {
        let detail = extract_google_error_message(&body).unwrap_or(body);
        return Err(AppError::new(
            "local_gemini_api_error",
            format!(
                "Google AI rejected the credential: {}",
                compact_redacted(&redact_message(&detail, api_key))
            ),
        ));
    }
    Ok(())
}

pub(crate) fn generate_content_endpoint(api_base: &str, model: &str) -> String {
    format!("{api_base}/models/{model}:generateContent")
}

/// Trim, strip the `models/` REST prefix, and fall back to the pinned default
/// when nothing usable remains — mirrors the overlay's model handling so a
/// cleared field can never persist an unusable model name. The result is
/// restricted to URL-path-safe model-name characters because it is
/// interpolated into the generateContent endpoint.
pub(crate) fn normalize_gemini_model(model: &str) -> String {
    let trimmed = model.trim().trim_start_matches("models/");
    let url_safe = !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if url_safe {
        trimmed.to_string()
    } else {
        DEFAULT_MODEL.to_string()
    }
}

pub(crate) fn gemini_key_missing_message() -> String {
    "Gemini translation needs a Google AI API key. Save a key in the Cloud settings panel or set the GEMINI_API_KEY environment variable.".to_string()
}

fn translation_system_prompt(source_language: Language, target_language: Language) -> String {
    let source = if source_language == Language::Auto {
        "the automatically detected source language".to_string()
    } else {
        format!("language code '{}'", source_language.realtime_code())
    };
    format!(
        "You are an expert simultaneous interpreter working in an interpreter booth for a live meeting. \
Translate from {source} to language code '{target}'. \
The user message is a JSON array of consecutive sentences captured by live speech recognition: treat the list as one continuous flow of speech, not independent lines. \
Stitch fragments that were cut awkwardly mid-thought and keep names and terminology consistent across the whole list. \
Translate each array element into a natural, conversational, fluid interpretation - the way a fluent speaker would actually say it. \
Return exactly one JSON array of translated strings with the same length and order as the input array; never merge, split, reorder, add, or drop elements. \
No markdown fences, no explanations, no text outside the JSON array.",
        source = source,
        target = target_language.realtime_code(),
    )
}

fn parse_generate_content_text(status: StatusCode, body: &str, api_key: &str) -> AppResult<String> {
    if !status.is_success() {
        let detail = extract_google_error_message(body).unwrap_or_else(|| body.to_string());
        return Err(AppError::new(
            "local_gemini_api_error",
            format!(
                "Gemini returned {status}: {}",
                compact_redacted(&redact_message(&detail, api_key))
            ),
        ));
    }
    let value: Value = serde_json::from_str(body).map_err(|err| {
        AppError::new(
            "local_gemini_parse_error",
            redact_message(&err.to_string(), api_key),
        )
    })?;
    let text = value
        .get("candidates")
        .and_then(Value::as_array)
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return Err(AppError::new(
            "local_gemini_empty_response",
            "Gemini returned an empty response.",
        ));
    }
    Ok(cleaned.to_string())
}

/// Repair and parse the model's JSON-array payload: strip an optional code
/// fence, then require an array of strings (the 1:1 mapping contract).
fn parse_translation_array(text: &str) -> AppResult<Vec<String>> {
    let candidate = strip_code_fence(text);
    let parsed: Value = serde_json::from_str(candidate).map_err(|err| {
        AppError::new(
            "local_gemini_parse_error",
            format!("Gemini did not return a JSON array: {err}"),
        )
    })?;
    let Some(items) = parsed.as_array() else {
        return Err(AppError::new(
            "local_gemini_parse_error",
            "Gemini did not return a JSON array.",
        ));
    };
    items
        .iter()
        .map(|item| {
            item.as_str().map(str::to_string).ok_or_else(|| {
                AppError::new(
                    "local_gemini_parse_error",
                    "Gemini returned a non-string array element.",
                )
            })
        })
        .collect()
}

fn strip_code_fence(text: &str) -> &str {
    let trimmed = text.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|rest| rest.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed)
}

fn extract_google_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<Value>(body).ok().and_then(|value| {
        value
            .pointer("/error/message")
            .or_else(|| value.get("message"))
            .and_then(Value::as_str)
            .map(ToString::to_string)
    })
}

/// Scrub the credential from every message that could reach the UI or logs:
/// exact-match replacement first, then the key's tail (reqwest errors can
/// embed URL fragments) and the generic prefix shapes the OpenAI-compatible
/// client also guards against.
fn redact_message(message: &str, api_key: &str) -> String {
    let mut redacted = message.replace(api_key, "[REDACTED]");
    let char_count = api_key.chars().count();
    if char_count >= 8 {
        let tail: String = api_key.chars().skip(char_count - 8).collect();
        redacted = redacted.replace(&tail, "[REDACTED]");
    }
    for prefix in ["Bearer ", "api-key=", "key="] {
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
    use crate::local_translation::test_http::spawn_capture_server;

    fn client_with_base(base: &str) -> GeminiClient {
        GeminiClient::new(
            "gemini-2.5-flash-lite",
            "test-secret-key-123456".to_string(),
            Language::Ja,
            Language::Vi,
        )
        .unwrap()
        .with_api_base(base.to_string())
    }

    #[tokio::test]
    async fn batch_request_uses_header_auth_json_mode_and_array_body() {
        let response_body =
            r#"{"candidates":[{"content":{"parts":[{"text":"[\"Xin chào\",\"Cảm ơn\"]"}]}}]}"#;
        let (addr, server) = spawn_capture_server("HTTP/1.1 200 OK", response_body).await;
        let client = client_with_base(&format!("http://{addr}/v1beta"));

        let results = client
            .translate_batch(&["こんにちは".to_string(), "ありがとう".to_string()])
            .await;

        let request = server.await.unwrap();
        let head = request.head();
        assert!(
            head.contains("POST /v1beta/models/gemini-2.5-flash-lite:generateContent"),
            "unexpected request line: {head}"
        );
        assert!(
            head.to_lowercase()
                .contains("x-goog-api-key: test-secret-key-123456"),
            "request must authenticate via the x-goog-api-key header: {head}"
        );
        assert!(
            !head.contains("key=test-secret"),
            "the key must never ride the query string"
        );

        let payload: Value =
            serde_json::from_slice(request.body()).expect("request body must be JSON");
        let system_prompt = payload
            .pointer("/system_instruction/parts/0/text")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert!(
            system_prompt.contains("interpreter booth")
                && system_prompt.contains("'ja'")
                && system_prompt.contains("'vi'")
                && system_prompt.contains("same length and order"),
            "system prompt must be the parameterized cabin-interpreter prompt: {system_prompt}"
        );
        assert_eq!(
            payload
                .pointer("/contents/0/parts/0/text")
                .and_then(Value::as_str),
            Some(r#"["こんにちは","ありがとう"]"#)
        );
        assert_eq!(
            payload
                .pointer("/generationConfig/response_mime_type")
                .and_then(Value::as_str),
            Some("application/json")
        );
        // f32 0.3 does not round-trip exactly through JSON, so compare with a
        // tolerance against the engine constant.
        let temperature = payload
            .pointer("/generationConfig/temperature")
            .and_then(Value::as_f64)
            .unwrap_or_default();
        assert!((temperature - f64::from(TRANSLATION_TEMPERATURE)).abs() < 1e-6);
        assert_eq!(
            payload
                .pointer("/generationConfig/maxOutputTokens")
                .and_then(Value::as_u64),
            Some(4_096)
        );
        assert_eq!(
            payload
                .pointer("/generationConfig/thinkingConfig/thinkingBudget")
                .and_then(Value::as_u64),
            Some(0)
        );

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().0, "Xin chào");
        assert_eq!(results[1].as_ref().unwrap().0, "Cảm ơn");
    }

    #[tokio::test]
    async fn batch_length_mismatch_maps_missing_and_truncates_extras() {
        // Short response: the missing index errors, nothing else is disturbed.
        let (short_addr, short_server) = spawn_capture_server(
            "HTTP/1.1 200 OK",
            r#"{"candidates":[{"content":{"parts":[{"text":"[\"Xin chào\"]"}]}}]}"#,
        )
        .await;
        let client = client_with_base(&format!("http://{short_addr}/v1beta"));
        let results = client
            .translate_batch(&["一".to_string(), "二".to_string()])
            .await;
        short_server.await.unwrap();
        assert_eq!(results[0].as_ref().unwrap().0, "Xin chào");
        assert_eq!(
            results[1].as_ref().unwrap_err().code,
            "local_gemini_batch_length_mismatch"
        );

        // Long response: extra results are truncated to the input length.
        let (long_addr, long_server) = spawn_capture_server(
            "HTTP/1.1 200 OK",
            r#"{"candidates":[{"content":{"parts":[{"text":"[\"Một\",\"Hai\",\"Ba\"]"}]}}]}"#,
        )
        .await;
        let client = client_with_base(&format!("http://{long_addr}/v1beta"));
        let results = client
            .translate_batch(&["一".to_string(), "二".to_string()])
            .await;
        long_server.await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].as_ref().unwrap().0, "Một");
        assert_eq!(results[1].as_ref().unwrap().0, "Hai");
    }

    #[tokio::test]
    async fn api_errors_never_leak_the_api_key() {
        let leaky_body =
            r#"{"error":{"code":401,"message":"API key not valid: test-secret-key-123456"}}"#;
        let (addr, server) = spawn_capture_server("HTTP/1.1 401 Unauthorized", leaky_body).await;
        let client = client_with_base(&format!("http://{addr}/v1beta"));

        let results = client.translate_batch(&["こんにちは".to_string()]).await;
        server.await.unwrap();

        let message = results[0].as_ref().unwrap_err().message.clone();
        assert!(
            !message.contains("test-secret-key-123456"),
            "key leaked: {message}"
        );
        assert!(
            message.contains("[REDACTED]"),
            "expected redaction marker: {message}"
        );
        assert_eq!(
            results[0].as_ref().unwrap_err().code,
            "local_gemini_api_error"
        );
    }

    #[test]
    fn parses_generate_content_text_table() {
        // Parts are joined in order.
        let joined = r#"{"candidates":[{"content":{"parts":[{"text":"A"},{"text":"B"}]}}]}"#;
        assert_eq!(
            parse_generate_content_text(StatusCode::OK, joined, "k").unwrap(),
            "AB"
        );

        // Missing or empty candidates surface the empty-response error.
        let empty = r#"{"candidates":[{"finishReason":"SAFETY"}]}"#;
        assert_eq!(
            parse_generate_content_text(StatusCode::OK, empty, "k")
                .unwrap_err()
                .code,
            "local_gemini_empty_response"
        );

        // Non-JSON success bodies surface the parse error.
        assert_eq!(
            parse_generate_content_text(StatusCode::OK, "not json", "k")
                .unwrap_err()
                .code,
            "local_gemini_parse_error"
        );

        // Error statuses carry Google's message.
        let error_body = r#"{"error":{"message":"Model not found"}}"#;
        let error =
            parse_generate_content_text(StatusCode::NOT_FOUND, error_body, "k").unwrap_err();
        assert_eq!(error.code, "local_gemini_api_error");
        assert!(error.message.contains("Model not found"));
    }

    #[test]
    fn parses_translation_array_variants() {
        // Typical: a clean array.
        assert_eq!(
            parse_translation_array(r#"["Một","Hai"]"#).unwrap(),
            vec!["Một".to_string(), "Hai".to_string()]
        );
        // Fenced output from a chatty model is repaired.
        assert_eq!(
            parse_translation_array("```json\n[\"Một\"]\n```").unwrap(),
            vec!["Một".to_string()]
        );
        assert_eq!(
            parse_translation_array("```\n[\"Một\"]\n```").unwrap(),
            vec!["Một".to_string()]
        );
        // Non-array payloads are rejected.
        assert_eq!(
            parse_translation_array(r#"{"translation":"Một"}"#)
                .unwrap_err()
                .code,
            "local_gemini_parse_error"
        );
        assert_eq!(
            parse_translation_array("Một").unwrap_err().code,
            "local_gemini_parse_error"
        );
        // Non-string elements break the 1:1 string contract.
        assert_eq!(
            parse_translation_array(r#"["Một", 2]"#).unwrap_err().code,
            "local_gemini_parse_error"
        );
    }

    #[test]
    fn normalize_gemini_model_table() {
        assert_eq!(
            normalize_gemini_model("gemini-2.5-flash"),
            "gemini-2.5-flash"
        );
        assert_eq!(
            normalize_gemini_model("models/gemini-2.5-flash"),
            "gemini-2.5-flash"
        );
        assert_eq!(normalize_gemini_model("   "), DEFAULT_MODEL);
        assert_eq!(normalize_gemini_model(""), DEFAULT_MODEL);
        assert_eq!(normalize_gemini_model("  models/  "), DEFAULT_MODEL);
        // Anything that could shape the request URL falls back to the default.
        assert_eq!(normalize_gemini_model("../secrets"), DEFAULT_MODEL);
        assert_eq!(normalize_gemini_model("gemini?key=x"), DEFAULT_MODEL);
        assert_eq!(normalize_gemini_model("gemini 2.5"), DEFAULT_MODEL);
    }

    #[test]
    fn key_gate_names_both_fixes() {
        assert!(crate::local_translation::ensure_gemini_key_available(Some("AIzaTest")).is_ok());
        let missing = crate::local_translation::ensure_gemini_key_available(None).unwrap_err();
        assert_eq!(missing.code, "local_gemini_api_key_missing");
        assert!(
            missing.message.contains("Cloud settings")
                && missing.message.contains("GEMINI_API_KEY"),
            "the gate must name both fixes, got: {}",
            missing.message
        );
        assert_eq!(
            crate::local_translation::ensure_gemini_key_available(Some("   "))
                .unwrap_err()
                .code,
            "local_gemini_api_key_missing"
        );
    }

    #[test]
    fn rejects_empty_credential_and_auto_target() {
        let no_key = GeminiClient::new("gemini", "  ".to_string(), Language::Ja, Language::Vi);
        assert_eq!(no_key.unwrap_err().code, "local_gemini_api_key_missing");

        let auto_target =
            GeminiClient::new("gemini", "k".to_string(), Language::Ja, Language::Auto);
        assert_eq!(auto_target.unwrap_err().code, "unsupported_target_language");
    }

    #[tokio::test]
    async fn engine_probe_maps_http_outcomes() {
        // Accepted key.
        let (ok_addr, ok_server) =
            spawn_capture_server("HTTP/1.1 200 OK", r#"{"models":[]}"#).await;
        assert!(
            probe_key_at(&format!("http://{ok_addr}/models"), "test-key")
                .await
                .is_ok()
        );
        ok_server.await.unwrap();

        // Rejected key: reachable but not accepted.
        let (rejected_addr, rejected_server) = spawn_capture_server(
            "HTTP/1.1 403 Forbidden",
            r#"{"error":{"message":"permission denied"}}"#,
        )
        .await;
        let rejected = probe_key_at(&format!("http://{rejected_addr}/models"), "test-key")
            .await
            .unwrap_err();
        rejected_server.await.unwrap();
        assert_eq!(rejected.code, "local_gemini_api_error");
        assert!(rejected.message.contains("permission denied"));

        // Unreachable endpoint: not reachable.
        let unreachable = probe_key_at("http://127.0.0.1:1/models", "test-key")
            .await
            .unwrap_err();
        assert_eq!(unreachable.code, "local_gemini_request_error");
    }

    #[test]
    fn empty_source_fails_the_window_without_a_request() {
        // No network client construction issues; translate_batch returns the
        // per-item error synchronously for empty sources.
        let client =
            GeminiClient::new("gemini", "k".to_string(), Language::Ja, Language::Vi).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let results =
            rt.block_on(client.translate_batch(&["こんにちは".to_string(), "  ".to_string()]));
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].as_ref().unwrap_err().code,
            "local_translation_empty_source"
        );
        assert_eq!(
            results[1].as_ref().unwrap_err().code,
            "local_translation_empty_source"
        );
    }

    #[tokio::test]
    async fn single_translate_rides_the_batch_contract() {
        let (addr, server) = spawn_capture_server(
            "HTTP/1.1 200 OK",
            r#"{"candidates":[{"content":{"parts":[{"text":"[\"Xin chào\"]"}]}}]}"#,
        )
        .await;
        let client = client_with_base(&format!("http://{addr}/v1beta"));
        let (translated, _latency) = client.translate("こんにちは").await.unwrap();
        server.await.unwrap();
        assert_eq!(translated, "Xin chào");
    }
}
