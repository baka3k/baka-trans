use crate::error::{AppError, AppResult};
use crate::models::{
    OverlayConfig, OverlayGeometry, OverlayStatus, OverlayStatusKind, OverlayTranslationUpdate,
    TranslationProvider,
};
use crate::security;
use serde_json::{json, Value};
use std::collections::{hash_map::DefaultHasher, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use objc2::{rc::Retained, runtime::AnyObject, AnyThread};
#[cfg(target_os = "macos")]
use objc2_app_kit::NSWindow;
#[cfg(target_os = "macos")]
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
#[cfg(target_os = "macos")]
use objc2_core_graphics::{CGImage, CGWindowID, CGWindowImageOption, CGWindowListOption};
#[cfg(target_os = "macos")]
use objc2_foundation::{NSArray, NSDictionary, NSError, NSString};
#[cfg(target_os = "macos")]
use objc2_vision::{
    VNImageOption, VNImageRequestHandler, VNRecognizeTextRequest, VNRequest,
    VNRequestTextRecognitionLevel,
};
#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};

const OVERLAY_LABEL: &str = "transparent-overlay";
const STATUS_EVENT: &str = "overlay-status-update";
const TRANSLATION_EVENT: &str = "overlay-translation-update";
const CACHE_LIMIT: usize = 24;
const SCREEN_RECORDING_PERMISSION_MESSAGE: &str = "Baka Trans needs Screen & System Audio Recording access to read text from browsers and other apps. Allow access for this exact Baka Trans app, then quit and reopen it.";

#[cfg(target_os = "macos")]
static SCREEN_RECORDING_PERMISSION_REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct OverlayState {
    inner: Mutex<OverlayInner>,
}

struct OverlayInner {
    config: OverlayConfig,
    geometry: Option<OverlayGeometry>,
    is_open: bool,
    is_paused: bool,
    status: OverlayStatusKind,
    message: String,
    window_id: Option<u32>,
    last_text_hash: Option<u64>,
    cache: VecDeque<(u64, String)>,
    runtime: Option<JoinHandle<()>>,
}

impl OverlayState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(OverlayInner {
                config: OverlayConfig::default(),
                geometry: None,
                is_open: false,
                is_paused: false,
                status: OverlayStatusKind::Idle,
                message: "Ready".to_string(),
                window_id: None,
                last_text_hash: None,
                cache: VecDeque::new(),
                runtime: None,
            }),
        }
    }

    pub fn open_overlay_window(&self, app: AppHandle, config: OverlayConfig) -> AppResult<()> {
        let config = config.normalized();
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            window
                .set_focus()
                .map_err(|err| AppError::new("overlay_focus_error", err.to_string()))?;
            let status = {
                let window_id = overlay_window_id(&window);
                let mut inner = self.inner.lock().map_err(lock_error)?;
                inner.config = config;
                inner.is_open = true;
                inner.window_id = window_id;
                inner.status_payload()
            };
            emit_status(&app, status)?;
            self.ensure_runtime(app);
            return Ok(());
        }

        let window = WebviewWindowBuilder::new(
            &app,
            OVERLAY_LABEL,
            WebviewUrl::App("index.html?overlay=transparent".into()),
        )
        .title("Baka Trans Overlay")
        .inner_size(480.0, 560.0)
        .min_inner_size(360.0, 420.0)
        .transparent(true)
        .decorations(false)
        .content_protected(true)
        .resizable(true)
        .always_on_top(true)
        .build()
        .map_err(|err| AppError::new("overlay_window_error", err.to_string()))?;

        let status = {
            let window_id = overlay_window_id(&window);
            let mut inner = self.inner.lock().map_err(lock_error)?;
            inner.config = config;
            inner.is_open = true;
            inner.is_paused = false;
            inner.window_id = window_id;
            inner.status = OverlayStatusKind::Scanning;
            inner.message = "Position the overlay over text.".to_string();
            inner.status_payload()
        };
        emit_status(&app, status)?;
        self.ensure_runtime(app);
        Ok(())
    }

    pub fn close_overlay_window(&self, app: AppHandle) -> AppResult<()> {
        if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
            window
                .close()
                .map_err(|err| AppError::new("overlay_close_error", err.to_string()))?;
        }
        let status = {
            let mut inner = self.inner.lock().map_err(lock_error)?;
            inner.is_open = false;
            inner.is_paused = false;
            inner.geometry = None;
            inner.window_id = None;
            inner.status = OverlayStatusKind::Idle;
            inner.message = "Overlay closed.".to_string();
            if let Some(handle) = inner.runtime.take() {
                handle.abort();
            }
            inner.status_payload()
        };
        emit_status(&app, status)
    }

    pub fn status(&self, app: &AppHandle) -> AppResult<OverlayStatus> {
        let window = app.get_webview_window(OVERLAY_LABEL);
        let mut inner = self.inner.lock().map_err(lock_error)?;
        inner.is_open = window.is_some();
        inner.window_id = window.as_ref().and_then(overlay_window_id);
        Ok(inner.status_payload())
    }

    pub fn update_geometry(&self, app: &AppHandle, geometry: OverlayGeometry) -> AppResult<()> {
        let status = {
            let mut inner = self.inner.lock().map_err(lock_error)?;
            inner.geometry = Some(geometry);
            if inner.status == OverlayStatusKind::Idle {
                inner.status = OverlayStatusKind::Scanning;
                inner.message = "Scanning".to_string();
            }
            inner.status_payload()
        };
        emit_status(app, status)
    }

    pub fn update_config(&self, app: &AppHandle, config: OverlayConfig) -> AppResult<()> {
        let config = config.normalized();
        let status = {
            let mut inner = self.inner.lock().map_err(lock_error)?;
            inner.config = config;
            inner.last_text_hash = None;
            if inner.status == OverlayStatusKind::Translated
                || inner.status == OverlayStatusKind::Error
            {
                inner.status = OverlayStatusKind::Scanning;
                inner.message = "Scanning".to_string();
            }
            inner.status_payload()
        };
        emit_status(app, status)
    }

    pub fn set_paused(&self, app: AppHandle, paused: bool) -> AppResult<()> {
        let status = {
            let mut inner = self.inner.lock().map_err(lock_error)?;
            inner.is_paused = paused;
            inner.status = if paused {
                OverlayStatusKind::Paused
            } else {
                OverlayStatusKind::Scanning
            };
            inner.message = if paused {
                "Overlay scanning paused.".to_string()
            } else {
                "Scanning".to_string()
            };
            inner.status_payload()
        };
        emit_status(&app, status)
    }

    fn ensure_runtime(&self, app: AppHandle) {
        let should_spawn = self
            .inner
            .lock()
            .map(|inner| inner.runtime.is_none())
            .unwrap_or(false);
        if !should_spawn {
            return;
        }

        let app_for_task = app.clone();
        let handle = tauri::async_runtime::spawn(async move {
            overlay_loop(app_for_task).await;
        });

        if let Ok(mut inner) = self.inner.lock() {
            if let Some(previous) = inner.runtime.replace(handle) {
                previous.abort();
            }
        }
    }
}

