use crate::error::{AppError, AppResult};
use crate::models::{HyMtModelPhase, HyMtModelProgress, HyMtModelStatus};
use crate::vieneu::model_cache_dir;
use serde::Deserialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as std_mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const HY_MT_MODEL_ID: &str = "tencent/Hy-MT2-1.8B";
pub const HY_MT_MODEL_REVISION: &str = "9a341cd1b679d3efd23b46e847b01745a71ed792";
pub const HY_MT_TOTAL_BYTES: u64 = 4_086_796_766;
const HY_MT_CACHE_SUBDIR: &str = "hy-mt";
const ACTIVE_DIR_NAME: &str = "active";
const STAGING_DIR_NAME: &str = ".staging";
const MANIFEST_NAME: &str = "install-manifest.json";
const EVENT_NAME: &str = "hy-mt-model-progress";
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
const BYTE_PROGRESS_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone)]
struct CommandSpec {
    program: PathBuf,
    prefix_args: Vec<std::ffi::OsString>,
    working_dir: PathBuf,
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
            staging_dir: model_root
                .join(STAGING_DIR_NAME)
                .join(HY_MT_MODEL_REVISION),
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
            .map_err(|_| {
                AppError::new("hy_mt_install_busy", "Hy-MT2 setup is already running.")
            })?;
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
                directory_size(&staging_dir).unwrap_or(0).min(HY_MT_TOTAL_BYTES),
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
            let staged = directory_size(&staging_dir).unwrap_or(0).min(HY_MT_TOTAL_BYTES);
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
    Some(
        ((downloaded_bytes.saturating_mul(100) / total_bytes).min(100)) as u8,
    )
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
        let packaged = sidecar_dir.join("dist").join("hy-mt-sidecar").join("hy-mt-sidecar");
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
