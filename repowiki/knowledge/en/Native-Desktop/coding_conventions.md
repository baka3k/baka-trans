# Observed conventions

Rust errors use `AppResult<T>` and structured `AppError` codes. Values crossing IPC use Serde with explicit casing. Long or blocking native operations are moved through Tauri's async runtime. State is protected by `Mutex` and atomic generation counters where session cancellation/staleness must be guarded.