impl OverlayInner {
    fn status_payload(&self) -> OverlayStatus {
        OverlayStatus {
            is_open: self.is_open,
            is_paused: self.is_paused,
            status: self.status,
            message: self.message.clone(),
            config: self.config.clone(),
            geometry: self.geometry.clone(),
        }
    }

    fn cached_translation(&self, text_hash: u64) -> Option<String> {
        self.cache
            .iter()
            .find_map(|(hash, translated)| (*hash == text_hash).then(|| translated.clone()))
    }

    fn remember_translation(&mut self, text_hash: u64, translated: String) {
        if self.cache.iter().any(|(hash, _)| *hash == text_hash) {
            return;
        }
        self.cache.push_back((text_hash, translated));
        while self.cache.len() > CACHE_LIMIT {
            let _ = self.cache.pop_front();
        }
    }
}

async fn overlay_loop(app: AppHandle) {
    loop {
        let (is_open, is_paused, geometry, config, window_id) = {
            let state = app.state::<OverlayState>();
            let snapshot = match state.inner.lock() {
                Ok(inner) => (
                    inner.is_open,
                    inner.is_paused,
                    inner.geometry.clone(),
                    inner.config.clone(),
                    inner.window_id,
                ),
                Err(_) => return,
            };
            snapshot
        };

        if !is_open {
            break;
        }

        if is_paused {
            tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
            continue;
        }

        if geometry.is_none() {
            let _ = set_status(
                &app,
                OverlayStatusKind::Scanning,
                "Waiting for overlay geometry.",
            );
            tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
            continue;
        }

        match capture_and_ocr_text(&geometry.unwrap(), config.minimum_confidence, window_id).await {
            Ok(raw_text) => {
                let normalized = normalize_ocr_text(&raw_text);
                if normalized.is_empty() {
                    let _ = set_status(&app, OverlayStatusKind::NoText, "No readable text found.");
                } else {
                    if let Err(error) = handle_ocr_text(&app, &config, normalized).await {
                        let _ = set_status(&app, OverlayStatusKind::Error, error.message);
                    }
                }
            }
            Err(error) => {
                let status = if error.code == "screen_recording_permission_needed" {
                    OverlayStatusKind::PermissionNeeded
                } else {
                    OverlayStatusKind::Error
                };
                let _ = set_status(&app, status, error.message);
            }
        }

        tokio::time::sleep(Duration::from_millis(config.capture_interval_ms)).await;
    }
}

