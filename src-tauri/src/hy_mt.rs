use crate::error::{AppError, AppResult};
use crate::models::{HyMtModelPhase, HyMtModelProgress, HyMtModelStatus, OfflineTranslationModel};
use crate::vieneu::model_cache_dir;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const HY_MT_MODEL_ID: &str = "tencent/Hy-MT2-1.8B";
pub const HY_MT_MODEL_REVISION: &str = "9a341cd1b679d3efd23b46e847b01745a71ed792";
pub const HY_MT_TOTAL_BYTES: u64 = 4_086_796_766;

/// Static registry entry for one managed offline translation model. The
/// `key` matches the sidecar CLI `--model` choices and the per-model cache
/// directory name (Hy-MT2 keeps the legacy unscoped cache layout).
pub struct OfflineModelSpec {
    pub model: OfflineTranslationModel,
    pub key: &'static str,
    pub display_name: &'static str,
    pub model_id: &'static str,
    pub revision: &'static str,
    pub total_bytes: u64,
    /// Gated repositories refuse anonymous downloads; installing them needs a
    /// Hugging Face token from an account that accepted the upstream license.
    pub gated: bool,
}

pub const HY_MT2_SPEC: OfflineModelSpec = OfflineModelSpec {
    model: OfflineTranslationModel::HyMt2,
    key: "hy-mt2",
    display_name: "Hy-MT2 1.8B",
    model_id: HY_MT_MODEL_ID,
    revision: HY_MT_MODEL_REVISION,
    total_bytes: HY_MT_TOTAL_BYTES,
    gated: false,
};

// Pinned to the public commit of google/translategemma-4b-it. The repository
// is manually gated on Hugging Face: installs require the user's HF token and
// the LFS sha256 digests are not public, so the sidecar verifies the small
// files against git blob ids and the weights against exact sizes.
pub const TRANSLATEGEMMA_4B_SPEC: OfflineModelSpec = OfflineModelSpec {
    model: OfflineTranslationModel::TranslateGemma4B,
    key: "translategemma-4b",
    display_name: "TranslateGemma 4B",
    model_id: "google/translategemma-4b-it",
    revision: "10042cb0e6e7fdce748996a71dc3dc432a4e0c89",
    total_bytes: 8_639_637_704,
    gated: true,
};

pub fn offline_model_spec(model: OfflineTranslationModel) -> &'static OfflineModelSpec {
    match model {
        OfflineTranslationModel::HyMt2 => &HY_MT2_SPEC,
        OfflineTranslationModel::TranslateGemma4B => &TRANSLATEGEMMA_4B_SPEC,
    }
}

const HY_MT_PROTOCOL_VERSION: u32 = 1;
const HY_MT_CACHE_SUBDIR: &str = "hy-mt";
const ACTIVE_DIR_NAME: &str = "active";
const STAGING_DIR_NAME: &str = ".staging";
const MANIFEST_NAME: &str = "install-manifest.json";
const EVENT_NAME: &str = "hy-mt-model-progress";
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
const BYTE_PROGRESS_INTERVAL: Duration = Duration::from_millis(500);
const SERVE_READY_TIMEOUT: Duration = Duration::from_secs(120);
const SERVE_TRANSLATE_TIMEOUT: Duration = Duration::from_secs(60);
const SERVE_PROBE_TEXT: &str = "こんにちは";
const SERVE_PROBE_MAX_NEW_TOKENS: u32 = 64;

#[derive(Clone)]
pub(crate) struct CommandSpec {
    pub(crate) program: PathBuf,
    pub(crate) prefix_args: Vec<std::ffi::OsString>,
    pub(crate) working_dir: PathBuf,
}

pub struct HyMtManager {
    install_active: AtomicBool,
    install_cancelled: Arc<AtomicBool>,
    last_error: Mutex<Option<String>>,
}

impl HyMtManager {
    pub fn new() -> Self {
        Self {
            install_active: AtomicBool::new(false),
            install_cancelled: Arc::new(AtomicBool::new(false)),
            last_error: Mutex::new(None),
        }
    }

    pub async fn status(&self, app: &AppHandle, model: OfflineTranslationModel) -> AppResult<HyMtModelStatus> {
        let spec = offline_model_spec(model);
        let paths = ManagedPaths::resolve_for(spec)?;
        let runtime_available = resolve_command(app).is_ok();
        let installing = self.install_active.load(Ordering::Acquire);
        let last_error = self.last_error.lock().map_err(lock_error)?.clone();
        let manifest_installed = paths.manifest_path().is_file();
        let partial_bytes = directory_size(&paths.staging_dir).unwrap_or(0);

        let (phase, message) = if !runtime_available {
            (
                HyMtModelPhase::Unsupported,
                format!("This build does not include the managed {} runtime.", spec.display_name),
            )
        } else if installing {
            (
                HyMtModelPhase::Downloading,
                format!("Downloading the {} model…", spec.display_name),
            )
        } else if let Some(error) = last_error {
            (HyMtModelPhase::Error, error)
        } else if manifest_installed {
            (
                HyMtModelPhase::Installed,
                format!("{} model is installed and verified.", spec.display_name),
            )
        } else if partial_bytes > 0 {
            (
                HyMtModelPhase::Paused,
                format!("{} setup can be resumed.", spec.display_name),
            )
        } else {
            (
                HyMtModelPhase::NotInstalled,
                format!(
                    "Install {} to download the pinned offline translation model.",
                    spec.display_name
                ),
            )
        };

        Ok(HyMtModelStatus {
            model,
            phase,
            runtime_available,
            model_installed: manifest_installed,
            model_id: spec.model_id.to_string(),
            model_revision: spec.revision.to_string(),
            total_bytes: spec.total_bytes,
            message,
        })
    }

