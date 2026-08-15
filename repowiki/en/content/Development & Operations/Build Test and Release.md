<cite>
- package.json
- vite.config.ts
- .github/workflows/desktop.yml
- scripts/release-macos.sh
- scripts/release-windows.ps1
- scripts/build-vieneu-sidecar.ps1
</cite>

# Build, Test, and Release

## Table of Contents

- [Commands](#commands)
- [CI](#ci)
- [Packaging](#packaging)
- [Troubleshooting](#troubleshooting)

## Commands

| Command | Verified purpose |
| --- | --- |
| `npm run dev` | Start Vite frontend server |
| `npm run build` | Type-check and build frontend assets |
| `npm test` | Run Vitest suite once |
| `npm run tauri -- dev` | Start desktop development runtime |
| `npm run release:windows:check` | Build/test validation on Windows without installer |
| `npm run release:windows` | Build VieNeu bridge and Windows NSIS installer |
| `npm run release:check -- <version>` | macOS release preflight/build/verification |

## CI

**Verified.** Desktop checks run on macOS and Windows for both push and pull request. They build frontend assets and run frontend/Rust tests. The Windows workflow also produces an NSIS artifact.

## Packaging

**Verified.** Tauri bundles the sidecar resource. macOS release scripting verifies code signatures and DMG integrity, optionally checks notarization, and requires a clean synchronized `main`; Windows packages per-user NSIS output.

## Troubleshooting

**Verified.** If native builds fail before compilation, check CMake and the platform C/C++ toolchain. For local translation, follow the README error-code table: readable Whisper model, reachable Ollama/model, installed or repaired VieNeu runtime, selected voice, and valid output device are the first diagnostics.