async fn handle_ocr_text(
    app: &AppHandle,
    config: &OverlayConfig,
    normalized: String,
) -> AppResult<()> {
    let text_hash = text_hash(&normalized);
    let maybe_cached = {
        let state = app.state::<OverlayState>();
        let mut inner = state.inner.lock().map_err(lock_error)?;
        if inner.last_text_hash == Some(text_hash) {
            return Ok(());
        }
        inner.status = OverlayStatusKind::Translating;
        inner.message = "Translating".to_string();
        let cached = inner.cached_translation(text_hash);
        emit_status(app, inner.status_payload())?;
        cached
    };

    let started = now_ms();
    let translated = if let Some(cached) = maybe_cached {
        cached
    } else {
        translate_with_gemini(config, &normalized).await?
    };
    let latency_ms = now_ms().saturating_sub(started);

    {
        let state = app.state::<OverlayState>();
        let mut inner = state.inner.lock().map_err(lock_error)?;
        inner.last_text_hash = Some(text_hash);
        inner.status = OverlayStatusKind::Translated;
        inner.message = "Translated".to_string();
        inner.remember_translation(text_hash, translated.clone());
        emit_status(app, inner.status_payload())?;
    }

    app.emit(
        TRANSLATION_EVENT,
        OverlayTranslationUpdate {
            source_text: normalized,
            translated_text: translated,
            status: OverlayStatusKind::Translated,
            message: "Translated".to_string(),
            confidence: None,
            latency_ms: Some(latency_ms),
            provider: "gemini_text".to_string(),
            model: config.gemini_model.clone(),
            updated_at_ms: now_ms(),
        },
    )
    .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

async fn translate_with_gemini(config: &OverlayConfig, text: &str) -> AppResult<String> {
    let api_key = security::load_translation_api_key(TranslationProvider::GoogleLiveTranslate)?;
    let model = config.gemini_model.trim().trim_start_matches("models/");
    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");
    let prompt = format!(
        "Translate the OCR text below into {}. Return only the translation. Preserve useful line breaks. Keep UI, code, and product terminology literal.\n\nOCR text:\n{}",
        config.target_language.realtime_code(),
        text
    );

    let response = reqwest::Client::new()
        .post(url)
        .query(&[("key", api_key.trim())])
        .json(&json!({
            "contents": [{ "parts": [{ "text": prompt }] }],
            "generationConfig": {
                "temperature": 0.1,
                "maxOutputTokens": 1200
            }
        }))
        .send()
        .await
        .map_err(|err| AppError::new("gemini_text_request_error", err.to_string()))?;

    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| AppError::new("gemini_text_response_error", err.to_string()))?;
    if !status.is_success() {
        return Err(AppError::new(
            "gemini_text_api_error",
            extract_google_error_message(&body).unwrap_or(body),
        ));
    }

    parse_gemini_text(&body)
}

