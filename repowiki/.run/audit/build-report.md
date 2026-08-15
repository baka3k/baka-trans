# Build audit

**Verified:** `npm run build` runs TypeScript then Vite; `npm test` invokes Vitest; `npm run tauri -- dev` starts Tauri development. Rust tests run through `cargo test --manifest-path src-tauri/Cargo.toml`.

**Prerequisites:** Node/npm, Rust/Cargo, CMake, platform C/C++ toolchain. Windows releases require `uv`; macOS release verification also uses git, gh, codesign, hdiutil, security, and optionally xcrun.

**Evidence:** `package.json`, `README.md`, `scripts/release-macos.sh`, `scripts/release-windows.ps1`.
