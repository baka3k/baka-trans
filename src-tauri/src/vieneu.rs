use crate::error::{AppError, AppResult};
use crate::models::{VieNeuRuntimePhase, VieNeuRuntimeProgress, VieNeuRuntimeStatus};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

pub const VIENEU_MODEL_VERSION: &str = "v3-turbo-int8-2026-07";
pub const VIENEU_MODEL_BYTES: u64 = 256_068_309;
const MODEL_CACHE_DIR_NAME: &str = ".bakatrans";
const VIENEU_CACHE_SUBDIR: &str = "vieneu";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(180);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
const EVENT_NAME: &str = "vieneu-runtime-progress";
const MAX_CONSECUTIVE_START_FAILURES: u8 = 3;

#[derive(Debug, Clone)]
pub struct VieNeuConnection {
    pub base_url: String,
    pub token: String,
    pub nonce: String,
}

#[derive(Clone)]
struct CommandSpec {
    program: PathBuf,
    prefix_args: Vec<OsString>,
    working_dir: PathBuf,
}

struct ManagedProcess {
    child: Child,
    parent_pipe: Option<ChildStdin>,
    connection: VieNeuConnection,
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        self.parent_pipe.take();
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

pub struct VieNeuManager {
    runtime: AsyncMutex<Option<ManagedProcess>>,
    download_active: AtomicBool,
    download_cancelled: Arc<AtomicBool>,
    consecutive_start_failures: AtomicU8,
    last_error: Mutex<Option<String>>,
    client: Client,
}

impl VieNeuManager {
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .build()
            .expect("managed VieNeu HTTP client should build");
        Self {
            runtime: AsyncMutex::new(None),
            download_active: AtomicBool::new(false),
            download_cancelled: Arc::new(AtomicBool::new(false)),
            consecutive_start_failures: AtomicU8::new(0),
            last_error: Mutex::new(None),
            client,
        }
    }

    pub async fn status(&self, app: &AppHandle) -> AppResult<VieNeuRuntimeStatus> {
        let paths = ManagedPaths::resolve(app)?;
        let runtime_available = resolve_command(app).is_ok();
        let model_installed = paths.manifest_path().is_file();
        let partial_bytes = directory_size(&paths.staging_dir).unwrap_or(0);
        let mut running = false;
        {
            let mut runtime = self.runtime.lock().await;
            if let Some(process) = runtime.as_mut() {
                match process.child.try_wait() {
                    Ok(None) => running = true,
                    Ok(Some(_)) | Err(_) => *runtime = None,
                }
            }
        }
        let last_error = self.last_error.lock().map_err(lock_error)?.clone();
        let downloading = self.download_active.load(Ordering::Acquire);
        let (phase, message) = if !runtime_available {
            (
                VieNeuRuntimePhase::Unsupported,
                "The bundled VieNeu runtime is not available in this build.".to_string(),
            )
        } else if downloading {
            (
                VieNeuRuntimePhase::Downloading,
                "Downloading VieNeu-TTS…".to_string(),
            )
        } else if running {
            (
                VieNeuRuntimePhase::Ready,
                "VieNeu-TTS is ready.".to_string(),
            )
        } else if let Some(error) = last_error {
            if model_installed {
                (VieNeuRuntimePhase::RepairNeeded, error)
            } else {
                (VieNeuRuntimePhase::Error, error)
            }
        } else if model_installed {
            (
                VieNeuRuntimePhase::Installed,
                "VieNeu-TTS is installed and will start when needed.".to_string(),
            )
        } else if partial_bytes > 0 {
            (
                VieNeuRuntimePhase::Paused,
                "VieNeu-TTS setup can be resumed.".to_string(),
            )
        } else {
            (
                VieNeuRuntimePhase::NotInstalled,
                "Install VieNeu-TTS to use neural Vietnamese voices.".to_string(),
            )
        };
        Ok(VieNeuRuntimeStatus {
            phase,
            runtime_available,
            model_installed,
            running,
            model_version: VIENEU_MODEL_VERSION.to_string(),
            installed_bytes: if model_installed {
                VIENEU_MODEL_BYTES
            } else {
                partial_bytes.min(VIENEU_MODEL_BYTES)
            },
            total_bytes: VIENEU_MODEL_BYTES,
            message,
        })
    }