fn parse_gemini_text(body: &str) -> AppResult<String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|err| AppError::new("gemini_text_parse_error", err.to_string()))?;
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
            "gemini_text_empty_response",
            "Gemini returned an empty translation.",
        ));
    }
    Ok(cleaned.to_string())
}

pub(crate) async fn capture_and_ocr_text(
    geometry: &OverlayGeometry,
    minimum_confidence: f32,
    window_id: Option<u32>,
) -> AppResult<String> {
    #[cfg(target_os = "macos")]
    {
        let permission_granted =
            screen_recording_permission_granted() || request_screen_recording_permission_once();
        require_screen_recording_permission(permission_granted)?;
        let geometry = geometry.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            capture_and_ocr_text_blocking(&geometry, minimum_confidence, window_id)
        })
        .await
        .map_err(|err| AppError::new("overlay_ocr_join_error", err.to_string()))?;
    }

    #[cfg(not(target_os = "macos"))]
    Err(AppError::new(
        "unsupported_platform",
        "Transparent OCR overlay capture is currently only available on macOS.",
    ))
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn capture_and_ocr_text_blocking(
    geometry: &OverlayGeometry,
    minimum_confidence: f32,
    window_id: Option<u32>,
) -> AppResult<String> {
    let rect = CGRect::new(
        CGPoint::new(geometry.x, geometry.y),
        CGSize::new(geometry.width.max(1.0), geometry.height.max(1.0)),
    );
    let image = objc2_core_graphics::CGWindowListCreateImage(
        rect,
        window_id
            .map(|_| CGWindowListOption::OptionOnScreenBelowWindow)
            .unwrap_or(CGWindowListOption::OptionOnScreenOnly),
        window_id.unwrap_or_default() as CGWindowID,
        CGWindowImageOption::BestResolution,
    )
    .ok_or_else(|| {
        AppError::new(
            "overlay_capture_empty",
            "macOS did not return an image for the selected overlay region.",
        )
    })?;

    recognize_text_in_image(&image, minimum_confidence)
}

#[cfg(target_os = "macos")]
fn recognize_text_in_image(image: &CGImage, minimum_confidence: f32) -> AppResult<String> {
    let request = VNRecognizeTextRequest::new();
    request.setRecognitionLevel(VNRequestTextRecognitionLevel::Fast);
    request.setUsesLanguageCorrection(false);
    request.setAutomaticallyDetectsLanguage(true);

    let options = NSDictionary::<VNImageOption, AnyObject>::from_slices::<NSString>(&[], &[]);
    let handler = unsafe {
        VNImageRequestHandler::initWithCGImage_options(
            VNImageRequestHandler::alloc(),
            image,
            &options,
        )
    };
    let request_for_handler: Retained<VNRequest> = request.clone().into_super().into_super();
    let requests = NSArray::from_retained_slice(&[request_for_handler]);
    handler
        .performRequests_error(&requests)
        .map_err(|err| AppError::new("vision_ocr_error", ns_error_message(&err)))?;

    let recognized = request
        .results()
        .unwrap_or_else(|| NSArray::from_retained_slice(&[]));
    let mut lines = Vec::new();
    for observation in recognized.iter() {
        let candidates = observation.topCandidates(1);
        if let Some(candidate) = candidates.iter().next() {
            if candidate.confidence() < minimum_confidence {
                continue;
            }
            let text = candidate.string().to_string();
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
        }
    }

    Ok(lines.join("\n"))
}

