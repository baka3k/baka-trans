# Native module commands

Run Rust tests from the repository root:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Native Whisper builds require CMake and the appropriate C/C++ toolchain. On macOS, release checks also run `cargo fmt --check` and `cargo check` through `scripts/release-macos.sh`.