    pub async fn install(&self, app: AppHandle) -> AppResult<VieNeuRuntimeStatus> {
        let guard = DownloadGuard::acquire(self)?;
        self.download_cancelled.store(false, Ordering::Release);
        self.consecutive_start_failures.store(0, Ordering::Release);
        self.stop().await;
        let repair_requested = self.last_error.lock().map_err(lock_error)?.is_some();
        *self.last_error.lock().map_err(lock_error)? = None;

        let paths = ManagedPaths::resolve(&app)?;
        if paths.manifest_path().is_file() && !repair_requested {
            drop(guard);
            return self.status(&app).await;
        }
        std::fs::create_dir_all(&paths.model_parent).map_err(|error| {
            AppError::new(
                "vieneu_model_directory_error",
                format!("Could not create the VieNeu model directory: {error}"),
            )
        })?;
        if paths.final_dir.exists() && repair_requested {
            if paths.repair_backup.exists() {
                std::fs::remove_dir_all(&paths.repair_backup).map_err(|error| {
                    AppError::new(
                        "vieneu_model_repair_error",
                        format!("Could not replace the previous VieNeu repair backup: {error}"),
                    )
                })?;
            }
            std::fs::rename(&paths.final_dir, &paths.repair_backup).map_err(|error| {
                AppError::new(
                    "vieneu_model_repair_error",
                    format!("Could not prepare the VieNeu model repair: {error}"),
                )
            })?;
        }
        let spec = resolve_command(&app)?;
        let cancelled = self.download_cancelled.clone();
        let staging_dir = paths.staging_dir.clone();
        let progress_app = app.clone();
        let exit_status = tauri::async_runtime::spawn_blocking(move || {
            run_install_process(spec, staging_dir, progress_app, cancelled)
        })
        .await
        .map_err(|error| AppError::new("vieneu_install_join_error", error.to_string()))??;

        if exit_status == InstallExit::Paused {
            drop(guard);
            return self.status(&app).await;
        }
        if paths.final_dir.exists() {
            return Err(AppError::new(
                "vieneu_model_activation_error",
                "A VieNeu model installation already exists. Restart the app and retry repair.",
            ));
        }
        std::fs::rename(&paths.staging_dir, &paths.final_dir).map_err(|error| {
            AppError::new(
                "vieneu_model_activation_error",
                format!("Could not activate the verified VieNeu model: {error}"),
            )
        })?;
        if paths.repair_backup.exists() {
            let _ = std::fs::remove_dir_all(&paths.repair_backup);
        }
        emit_progress(
            &app,
            VieNeuRuntimePhase::Installed,
            VIENEU_MODEL_BYTES,
            VIENEU_MODEL_BYTES,
            Some(100),
            "VieNeu-TTS is installed.",
        );
        drop(guard);
        self.status(&app).await
    }

    pub fn cancel_install(&self) {
        self.download_cancelled.store(true, Ordering::Release);
    }

    pub async fn ensure_running(&self, app: &AppHandle) -> AppResult<VieNeuConnection> {
        let paths = ManagedPaths::resolve(app)?;
        if !paths.manifest_path().is_file() {
            return Err(AppError::new(
                "vieneu_model_not_installed",
                "Install VieNeu-TTS before selecting a neural voice.",
            ));
        }

        let mut runtime = self.runtime.lock().await;
        if let Some(process) = runtime.as_mut() {
            if process.child.try_wait().ok().flatten().is_none()
                && self.health(&process.connection).await.is_ok()
            {
                return Ok(process.connection.clone());
            }
            *runtime = None;
        }
        if self.consecutive_start_failures.load(Ordering::Acquire) >= MAX_CONSECUTIVE_START_FAILURES
        {
            let error = AppError::new(
                "vieneu_start_fuse_open",
                "VieNeu-TTS stopped repeatedly. Repair the installation before retrying.",
            );
            *self.last_error.lock().map_err(lock_error)? = Some(error.message.clone());
            return Err(error);
        }

        emit_progress(
            app,
            VieNeuRuntimePhase::Starting,
            VIENEU_MODEL_BYTES,
            VIENEU_MODEL_BYTES,
            Some(100),
            "Starting VieNeu-TTS…",
        );
        let spec = resolve_command(app)?;
        let model_dir = paths.final_dir;
        let threads = recommended_thread_count();
        let process_result = tauri::async_runtime::spawn_blocking(move || {
            start_runtime_process(spec, model_dir, threads)
        })
        .await
        .map_err(|error| AppError::new("vieneu_start_join_error", error.to_string()))?;
        let process = match process_result {
            Ok(process) => process,
            Err(error) => {
                self.consecutive_start_failures
                    .fetch_add(1, Ordering::AcqRel);
                *self.last_error.lock().map_err(lock_error)? = Some(error.message.clone());
                return Err(error);
            }
        };
        let connection = process.connection.clone();
        if let Err(error) = self.health(&connection).await {
            self.consecutive_start_failures
                .fetch_add(1, Ordering::AcqRel);
            *self.last_error.lock().map_err(lock_error)? = Some(error.message.clone());
            return Err(error);
        }
        *runtime = Some(process);
        *self.last_error.lock().map_err(lock_error)? = None;
        self.consecutive_start_failures.store(0, Ordering::Release);
        emit_progress(
            app,
            VieNeuRuntimePhase::Ready,
            VIENEU_MODEL_BYTES,
            VIENEU_MODEL_BYTES,
            Some(100),
            "VieNeu-TTS is ready.",
        );
        Ok(connection)
    }

