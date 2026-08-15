use crate::error::{AppError, AppResult};
use crate::models::{HyMtModelPhase, HyMtModelProgress, HyMtModelStatus};
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

    pub async fn status(&self, app: &AppHandle) -> AppResult<HyMtModelStatus> {
        let paths = ManagedPaths::resolve()?;
        let runtime_available = resolve_command(app).is_ok();
        let installing = self.install_active.load(Ordering::Acquire);
        let last_error = self.last_error.lock().map_err(lock_error)?.clone();
        let manifest_installed = paths.manifest_path().is_file();
        let partial_bytes = directory_size(&paths.staging_dir).unwrap_or(0);

        let (phase, message) = if !runtime_available {
            (
                HyMtModelPhase::Unsupported,
                "This build does not include the managed Hy-MT2 runtime.".to_string(),
            )
        } else if installing {
            (
                HyMtModelPhase::Downloading,
                "Downloading the Hy-MT2 model…".to_string(),
            )
        } else if let Some(error) = last_error {
            (HyMtModelPhase::Error, error)
        } else if manifest_installed {
            (
                HyMtModelPhase::Installed,
                "Hy-MT2 model is installed and verified.".to_string(),
            )
        } else if partial_bytes > 0 {
            (
                HyMtModelPhase::Paused,
                "Hy-MT2 setup can be resumed.".to_string(),
            )
        } else {
            (
                HyMtModelPhase::NotInstalled,
                "Install Hy-MT2 to download the pinned offline translation model.".to_string(),
            )
        };

        Ok(HyMtModelStatus {
            phase,
            runtime_available,
            model_installed: manifest_installed,
            model_id: HY_MT_MODEL_ID.to_string(),
            model_revision: HY_MT_MODEL_REVISION.to_string(),
            total_bytes: HY_MT_TOTAL_BYTES,
            message,
        })
    }

    pub async fn install(&self, app: AppHandle) -> AppResult<HyMtModelStatus> {
        let guard = InstallGuard::acquire(self)?;
        self.install_cancelled.store(false, Ordering::Release);
        let repair_requested = self.last_error.lock().map_err(lock_error)?.is_some();
        *self.last_error.lock().map_err(lock_error)? = None;

        let paths = ManagedPaths::resolve()?;
        if paths.manifest_path().is_file() && !repair_requested {
            drop(guard);
            return self.status(&app).await;
        }
        let spec = resolve_command(&app)?;
        let cancelled = self.install_cancelled.clone();
        let staging_dir = paths.staging_dir.clone();
        let model_root = paths.model_root.clone();
        let progress_app = app.clone();
        let exit_status = match tauri::async_runtime::spawn_blocking(move || {
            run_install_process(spec, model_root, staging_dir, progress_app, cancelled)
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
            return self.status(&app).await;
        }
        if !paths.manifest_path().is_file() {
            let error = AppError::new(
                "hy_mt_model_activation_error",
                "Hy-MT2 finished setup without an active verified model. Retry the install.",
            );
            self.remember_error(&error);
            return Err(error);
        }
        emit_progress(
            &app,
            HyMtModelPhase::Installed,
            HY_MT_TOTAL_BYTES,
            HY_MT_TOTAL_BYTES,
            Some(100),
            "Hy-MT2 model is installed and verified.",
        );
        drop(guard);
        self.status(&app).await
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
    fn resolve() -> AppResult<Self> {
        let model_root = model_cache_dir()?.join(HY_MT_CACHE_SUBDIR);
        Ok(Self {
            staging_dir: model_root.join(STAGING_DIR_NAME).join(HY_MT_MODEL_REVISION),
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
    spec: CommandSpec,
    model_root: PathBuf,
    staging_dir: PathBuf,
    app: AppHandle,
    cancelled: Arc<AtomicBool>,
) -> AppResult<InstallExit> {
    let mut command = command_from_spec(&spec);
    command
        .arg("install")
        .arg("--model-root")
        .arg(&model_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_child_window(&mut command);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            "hy_mt_runtime_missing",
            format!("Could not start the bundled Hy-MT2 installer: {error}"),
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
        phase,
        0,
        HY_MT_TOTAL_BYTES,
        Some(0),
        "Preparing the verified Hy-MT2 download…",
    );

    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            emit_progress(
                &app,
                HyMtModelPhase::Paused,
                directory_size(&staging_dir)
                    .unwrap_or(0)
                    .min(HY_MT_TOTAL_BYTES),
                HY_MT_TOTAL_BYTES,
                None,
                "Hy-MT2 setup was paused. Resume when ready.",
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
                        let total_bytes = frame.total_bytes.unwrap_or(HY_MT_TOTAL_BYTES);
                        if frame.kind == "complete" {
                            phase = HyMtModelPhase::Installed;
                            downloaded_bytes = total_bytes.max(downloaded_bytes);
                        }
                        emit_progress(
                            &app,
                            phase,
                            downloaded_bytes.min(total_bytes.max(HY_MT_TOTAL_BYTES)),
                            HY_MT_TOTAL_BYTES,
                            progress_percent(downloaded_bytes, total_bytes),
                            default_progress_message(phase),
                        );
                    }
                    "error" => {
                        frame_error = Some(frame.message.unwrap_or_else(|| {
                            "Hy-MT2 setup failed. Resume to retry the verified download."
                                .to_string()
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
                .min(HY_MT_TOTAL_BYTES);
            if staged > downloaded_bytes {
                downloaded_bytes = staged;
                emit_progress(
                    &app,
                    phase,
                    downloaded_bytes,
                    HY_MT_TOTAL_BYTES,
                    progress_percent(downloaded_bytes, HY_MT_TOTAL_BYTES),
                    default_progress_message(phase),
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
                "Hy-MT2 setup failed. Resume to retry the verified download.",
            ));
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn default_progress_message(phase: HyMtModelPhase) -> &'static str {
    match phase {
        HyMtModelPhase::Downloading => "Downloading the verified Hy-MT2 model…",
        HyMtModelPhase::Verifying => "Verifying Hy-MT2 model files…",
        HyMtModelPhase::Installed => "Hy-MT2 model is installed and verified.",
        _ => "Hy-MT2 setup is running…",
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
    phase: HyMtModelPhase,
    downloaded_bytes: u64,
    total_bytes: u64,
    percent: Option<u8>,
    message: &str,
) {
    let _ = app.emit(
        EVENT_NAME,
        HyMtModelProgress {
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
pub async fn probe_translation_engine(app: &AppHandle) -> HyMtEngineProbe {
    let app = app.clone();
    match tauri::async_runtime::spawn_blocking(move || probe_offline_engine(&app)).await {
        Ok(probe) => probe,
        Err(error) => HyMtEngineProbe::failed(AppError::new(
            "hy_mt_probe_join_error",
            format!("The offline Hy-MT2 engine test could not run: {error}"),
        )),
    }
}

fn probe_offline_engine(app: &AppHandle) -> HyMtEngineProbe {
    let paths = match ManagedPaths::resolve() {
        Ok(paths) => paths,
        Err(error) => return HyMtEngineProbe::failed(error),
    };
    if !paths.manifest_path().is_file() {
        return HyMtEngineProbe::failed(AppError::new(
            "hy_mt_model_missing",
            "The Hy-MT2 model is not installed yet. Install it from the model card before testing the engine.",
        ));
    }
    let spec = match resolve_command(app) {
        Ok(spec) => spec,
        Err(error) => return HyMtEngineProbe::failed(error),
    };
    let device = preferred_serve_device();
    match run_serve_probe(&spec, &paths.model_root, device) {
        Ok(probe) => probe,
        Err(error) => {
            // An Intel Mac or a machine without a usable MPS build exits before
            // the ready frame; retry once on CPU instead of failing the test.
            if device == "mps" && error.code == "hy_mt_serve_early_exit" {
                run_serve_probe(&spec, &paths.model_root, "cpu")
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
    spec: &CommandSpec,
    model_root: &Path,
    device: &str,
) -> AppResult<HyMtEngineProbe> {
    let mut session = HyMtSession::start_with_spec(spec, model_root, device)?;
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
    pub fn start(app: &AppHandle) -> AppResult<Self> {
        let paths = ManagedPaths::resolve()?;
        if !paths.manifest_path().is_file() {
            return Err(AppError::new(
                "hy_mt_model_missing",
                "The Hy-MT2 model is not installed yet. Install it from the model card before starting the engine.",
            ));
        }
        let spec = resolve_command(app)?;
        let device = preferred_serve_device();
        match Self::start_with_spec(&spec, &paths.model_root, device) {
            Ok(session) => Ok(session),
            // Intel Macs and machines without a usable MPS build exit before
            // the ready frame; retry once on CPU instead of failing outright.
            Err(error) if device == "mps" && error.code == "hy_mt_serve_early_exit" => {
                Self::start_with_spec(&spec, &paths.model_root, "cpu")
            }
            Err(error) => Err(error),
        }
    }

    /// Internal helper used directly by [`run_serve_probe`] with a synthetic
    /// `CommandSpec` and by [`start`](Self::start) for the managed runtime.
    pub(crate) fn start_with_spec(
        spec: &CommandSpec,
        model_root: &Path,
        device: &str,
    ) -> AppResult<Self> {
        let mut command = command_from_spec(spec);
        command
            .arg("serve")
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
            || ready.model_id.as_deref() != Some(HY_MT_MODEL_ID)
            || ready.revision.as_deref() != Some(HY_MT_MODEL_REVISION)
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "hy_mt_identity_mismatch",
                "The Hy-MT2 runtime reported an unexpected model identity.",
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
        let probe = run_serve_probe(&spec, &directory, "cpu").unwrap();

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
        let error = run_serve_probe(&spec, &directory, "cpu").unwrap_err();
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
        let paths = ManagedPaths::resolve().unwrap();
        if !paths.manifest_path().is_file() {
            eprintln!("verified model not installed; nothing to probe");
            return;
        }
        let spec = CommandSpec {
            working_dir: sidecar.parent().unwrap().to_path_buf(),
            program: sidecar,
            prefix_args: Vec::new(),
        };
        let probe = run_serve_probe(&spec, &paths.model_root, preferred_serve_device()).unwrap();
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
        let paths = ManagedPaths::resolve().unwrap();
        assert!(paths
            .model_root
            .ends_with(Path::new(".bakatrans").join("hy-mt")));
        assert!(paths.staging_dir.starts_with(&paths.model_root));
        assert!(paths
            .manifest_path()
            .ends_with(Path::new("active").join("install-manifest.json")));
    }
}