    pub async fn install(&self, app: AppHandle, model: OfflineTranslationModel) -> AppResult<HyMtModelStatus> {
        let spec = offline_model_spec(model);
        let guard = InstallGuard::acquire(self)?;
        self.install_cancelled.store(false, Ordering::Release);
        let repair_requested = self.last_error.lock().map_err(lock_error)?.is_some();
        *self.last_error.lock().map_err(lock_error)? = None;

        let paths = ManagedPaths::resolve_for(spec)?;
        if paths.manifest_path().is_file() && !repair_requested {
            drop(guard);
            return self.status(&app, model).await;
        }
        let command_spec = resolve_command(&app)?;
        let cancelled = self.install_cancelled.clone();
        let staging_dir = paths.staging_dir.clone();
        let model_root = paths.model_root.clone();
        let progress_app = app.clone();
        let exit_status = match tauri::async_runtime::spawn_blocking(move || {
            run_install_process(command_spec, spec, model_root, staging_dir, progress_app, cancelled)
        })
        .await
        {
            Ok(result) => result,
            Err(error) => {
                let error = AppError::new("hy_mt_install_join_error", error.to_string());
                self.remember_error(&error);
                return Err(error);
            }
        };
        let exit_status = match exit_status {
            Ok(status) => status,
            Err(error) => {
                self.remember_error(&error);
                return Err(error);
            }
        };

        if exit_status == InstallExit::Paused {
            drop(guard);
            return self.status(&app, model).await;
        }
        if !paths.manifest_path().is_file() {
            let error = AppError::new(
                "hy_mt_model_activation_error",
                format!(
                    "{} finished setup without an active verified model. Retry the install.",
                    spec.display_name
                ),
            );
            self.remember_error(&error);
            return Err(error);
        }
        emit_progress(
            &app,
            spec,
            HyMtModelPhase::Installed,
            spec.total_bytes,
            spec.total_bytes,
            Some(100),
            &format!("{} model is installed and verified.", spec.display_name),
        );
        drop(guard);
        self.status(&app, model).await
    }

    pub fn cancel_install(&self) {
        self.install_cancelled.store(true, Ordering::Release);
    }

    fn remember_error(&self, error: &AppError) {
        if let Ok(mut slot) = self.last_error.lock() {
            *slot = Some(error.message.clone());
        }
    }
}

impl Default for HyMtManager {
    fn default() -> Self {
        Self::new()
    }
}

struct ManagedPaths {
    model_root: PathBuf,
    staging_dir: PathBuf,
}

impl ManagedPaths {
    /// Hy-MT2 keeps the legacy unscoped cache directory so existing verified
    /// installs stay valid; every later model nests under its registry key.
    fn resolve_for(spec: &OfflineModelSpec) -> AppResult<Self> {
        let hy_mt_root = model_cache_dir()?.join(HY_MT_CACHE_SUBDIR);
        let model_root = if spec.key == HY_MT2_SPEC.key {
            hy_mt_root
        } else {
            hy_mt_root.join(spec.key)
        };
        Ok(Self {
            staging_dir: model_root.join(STAGING_DIR_NAME).join(spec.revision),
            model_root,
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.model_root.join(ACTIVE_DIR_NAME).join(MANIFEST_NAME)
    }
}

struct InstallGuard<'a> {
    manager: &'a HyMtManager,
}

impl<'a> InstallGuard<'a> {
    fn acquire(manager: &'a HyMtManager) -> AppResult<Self> {
        manager
            .install_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| AppError::new("hy_mt_install_busy", "Hy-MT2 setup is already running."))?;
        Ok(Self { manager })
    }
}