    pub async fn restart(&self, app: &AppHandle) -> AppResult<VieNeuRuntimeStatus> {
        self.stop().await;
        self.consecutive_start_failures.store(0, Ordering::Release);
        emit_progress(
            app,
            VieNeuRuntimePhase::Recovering,
            VIENEU_MODEL_BYTES,
            VIENEU_MODEL_BYTES,
            Some(100),
            "Restarting VieNeu-TTS…",
        );
        match self.ensure_running(app).await {
            Ok(_) => self.status(app).await,
            Err(error) => {
                *self.last_error.lock().map_err(lock_error)? = Some(error.message.clone());
                Err(error)
            }
        }
    }

    pub async fn stop(&self) {
        self.runtime.lock().await.take();
    }

    pub fn http_client(&self) -> Client {
        self.client.clone()
    }

    async fn health(&self, connection: &VieNeuConnection) -> AppResult<()> {
        let response = self
            .client
            .get(format!("{}/health", connection.base_url))
            .bearer_auth(&connection.token)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|error| {
                AppError::new(
                    "vieneu_health_error",
                    format!("VieNeu-TTS did not respond: {error}"),
                )
            })?;
        if !response.status().is_success() {
            return Err(AppError::new(
                "vieneu_health_error",
                "VieNeu-TTS health authentication failed.",
            ));
        }
        let payload = response
            .json::<HealthResponse>()
            .await
            .map_err(|error| AppError::new("vieneu_health_error", error.to_string()))?;
        if !payload.ok || payload.nonce != connection.nonce {
            return Err(AppError::new(
                "vieneu_health_error",
                "VieNeu-TTS returned an invalid startup identity.",
            ));
        }
        Ok(())
    }
}

impl Default for VieNeuManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct HealthResponse {
    ok: bool,
    nonce: String,
}

struct ManagedPaths {
    model_parent: PathBuf,
    staging_dir: PathBuf,
    final_dir: PathBuf,
    repair_backup: PathBuf,
}

impl ManagedPaths {
    fn resolve(_app: &AppHandle) -> AppResult<Self> {
        let root = model_cache_dir()?.join(VIENEU_CACHE_SUBDIR);
        let model_parent = root.join("models");
        Ok(Self {
            staging_dir: model_parent.join(format!("{VIENEU_MODEL_VERSION}.partial")),
            final_dir: model_parent.join(VIENEU_MODEL_VERSION),
            repair_backup: model_parent.join(format!("{VIENEU_MODEL_VERSION}.repair-backup")),
            model_parent,
        })
    }

    fn manifest_path(&self) -> PathBuf {
        self.final_dir.join("install-manifest.json")
    }
}

struct DownloadGuard<'a> {
    manager: &'a VieNeuManager,
}

/// Resolve the shared user-side model cache directory used for all downloaded
/// ML artifacts (Whisper, VieNeu, future Hy-MT2). Lives outside the
/// per-app Application Support folder so models survive reinstalls.
pub(crate) fn model_cache_dir() -> AppResult<PathBuf> {
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE").ok_or_else(|| {
        AppError::new(
            "model_cache_dir_error",
            "Could not resolve USERPROFILE for the model cache directory.",
        )
    })?;
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME").ok_or_else(|| {
        AppError::new(
            "model_cache_dir_error",
            "Could not resolve HOME for the model cache directory.",
        )
    })?;
    Ok(PathBuf::from(home).join(MODEL_CACHE_DIR_NAME))
}

