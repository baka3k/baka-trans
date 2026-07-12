# Phase 01: Cross-Platform Build and Credential Foundation

Status: implemented; macOS CI validation pending

## Goal

Make the normal application compile and launch on Windows while retaining the existing macOS build.

## Implementation Scope

1. Refactor `src-tauri/Cargo.toml` so Apple and Windows keyring backends/features are target appropriate; retain Objective-C/Vision crates under the existing macOS target section.
2. Remove global reliance on Tauri `macos-private-api` where possible, or gate it through macOS-specific configuration.
3. Split `src-tauri/tauri.conf.json` into shared settings plus platform overrides: macOS keeps DMG/app and Apple signing; Windows enables NSIS and optionally MSI without Apple-only fields.
4. Add PowerShell-friendly Windows development/release scripts in `scripts/` and package scripts in `package.json`; keep the macOS release flow intact.
5. Make product metadata and platform labels accurate in `src-tauri/Cargo.toml`, `README.md`, and relevant UI strings.
6. Add Windows CI for frontend build/tests, Rust tests/check, and an unsigned NSIS smoke artifact.

## Verification

- `npm ci`
- `npm run build`
- `npm test`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npm run tauri -- dev` on Windows
- `npm run tauri -- build --bundles nsis` on Windows
- Repeat the existing macOS build/check in macOS CI.

## Exit Criteria

The main window launches on Windows, secrets round-trip through Windows Credential Manager, and both Windows and macOS CI paths compile from a clean checkout.
