<cite>
- package.json
- .github/workflows/desktop.yml
- src-tauri/Cargo.toml
- scripts/release-macos.sh
- scripts/release-windows.ps1
</cite>

# Development & Contributing

## Table of Contents

- [Development loop](#development-loop)
- [Verification](#verification)
- [Release boundaries](#release-boundaries)

## Development loop

**Verified.** Frontend changes are built with `npm run build` and tested with `npm test`. Native changes can be checked with `cargo test --manifest-path src-tauri/Cargo.toml`. The app is launched with `npm run tauri -- dev`.

## Verification

**Verified.** GitHub Actions runs on pushes and pull requests for macOS and Windows, executes `npm ci`, `npm run build`, `npm test`, and Rust tests. Windows CI additionally builds an NSIS installer and uploads it as an artifact.

## Release boundaries

**Verified.** macOS release automation requires a clean `main` checkout, matching version fields, signing identity validation, tests, Rust format/check/test, and a signed app/DMG verification. Windows release automation builds the VieNeu sidecar before producing the NSIS installer. Do not add keys or notarization credentials to repository files.