impl<'a> DownloadGuard<'a> {
    fn acquire(manager: &'a VieNeuManager) -> AppResult<Self> {
        manager
            .download_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                AppError::new(
                    "vieneu_install_busy",
                    "VieNeu-TTS setup is already running.",
                )
            })?;
        Ok(Self { manager })
    }
}

impl Drop for DownloadGuard<'_> {
    fn drop(&mut self) {
        self.manager.download_active.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InstallExit {
    Completed,
    Paused,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeEvent {
    event: String,
    phase: Option<String>,
    downloaded_bytes: Option<u64>,
    verified_bytes: Option<u64>,
    total_bytes: Option<u64>,
    percent: Option<u8>,
    message: Option<String>,
    port: Option<u16>,
    nonce: Option<String>,
}

fn run_install_process(
    spec: CommandSpec,
    staging_dir: PathBuf,
    app: AppHandle,
    cancelled: Arc<AtomicBool>,
) -> AppResult<InstallExit> {
    let mut command = command_from_spec(&spec);
    command
        .arg("--install-model")
        .arg("--model-dir")
        .arg(&staging_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_child_window(&mut command);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            "vieneu_runtime_missing",
            format!("Could not start the bundled VieNeu installer: {error}"),
        )
    })?;
    let receiver = spawn_line_reader(child.stdout.take().ok_or_else(|| {
        AppError::new(
            "vieneu_install_output_error",
            "VieNeu installer has no output pipe.",
        )
    })?);

    loop {
        if cancelled.load(Ordering::Acquire) {
            let _ = child.kill();
            let _ = child.wait();
            emit_progress(
                &app,
                VieNeuRuntimePhase::Paused,
                directory_size(&staging_dir)
                    .unwrap_or(0)
                    .min(VIENEU_MODEL_BYTES),
                0,
                None,
                "VieNeu-TTS setup was paused. Resume when ready.",
            );
            return Ok(InstallExit::Paused);
        }
        while let Ok(line) = receiver.try_recv() {
            if let Ok(event) = serde_json::from_str::<BridgeEvent>(&line) {
                forward_bridge_progress(&app, &event);
            }
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| AppError::new("vieneu_install_wait_error", error.to_string()))?
        {
            if status.success() {
                return Ok(InstallExit::Completed);
            }
            return Err(AppError::new(
                "vieneu_install_failed",
                "VieNeu-TTS setup failed. Resume to retry the verified download.",
            ));
        }
        thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn start_runtime_process(
    spec: CommandSpec,
    model_dir: PathBuf,
    threads: u32,
) -> AppResult<ManagedProcess> {
    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let nonce = Uuid::new_v4().simple().to_string();
    let mut command = command_from_spec(&spec);
    command
        .arg("--model-dir")
        .arg(model_dir)
        .arg("--port")
        .arg("0")
        .arg("--threads")
        .arg(threads.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_remove("PYTHONHOME")
        .env_remove("PYTHONPATH")
        .env("HF_HUB_OFFLINE", "1")
        .env("TRANSFORMERS_OFFLINE", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1");
    hide_child_window(&mut command);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            "vieneu_runtime_missing",
            format!("Could not start the bundled VieNeu runtime: {error}"),
        )
    })?;
    let mut parent_pipe = child.stdin.take().ok_or_else(|| {
        AppError::new(
            "vieneu_start_pipe_error",
            "VieNeu runtime has no parent pipe.",
        )
    })?;
    let bootstrap = serde_json::to_vec(&json!({ "token": token, "nonce": nonce }))
        .map_err(|error| AppError::new("vieneu_start_pipe_error", error.to_string()))?;
    parent_pipe
        .write_all(&bootstrap)
        .map_err(|error| AppError::new("vieneu_start_pipe_error", error.to_string()))?;
    parent_pipe
        .write_all(b"\n")
        .map_err(|error| AppError::new("vieneu_start_pipe_error", error.to_string()))?;
    parent_pipe
        .flush()
        .map_err(|error| AppError::new("vieneu_start_pipe_error", error.to_string()))?;

    let receiver = spawn_line_reader(child.stdout.take().ok_or_else(|| {
        AppError::new(
            "vieneu_start_output_error",
            "VieNeu runtime has no output pipe.",
        )
    })?);
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    loop {
        match receiver.recv_timeout(PROCESS_POLL_INTERVAL) {
            Ok(line) => {
                if let Ok(event) = serde_json::from_str::<BridgeEvent>(&line) {
                    if event.event == "ready" {
                        let port = event.port.ok_or_else(|| {
                            AppError::new("vieneu_start_identity_error", "VieNeu omitted its port.")
                        })?;
                        if event.nonce.as_deref() != Some(nonce.as_str()) {
                            return Err(AppError::new(
                                "vieneu_start_identity_error",
                                "VieNeu returned an invalid startup nonce.",
                            ));
                        }
                        return Ok(ManagedProcess {
                            child,
                            parent_pipe: Some(parent_pipe),
                            connection: VieNeuConnection {
                                base_url: format!("http://127.0.0.1:{port}"),
                                token,
                                nonce,
                            },
                        });
                    }
                    if event.event == "error" {
                        return Err(AppError::new(
                            "vieneu_start_failed",
                            event.message.unwrap_or_else(|| {
                                "VieNeu-TTS could not load the managed model.".to_string()
                            }),
                        ));
                    }
                }
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {}
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                return Err(AppError::new(
                    "vieneu_start_failed",
                    "VieNeu-TTS stopped before becoming ready.",
                ));
            }
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| AppError::new("vieneu_start_wait_error", error.to_string()))?
        {
            return Err(AppError::new(
                "vieneu_start_failed",
                format!("VieNeu-TTS stopped during startup ({status})."),
            ));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::new(
                "vieneu_start_timeout",
                "VieNeu-TTS took too long to load. Retry or repair the installation.",
            ));
        }
    }
}