impl Drop for InstallGuard<'_> {
    fn drop(&mut self) {
        self.manager.install_active.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallExit {
    Completed,
    Paused,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeFrame {
    #[serde(rename = "type")]
    kind: String,
    state: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    message: Option<String>,
}

fn map_state_to_phase(state: Option<&str>) -> Option<HyMtModelPhase> {
    match state {
        Some("downloading") => Some(HyMtModelPhase::Downloading),
        Some("verifying") => Some(HyMtModelPhase::Verifying),
        Some("installed") => Some(HyMtModelPhase::Installed),
        Some("not_installed") => Some(HyMtModelPhase::NotInstalled),
        _ => None,
    }
}

fn run_install_process(
    command_spec: CommandSpec,
    offline: &'static OfflineModelSpec,
    model_root: PathBuf,
    staging_dir: PathBuf,
    app: AppHandle,
    cancelled: Arc<AtomicBool>,
) -> AppResult<InstallExit> {
    let mut command = command_from_spec(&command_spec);
    command
        .arg("install")
        .arg("--model")
        .arg(offline.key)
        .arg("--model-root")
        .arg(&model_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // Gated repositories (TranslateGemma) refuse anonymous downloads. The
    // token is passed only to this one-shot install child; serve mode rejects
    // hub credentials inside the sidecar itself.
    if offline.gated {
        if let Ok(Some(token)) = crate::local_translation::api_key::load_huggingface_token() {
            command.env("HF_TOKEN", token);
        }
    }
    hide_child_window(&mut command);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            "hy_mt_runtime_missing",
            format!(
                "Could not start the bundled {} installer: {error}",
                offline.display_name
            ),
        )
    })?;
    let receiver = spawn_line_reader(child.stdout.take().ok_or_else(|| {
        AppError::new(
            "hy_mt_install_output_error",
            "Hy-MT2 installer has no output pipe.",
        )
    })?);

    let mut phase = HyMtModelPhase::Downloading;
    let mut downloaded_bytes = 0_u64;
    let mut last_byte_poll = Instant::now() - BYTE_PROGRESS_INTERVAL;
    emit_progress(
        &app,
        offline,
        phase,
        0,
        offline.total_bytes,
        Some(0),
        &format!("Preparing the verified {} download…", offline.display_name),
    );

    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            emit_progress(
                &app,
                offline,
                HyMtModelPhase::Paused,
                directory_size(&staging_dir)
                    .unwrap_or(0)
                    .min(offline.total_bytes),
                offline.total_bytes,
                None,
                &format!("{} setup was paused. Resume when ready.", offline.display_name),
            );
            return Ok(InstallExit::Paused);
        }
        let mut frame_error: Option<String> = None;
        while let Ok(line) = receiver.try_recv() {
            if let Ok(frame) = serde_json::from_str::<BridgeFrame>(&line) {
                match frame.kind.as_str() {
                    "progress" | "status" | "complete" => {
                        if let Some(mapped) = map_state_to_phase(frame.state.as_deref()) {
                            phase = mapped;
                        }
                        downloaded_bytes = frame.downloaded_bytes.unwrap_or(downloaded_bytes);
                        let total_bytes = frame.total_bytes.unwrap_or(offline.total_bytes);
                        if frame.kind == "complete" {
                            phase = HyMtModelPhase::Installed;
                            downloaded_bytes = total_bytes.max(downloaded_bytes);
                        }
                        emit_progress(
                            &app,
                            offline,
                            phase,
                            downloaded_bytes.min(total_bytes.max(offline.total_bytes)),
                            offline.total_bytes,
                            progress_percent(downloaded_bytes, total_bytes),
                            &default_progress_message(offline, phase),
                        );
                    }
                    "error" => {
                        frame_error = Some(frame.message.unwrap_or_else(|| {
                            format!(
                                "{} setup failed. Resume to retry the verified download.",
                                offline.display_name
                            )
                        }));
                    }
                    _ => {}
                }
            }
        }
        if let Some(message) = frame_error {
            return Err(AppError::new("hy_mt_install_failed", message));
        }
        if phase == HyMtModelPhase::Downloading
            && last_byte_poll.elapsed() >= BYTE_PROGRESS_INTERVAL
        {
            last_byte_poll = Instant::now();
            let staged = directory_size(&staging_dir)
                .unwrap_or(0)
                .min(offline.total_bytes);
            if staged > downloaded_bytes {
                downloaded_bytes = staged;
                emit_progress(
                    &app,
                    offline,
                    phase,
                    downloaded_bytes,
                    offline.total_bytes,
                    progress_percent(downloaded_bytes, offline.total_bytes),
                    &default_progress_message(offline, phase),
                );
            }
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| AppError::new("hy_mt_install_wait_error", error.to_string()))?
        {
            if status.success() {
                return Ok(InstallExit::Completed);
            }
            return Err(AppError::new(
                "hy_mt_install_failed",
                format!(
                    "{} setup failed. Resume to retry the verified download.",
                    offline.display_name
                ),
            ));
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn default_progress_message(spec: &OfflineModelSpec, phase: HyMtModelPhase) -> String {
    match phase {
        HyMtModelPhase::Downloading => format!("Downloading the verified {} model…", spec.display_name),
        HyMtModelPhase::Verifying => format!("Verifying {} model files…", spec.display_name),
        HyMtModelPhase::Installed => format!("{} model is installed and verified.", spec.display_name),
        _ => format!("{} setup is running…", spec.display_name),
    }
}

fn progress_percent(downloaded_bytes: u64, total_bytes: u64) -> Option<u8> {
    if total_bytes == 0 {
        return None;
    }
    Some(((downloaded_bytes.saturating_mul(100) / total_bytes).min(100)) as u8)
}

fn emit_progress(
    app: &AppHandle,
    spec: &OfflineModelSpec,
    phase: HyMtModelPhase,
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: Option<u8>,
    message: &str,
) {
    let _ = app.emit(
        EVENT_NAME,
        HyMtModelProgress {
            model: spec.model,
            phase,
            downloaded_bytes,
            total_bytes,
            percent,
            message: message.to_string(),
        },
    );
}

fn resolve_command(app: &AppHandle) -> AppResult<CommandSpec> {
    if let Some(override_path) = std::env::var_os("BAKA_TRANS_HY_MT_SIDECAR") {
        let program = PathBuf::from(override_path);
        if program.is_file() {
            return Ok(CommandSpec {
                working_dir: program
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                program,
                prefix_args: Vec::new(),
            });
        }
    }

    if let Ok(resources) = app.path().resource_dir() {
        let name = if cfg!(target_os = "windows") {
            "hy-mt-sidecar.exe"
        } else {
            "hy-mt-sidecar"
        };
        let program = resources.join("hy-mt").join(name);
        if program.is_file() {
            return Ok(CommandSpec {
                working_dir: program
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                program,
                prefix_args: Vec::new(),
            });
        }
    }

    #[cfg(debug_assertions)]
    {
        let sidecar_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("sidecars")
            .join("hy-mt");
        let packaged = sidecar_dir
            .join("dist")
            .join("hy-mt-sidecar")
            .join("hy-mt-sidecar");
        if packaged.is_file() {
            return Ok(CommandSpec {
                working_dir: packaged
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf(),
                program: packaged,
                prefix_args: Vec::new(),
            });
        }
        let python = if cfg!(target_os = "windows") {
            sidecar_dir.join(".venv").join("Scripts").join("python.exe")
        } else {
            sidecar_dir.join(".venv").join("bin").join("python")
        };
        let script = sidecar_dir.join("server.py");
        if python.is_file() && script.is_file() {
            return Ok(CommandSpec {
                program: python,
                prefix_args: vec![script.into_os_string()],
                working_dir: sidecar_dir,
            });
        }
    }

    Err(AppError::new(
        "hy_mt_runtime_missing",
        "This build does not include the managed Hy-MT2 runtime.",
    ))
}

fn command_from_spec(spec: &CommandSpec) -> Command {
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.prefix_args)
        .current_dir(&spec.working_dir);
    command
}

