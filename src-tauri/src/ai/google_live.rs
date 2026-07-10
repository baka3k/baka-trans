use crate::error::{AppError, AppResult};
use serde_json::Value;

pub async fn test_live_translation_connection(api_key: &str) -> AppResult<()> {
    let response = reqwest::Client::new()
        .get("https://generativelanguage.googleapis.com/v1beta/models")
        .query(&[("key", api_key.trim())])
        .send()
        .await
        .map_err(|err| AppError::new("google_live_key_test_error", err.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::new("google_live_key_test_error", err.to_string()))?;

    if !status.is_success() {
        let message = extract_google_error_message(&body).unwrap_or(body);
        return Err(AppError::new("google_live_key_test_error", message));
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::extract_google_error_message;

    #[test]
    fn extracts_google_error_message() {
        let body = r#"{"error":{"code":400,"message":"API key not valid."}}"#;

        assert_eq!(
            extract_google_error_message(body),
            Some("API key not valid.".to_string())
        );
    }
}