#[cfg(target_os = "macos")]
fn ns_error_message(error: &NSError) -> String {
    error.localizedDescription().to_string()
}

#[cfg(target_os = "macos")]
fn overlay_window_id(window: &WebviewWindow) -> Option<u32> {
    let ns_window = window.ns_window().ok()?;
    if ns_window.is_null() {
        return None;
    }
    let ns_window = unsafe { &*ns_window.cast::<NSWindow>() };
    Some(ns_window.windowNumber() as u32)
}

#[cfg(not(target_os = "macos"))]
fn overlay_window_id(_window: &WebviewWindow) -> Option<u32> {
    None
}

pub fn open_screen_recording_settings() -> AppResult<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn()
            .map_err(|err| AppError::new("open_settings_error", err.to_string()))?;
        return Ok(());
    }

    #[cfg(not(target_os = "macos"))]
    Err(AppError::new(
        "unsupported_platform",
        "Screen Recording settings are only available on macOS.",
    ))
}

pub fn normalize_ocr_text(text: &str) -> String {
    text.lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn set_status(
    app: &AppHandle,
    status: OverlayStatusKind,
    message: impl Into<String>,
) -> AppResult<()> {
    let payload = {
        let state = app.state::<OverlayState>();
        let mut inner = state.inner.lock().map_err(lock_error)?;
        inner.status = status;
        inner.message = message.into();
        inner.status_payload()
    };
    emit_status(app, payload)
}

fn emit_status(app: &AppHandle, payload: OverlayStatus) -> AppResult<()> {
    app.emit(STATUS_EVENT, payload)
        .map_err(|err| AppError::new("event_emit_error", err.to_string()))
}

fn extract_google_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn lock_error<T>(_: std::sync::PoisonError<T>) -> AppError {
    AppError::new("state_lock_error", "Overlay state lock was poisoned.")
}

fn require_screen_recording_permission(permission_granted: bool) -> AppResult<()> {
    if permission_granted {
        Ok(())
    } else {
        Err(AppError::new(
            "screen_recording_permission_needed",
            SCREEN_RECORDING_PERMISSION_MESSAGE,
        ))
    }
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
fn screen_recording_permission_granted() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

#[cfg(not(target_os = "macos"))]
fn screen_recording_permission_granted() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn request_screen_recording_permission_once() -> bool {
    if SCREEN_RECORDING_PERMISSION_REQUESTED.swap(true, Ordering::AcqRel) {
        return false;
    }

    unsafe { CGRequestScreenCaptureAccess() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Language;

    #[test]
    fn normalizes_ocr_text() {
        assert_eq!(
            normalize_ocr_text("  Hello   world \n\n second\tline "),
            "Hello world\nsecond line"
        );
    }

    #[test]
    fn text_hash_changes_with_text() {
        assert_ne!(text_hash("hello"), text_hash("hello!"));
    }

    #[test]
    fn screen_recording_permission_gate_rejects_privacy_limited_capture() {
        let error = require_screen_recording_permission(false).unwrap_err();

        assert_eq!(error.code, "screen_recording_permission_needed");
        assert!(require_screen_recording_permission(true).is_ok());
    }

    #[test]
    fn parses_gemini_text_response() {
        let body = r#"{
          "candidates": [
            { "content": { "parts": [{ "text": "Xin chao" }] } }
          ]
        }"#;

        assert_eq!(parse_gemini_text(body).unwrap(), "Xin chao");
    }

    #[test]
    fn overlay_config_clamps_runtime_values() {
        let config = OverlayConfig {
            capture_interval_ms: 10,
            minimum_confidence: 2.0,
            opacity: 0.1,
            gemini_model: String::new(),
            source_language: Language::Auto,
            target_language: Language::Vi,
        }
        .normalized();

        assert_eq!(config.capture_interval_ms, 500);
        assert_eq!(config.minimum_confidence, 1.0);
        assert_eq!(config.opacity, 0.35);
        assert_eq!(config.gemini_model, "models/gemini-2.5-flash");
    }
}