fn spawn_line_reader(stdout: impl std::io::Read + Send + 'static) -> std_mpsc::Receiver<String> {
    let (sender, receiver) = std_mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if sender.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    receiver
}

#[derive(Debug)]
pub struct HyMtEngineProbe {
    pub device: String,
    pub load_ms: f64,
    pub translated_text: Option<String>,
    pub latency_ms: Option<f64>,
    pub reachable: bool,
    pub accepted: bool,
    pub error: Option<AppError>,
}

impl HyMtEngineProbe {
    fn failed(error: AppError) -> Self {
        Self {
            device: String::new(),
            load_ms: 0.0,
            translated_text: None,
            latency_ms: None,
            reachable: false,
            accepted: false,
            error: Some(error),
        }
    }
}

/// Runs one probe translation through the managed offline runtime.
///
/// The engine is started in `serve` mode, answers a single probe request,
/// and is shut down again. Live-session routing stays gated in
/// `local_translation::TranslationClient`.
pub async fn probe_translation_engine(
    app: &AppHandle,
    model: OfflineTranslationModel,
) -> HyMtEngineProbe {
    let app = app.clone();
    match tauri::async_runtime::spawn_blocking(move || probe_offline_engine(&app, model)).await {
        Ok(probe) => probe,
        Err(error) => HyMtEngineProbe::failed(AppError::new(
            "hy_mt_probe_join_error",
            format!("The offline engine test could not run: {error}"),
        )),
    }
}

fn probe_offline_engine(app: &AppHandle, model: OfflineTranslationModel) -> HyMtEngineProbe {
    let spec = offline_model_spec(model);
    let paths = match ManagedPaths::resolve_for(spec) {
        Ok(paths) => paths,
        Err(error) => return HyMtEngineProbe::failed(error),
    };
    if !paths.manifest_path().is_file() {
        return HyMtEngineProbe::failed(AppError::new(
            "hy_mt_model_missing",
            format!(
                "The {} model is not installed yet. Install it from the model card before testing the engine.",
                spec.display_name
            ),
        ));
    }
    let command_spec = match resolve_command(app) {
        Ok(command_spec) => command_spec,
        Err(error) => return HyMtEngineProbe::failed(error),
    };
    let device = preferred_serve_device();
    match run_serve_probe(&command_spec, &paths.model_root, device, spec) {
        Ok(probe) => probe,
        Err(error) => {
            // An Intel Mac or a machine without a usable MPS build exits before
            // the ready frame; retry once on CPU instead of failing the test.
            if device == "mps" && error.code == "hy_mt_serve_early_exit" {
                run_serve_probe(&command_spec, &paths.model_root, "cpu", spec)
                    .unwrap_or_else(HyMtEngineProbe::failed)
            } else {
                HyMtEngineProbe::failed(error)
            }
        }
    }
}