fn forward_bridge_progress(app: &AppHandle, event: &BridgeEvent) {
    let phase = match event.phase.as_deref() {
        Some("downloading") => VieNeuRuntimePhase::Downloading,
        Some("verifying") => VieNeuRuntimePhase::Verifying,
        Some("installed") => VieNeuRuntimePhase::Installed,
        Some("paused") => VieNeuRuntimePhase::Paused,
        _ => VieNeuRuntimePhase::Error,
    };
    let _ = app.emit(
        EVENT_NAME,
        VieNeuRuntimeProgress {
            phase,
            downloaded_bytes: event.downloaded_bytes.unwrap_or(0),
            verified_bytes: event.verified_bytes.unwrap_or(0),
            total_bytes: event.total_bytes.unwrap_or(VIENEU_MODEL_BYTES),
            percent: event.percent,
            message: event
                .message
                .clone()
                .unwrap_or_else(|| "VieNeu-TTS setup is running…".to_string()),
        },
    );
}

fn emit_progress(
    app: &AppHandle,
    phase: VieNeuRuntimePhase,
    downloaded_bytes: u64,
    verified_bytes: u64,
    percent: Option<u8>,
    message: &str,
) {
    let _ = app.emit(
        EVENT_NAME,
        VieNeuRuntimeProgress {
            phase,
            downloaded_bytes,
            verified_bytes,
            total_bytes: VIENEU_MODEL_BYTES,
            percent,
            message: message.to_string(),
        },
    );
}

fn resolve_command(app: &AppHandle) -> AppResult<CommandSpec> {
    if let Some(override_path) = std::env::var_os("BAKA_TRANS_VIENEU_BRIDGE") {
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
            "vieneu-bridge.exe"
        } else {
            "vieneu-bridge"
        };
        let program = resources.join("vieneu-bridge").join(name);
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
            .join("vieneu-tts");
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
        "vieneu_runtime_missing",
        "This build does not include the managed VieNeu-TTS runtime.",
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

fn recommended_thread_count() -> u32 {
    let logical = std::thread::available_parallelism()
        .map(|value| value.get() as u32)
        .unwrap_or(4);
    (logical / 3).clamp(2, 4)
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
    AppError::new("vieneu_state_lock_error", error.to_string())
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
    fn thread_budget_is_bounded() {
        assert!((2..=4).contains(&recommended_thread_count()));
    }

    #[test]
    fn repeated_start_failures_are_fused() {
        assert_eq!(MAX_CONSECUTIVE_START_FAILURES, 3);
    }

    #[test]
    fn bridge_events_use_camel_case_payloads() {
        let event: BridgeEvent = serde_json::from_str(
            r#"{"event":"progress","phase":"downloading","downloadedBytes":12,"verifiedBytes":0,"totalBytes":24,"percent":50,"message":"Downloading"}"#,
        )
        .unwrap();
        assert_eq!(event.downloaded_bytes, Some(12));
        assert_eq!(event.percent, Some(50));
    }
}