fn preferred_serve_device() -> &'static str {
    if cfg!(target_os = "macos") {
        "mps"
    } else {
        "cpu"
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslateRequest<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    protocol_version: u32,
    id: &'a str,
    source_language: &'a str,
    target_language: &'a str,
    text: &'a str,
    max_new_tokens: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ServeFrame {
    #[serde(rename = "type")]
    kind: String,
    protocol_version: Option<u32>,
    model_id: Option<String>,
    revision: Option<String>,
    device: Option<String>,
    load_ms: Option<f64>,
    id: Option<String>,
    text: Option<String>,
    latency_ms: Option<f64>,
    message: Option<String>,
}

fn run_serve_probe(
    command_spec: &CommandSpec,
    model_root: &Path,
    device: &str,
    offline: &'static OfflineModelSpec,
) -> AppResult<HyMtEngineProbe> {
    let mut session = HyMtSession::start_with_spec(command_spec, model_root, device, offline)?;
    let device = session.device.clone();
    let load_ms = session.load_ms;
    let (translated_text, _wall_clock_ms) = session.translate("ja", "vi", SERVE_PROBE_TEXT)?;
    Ok(HyMtEngineProbe {
        device,
        load_ms,
        translated_text: Some(translated_text),
        latency_ms: session.last_protocol_latency_ms(),
        reachable: true,
        accepted: true,
        error: None,
    })
}

fn parse_serve_frame(line: &str) -> AppResult<ServeFrame> {
    serde_json::from_str(line).map_err(|_| {
        AppError::new(
            "hy_mt_serve_protocol_error",
            "The Hy-MT2 runtime sent a malformed message.",
        )
    })
}

/// Reads one framed line from the runtime, with timeout. The caller owns the
/// running [`Child`] and reaps it via its own drop path. Used by both the
/// probe path and the long-lived [`HyMtSession`].
fn read_serve_line_without_guard(
    receiver: &std::sync::mpsc::Receiver<String>,
    timeout: Duration,
    timeout_code: &str,
    timeout_message: &str,
) -> AppResult<String> {
    let deadline = Instant::now() + timeout;
    let remaining = deadline.saturating_duration_since(Instant::now());
    match receiver.recv_timeout(remaining) {
        Ok(line) => Ok(line),
        Err(std_mpsc::RecvTimeoutError::Timeout) => {
            Err(AppError::new(timeout_code, timeout_message))
        }
        Err(std_mpsc::RecvTimeoutError::Disconnected) => Err(AppError::new(
            "hy_mt_serve_early_exit",
            "The Hy-MT2 runtime exited before answering. Retry the live session.",
        )),
    }
}

/// Long-lived Hy-MT2 runtime handle for live translation sessions.
///
/// The runtime is started in `serve` mode and reused across many translate
/// requests so the ~4 GB model is loaded only once. Drop kills the child
/// after a short grace window.
pub struct HyMtSession {
    child: Child,
    stdin: ChildStdin,
    receiver: std_mpsc::Receiver<String>,
    device: String,
    load_ms: f64,
    last_request_latency_ms: Option<f64>,
}

impl Drop for HyMtSession {
    fn drop(&mut self) {
        // Field drop order in Rust is reverse-of-declaration, so `stdin`
        // (declared after `child`) is dropped first, sending EOF to the
        // runtime and giving it a chance to flush and exit cleanly. Then
        // we give the runtime a brief grace window before force-killing.
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
                _ => break,
            }
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl HyMtSession {
    /// Resolve a fully-managed session using the bundled sidecar. The caller
    /// owns the returned handle until drop.
    pub fn start(app: &AppHandle, model: OfflineTranslationModel) -> AppResult<Self> {
        let offline = offline_model_spec(model);
        let paths = ManagedPaths::resolve_for(offline)?;
        if !paths.manifest_path().is_file() {
            return Err(AppError::new(
                "hy_mt_model_missing",
                format!(
                    "The {} model is not installed yet. Install it from the model card before starting the engine.",
                    offline.display_name
                ),
            ));
        }
        let command_spec = resolve_command(app)?;
        let device = preferred_serve_device();
        match Self::start_with_spec(&command_spec, &paths.model_root, device, offline) {
            Ok(session) => Ok(session),
            // Intel Macs and machines without a usable MPS build exit before
            // the ready frame; retry once on CPU instead of failing outright.
            Err(error) if device == "mps" && error.code == "hy_mt_serve_early_exit" => {
                Self::start_with_spec(&command_spec, &paths.model_root, "cpu", offline)
            }
            Err(error) => Err(error),
        }
    }

    /// Internal helper used directly by [`run_serve_probe`] with a synthetic
    /// `CommandSpec` and by [`start`](Self::start) for the managed runtime.
    pub(crate) fn start_with_spec(
        command_spec: &CommandSpec,
        model_root: &Path,
        device: &str,
        offline: &'static OfflineModelSpec,
    ) -> AppResult<Self> {
        let mut command = command_from_spec(command_spec);
        command
            .arg("serve")
            .arg("--model")
            .arg(offline.key)
            .arg("--model-root")
            .arg(model_root)
            .arg("--device")
            .arg(device)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        hide_child_window(&mut command);
        let mut child = command.spawn().map_err(|error| {
            AppError::new(
                "hy_mt_serve_spawn_error",
                format!("Could not start the Hy-MT2 runtime: {error}"),
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            AppError::new("hy_mt_serve_io_error", "Hy-MT2 runtime has no output pipe.")
        })?;
        let stdin = child.stdin.take().ok_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            AppError::new("hy_mt_serve_io_error", "Hy-MT2 runtime has no input pipe.")
        })?;
        let receiver = spawn_line_reader(stdout);

        let ready_line = read_serve_line_without_guard(
            &receiver,
            SERVE_READY_TIMEOUT,
            "hy_mt_serve_ready_timeout",
            "The offline Hy-MT2 engine did not finish loading within two minutes.",
        );
        let ready_line = match ready_line {
            Ok(line) => line,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let ready = match parse_serve_frame(&ready_line) {
            Ok(frame) => frame,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        if ready.kind == "error" {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "hy_mt_serve_failed",
                ready
                    .message
                    .unwrap_or_else(|| "The Hy-MT2 runtime failed to start.".to_string()),
            ));
        }
        if ready.kind != "ready" {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "hy_mt_serve_protocol_error",
                "The Hy-MT2 runtime sent an unexpected startup message.",
            ));
        }
        if ready.protocol_version != Some(HY_MT_PROTOCOL_VERSION)
            || ready.model_id.as_deref() != Some(offline.model_id)
            || ready.revision.as_deref() != Some(offline.revision)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "hy_mt_identity_mismatch",
                "The offline runtime reported an unexpected model identity.",
            ));
        }

        Ok(Self {
            child,
            stdin,
            receiver,
            device: ready.device.unwrap_or_else(|| device.to_string()),
            load_ms: ready.load_ms.unwrap_or(0.0),
            last_request_latency_ms: None,
        })
    }

    /// Send one translate request to the running runtime and wait for the
    /// matching result frame. Returns `(translated_text, wall_clock_ms)`.
    pub fn translate(
        &mut self,
        source_language: &str,
        target_language: &str,
        text: &str,
    ) -> AppResult<(String, u64)> {
        let started = Instant::now();
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = TranslateRequest {
            kind: "translate",
            protocol_version: HY_MT_PROTOCOL_VERSION,
            id: &request_id,
            source_language,
            target_language,
            text,
            max_new_tokens: SERVE_PROBE_MAX_NEW_TOKENS,
        };
        let mut request_line = serde_json::to_string(&request).map_err(|error| {
            AppError::new(
                "hy_mt_serve_protocol_error",
                format!("Could not encode the translate request: {error}"),
            )
        })?;
        request_line.push('\n');
        self.stdin
            .write_all(request_line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|error| {
                AppError::new(
                    "hy_mt_serve_io_error",
                    format!("Could not send the translate request: {error}"),
                )
            })?;

        loop {
            let raw = read_serve_line_without_guard(
                &self.receiver,
                SERVE_TRANSLATE_TIMEOUT,
                "hy_mt_serve_timeout",
                "The offline Hy-MT2 engine did not answer the translate request in time.",
            )?;
            let frame = parse_serve_frame(&raw)?;
            let frame_id = frame.id.as_deref();
            match frame.kind.as_str() {
                "result" if frame_id == Some(request_id.as_str()) => {
                    let translated_text = frame.text.filter(|text| !text.trim().is_empty());
                    let translated_text = match translated_text {
                        Some(value) => value,
                        None => {
                            return Err(AppError::new(
                                "hy_mt_translate_failed",
                                "The offline Hy-MT2 engine returned an empty translation.",
                            ));
                        }
                    };
                    self.last_request_latency_ms = frame.latency_ms;
                    let elapsed = started.elapsed().as_millis().min(86_400_000) as u64;
                    return Ok((translated_text, elapsed));
                }
                "cancelled" if frame_id == Some(request_id.as_str()) => {
                    return Err(AppError::new(
                        "hy_mt_translate_cancelled",
                        "The translation was cancelled before completing.",
                    ));
                }
                "error" => {
                    return Err(AppError::new(
                        "hy_mt_translate_failed",
                        frame.message.unwrap_or_else(|| {
                            "The offline Hy-MT2 engine rejected the translate request.".to_string()
                        }),
                    ));
                }
                _ => {}
            }
        }
    }

    /// Cheap accessor used by callers that need the protocol-reported
    /// translation latency (separate from wall-clock) for diagnostics.
    pub fn last_protocol_latency_ms(&self) -> Option<f64> {
        self.last_request_latency_ms
    }
}

fn directory_size(path: &Path) -> std::io::Result<u64> {
    if !path.exists() || path.is_symlink() {
        return Ok(0);
    }
    let mut total = 0_u64;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            total = total.saturating_add(directory_size(&entry.path())?);
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> AppError {
    AppError::new("hy_mt_state_lock_error", error.to_string())
}

#[cfg(target_os = "windows")]
fn hide_child_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(0x0800_0000);
}

#[cfg(not(target_os = "windows"))]
fn hide_child_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_constants_are_pinned() {
        assert_eq!(HY_MT_MODEL_ID, "tencent/Hy-MT2-1.8B");
        assert_eq!(HY_MT_MODEL_REVISION.len(), 40);
        assert_eq!(
            HY_MT_TOTAL_BYTES,
            1777 + 11629 + 14763 + 654 + 1348 + 221 + 4077072784 + 488 + 9527287 + 165815
        );
        assert_eq!(offline_model_spec(OfflineTranslationModel::HyMt2).key, "hy-mt2");
    }

    #[test]
    fn translategemma_spec_is_pinned_and_gated() {
        let spec = offline_model_spec(OfflineTranslationModel::TranslateGemma4B);
        assert_eq!(spec.key, "translategemma-4b");
        assert_eq!(spec.model_id, "google/translategemma-4b-it");
        assert_eq!(spec.revision, "10042cb0e6e7fdce748996a71dc3dc432a4e0c89");
        assert_eq!(spec.revision.len(), 40);
        assert_eq!(spec.total_bytes, 8_639_637_704);
        assert!(spec.gated);
        assert!(!HY_MT2_SPEC.gated);
        assert_ne!(HY_MT2_SPEC.revision, spec.revision);
    }

    #[test]
    fn bridge_frames_use_camel_case_payloads() {
        let frame: BridgeFrame = serde_json::from_str(
            r#"{"type":"progress","state":"downloading","downloadedBytes":12,"totalBytes":24}"#,
        )
        .unwrap();
        assert_eq!(frame.kind, "progress");
        assert_eq!(frame.downloaded_bytes, Some(12));
        assert_eq!(frame.total_bytes, Some(24));
        assert_eq!(
            map_state_to_phase(frame.state.as_deref()),
            Some(HyMtModelPhase::Downloading)
        );

        let complete: BridgeFrame = serde_json::from_str(
            r#"{"type":"complete","state":"installed","downloadedBytes":4086796766,"totalBytes":4086796766}"#,
        )
        .unwrap();
        assert_eq!(complete.kind, "complete");
        assert_eq!(
            map_state_to_phase(complete.state.as_deref()),
            Some(HyMtModelPhase::Installed)
        );

        let error: BridgeFrame = serde_json::from_str(
            r#"{"type":"error","code":"lifecycle_failed","message":"Model lifecycle operation failed.","retryable":false}"#,
        )
        .unwrap();
        assert_eq!(error.kind, "error");
        assert!(error.message.is_some());
        assert_eq!(map_state_to_phase(error.state.as_deref()), None);
    }

    #[test]
    fn progress_percentage_is_bounded() {
        assert_eq!(progress_percent(50, 100), Some(50));
        assert_eq!(progress_percent(101, 100), Some(100));
        assert_eq!(progress_percent(50, 0), None);
    }

    #[test]
    fn translate_request_serializes_bounded_protocol_v1() {
        let request = TranslateRequest {
            kind: "translate",
            protocol_version: 1,
            id: "probe-1",
            source_language: "ja",
            target_language: "vi",
            text: "こんにちは",
            max_new_tokens: 64,
        };
        let encoded = serde_json::to_string(&request).unwrap();
        assert_eq!(
            encoded,
            r#"{"type":"translate","protocolVersion":1,"id":"probe-1","sourceLanguage":"ja","targetLanguage":"vi","text":"こんにちは","maxNewTokens":64}"#
        );
    }

    #[test]
    fn serve_frames_parse_ready_result_and_error_payloads() {
        let ready: ServeFrame = serde_json::from_str(
            r#"{"type":"ready","protocolVersion":1,"runtimeVersion":"0.2.0","modelId":"tencent/Hy-MT2-1.8B","revision":"9a341cd1b679d3efd23b46e847b01745a71ed792","trustRemoteCode":false,"device":"mps:0","dtype":"bfloat16","pid":123,"loadMs":1234.5}"#,
        )
        .unwrap();
        assert_eq!(ready.kind, "ready");
        assert_eq!(ready.protocol_version, Some(1));
        assert_eq!(ready.model_id.as_deref(), Some("tencent/Hy-MT2-1.8B"));
        assert_eq!(
            ready.revision.as_deref(),
            Some("9a341cd1b679d3efd23b46e847b01745a71ed792")
        );
        assert_eq!(ready.device.as_deref(), Some("mps:0"));
        assert_eq!(ready.load_ms, Some(1234.5));

        let result: ServeFrame = serde_json::from_str(
            r#"{"type":"result","id":"probe-1","text":"Xin chào","inputTokens":3,"outputTokens":3,"latencyMs":7.25}"#,
        )
        .unwrap();
        assert_eq!(result.kind, "result");
        assert_eq!(result.id.as_deref(), Some("probe-1"));
        assert_eq!(result.text.as_deref(), Some("Xin chào"));
        assert_eq!(result.latency_ms, Some(7.25));

        let error: ServeFrame = serde_json::from_str(
            r#"{"type":"error","code":"inference_failed","message":"Translation could not be completed.","retryable":true,"id":"probe-1"}"#,
        )
        .unwrap();
        assert_eq!(error.kind, "error");
        assert_eq!(
            error.message.as_deref(),
            Some("Translation could not be completed.")
        );
    }

    #[test]
    fn preferred_serve_device_matches_platform() {
        let device = preferred_serve_device();
        if cfg!(target_os = "macos") {
            assert_eq!(device, "mps");
        } else {
            assert_eq!(device, "cpu");
        }
    }

    #[cfg(unix)]
    #[test]
    fn serve_probe_translates_through_fake_runtime() {
        let directory =
            std::env::temp_dir().join(format!("baka-trans-hy-mt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("fake-hy-mt.sh");
        std::fs::write(
            &script,
            concat!(
                "#!/bin/sh\n",
                "printf '%s\\n' '{\"type\":\"ready\",\"protocolVersion\":1,\"runtimeVersion\":\"0.2.0\",\"modelId\":\"tencent/Hy-MT2-1.8B\",\"revision\":\"9a341cd1b679d3efd23b46e847b01745a71ed792\",\"trustRemoteCode\":false,\"device\":\"cpu\",\"dtype\":\"float32\",\"pid\":1,\"loadMs\":25.5}'\n",
                "IFS= read -r line\n",
                "id=$(printf '%s' \"$line\" | sed -n 's/.*\"id\":\"\\([^\"]*\\)\".*/\\1/p')\n",
                "printf '{\"type\":\"result\",\"id\":\"%s\",\"text\":\"Xin chào\",\"inputTokens\":3,\"outputTokens\":3,\"latencyMs\":4.5}\\n' \"$id\"\n",
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let spec = CommandSpec {
            program: script,
            prefix_args: Vec::new(),
            working_dir: directory.clone(),
        };
        let probe = run_serve_probe(&spec, &directory, "cpu", &HY_MT2_SPEC).unwrap();

        assert!(probe.reachable);
        assert!(probe.accepted);
        assert!(probe.error.is_none());
        assert_eq!(probe.translated_text.as_deref(), Some("Xin chào"));
        assert_eq!(probe.device, "cpu");
        assert_eq!(probe.load_ms, 25.5);
        assert_eq!(probe.latency_ms, Some(4.5));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn serve_probe_rejects_identity_mismatch() {
        let directory =
            std::env::temp_dir().join(format!("baka-trans-hy-mt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("fake-hy-mt.sh");
        std::fs::write(
            &script,
            concat!(
                "#!/bin/sh\n",
                "printf '%s\\n' '{\"type\":\"ready\",\"protocolVersion\":1,\"modelId\":\"other/model\",\"revision\":\"deadbeef\",\"device\":\"cpu\",\"loadMs\":1}'\n",
            ),
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let spec = CommandSpec {
            program: script,
            prefix_args: Vec::new(),
            working_dir: directory.clone(),
        };
        let error = run_serve_probe(&spec, &directory, "cpu", &HY_MT2_SPEC).unwrap_err();
        assert_eq!(error.code, "hy_mt_identity_mismatch");
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    #[ignore = "requires the packaged Hy-MT sidecar and an installed verified model"]
    fn serve_probe_translates_through_real_runtime() {
        let sidecar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("sidecars")
            .join("hy-mt")
            .join("bundle")
            .join("hy-mt-sidecar");
        if !sidecar.is_file() {
            eprintln!("packaged sidecar not found; nothing to probe");
            return;
        }
        let paths = ManagedPaths::resolve_for(&HY_MT2_SPEC).unwrap();
        if !paths.manifest_path().is_file() {
            eprintln!("verified model not installed; nothing to probe");
            return;
        }
        let spec = CommandSpec {
            working_dir: sidecar.parent().unwrap().to_path_buf(),
            program: sidecar,
            prefix_args: Vec::new(),
        };
        let probe = run_serve_probe(&spec, &paths.model_root, preferred_serve_device(), &HY_MT2_SPEC).unwrap();
        assert!(probe.reachable);
        assert!(probe.accepted);
        assert!(probe.error.is_none());
        println!(
            "device={} load_ms={} latency_ms={:?} text={:?}",
            probe.device, probe.load_ms, probe.latency_ms, probe.translated_text
        );
    }

    #[test]
    fn managed_paths_stay_inside_shared_cache() {
        let paths = ManagedPaths::resolve_for(&HY_MT2_SPEC).unwrap();
        assert!(paths
            .model_root
            .ends_with(Path::new(".bakatrans").join("hy-mt")));
        assert!(paths.staging_dir.starts_with(&paths.model_root));
        assert!(paths
            .manifest_path()
            .ends_with(Path::new("active").join("install-manifest.json")));
        // Hy-MT2 keeps the legacy unscoped layout: the revision staging dir
        // hangs directly off the hy-mt cache root.
        assert!(paths
            .staging_dir
            .ends_with(Path::new(".staging").join(HY_MT_MODEL_REVISION)));

        let gemma = ManagedPaths::resolve_for(&TRANSLATEGEMMA_4B_SPEC).unwrap();
        assert!(gemma
            .model_root
            .ends_with(Path::new(".bakatrans").join("hy-mt").join("translategemma-4b")));
        assert!(gemma
            .staging_dir
            .ends_with(Path::new(".staging").join(TRANSLATEGEMMA_4B_SPEC.revision)));
        assert_ne!(paths.model_root, gemma.model_root);
        assert_ne!(paths.manifest_path(), gemma.manifest_path());
    }
}
