# Windows Release Guide

## Toolchain

- Windows 10/11 x64
- Node.js 22
- Rust stable with the MSVC target
- Visual Studio Build Tools with **Desktop development with C++**
- WebView2 Runtime (normally present on supported Windows versions)

## Check

```powershell
npm run release:windows:check
```

This installs locked dependencies, builds/tests the frontend, and runs Rust tests.

## Build NSIS Installer

```powershell
npm run release:windows
```

Artifacts and SHA-256 sidecars are written under:

```text
src-tauri\target\release\bundle\nsis\
```

NSIS is the supported baseline. MSI/WiX is not required unless an enterprise distribution policy specifically needs MSI.

## Signing

Public installers should be Authenticode-signed by the release pipeline. Keep certificate material in the CI secret store, never in this repository. Verify the signature with:

```powershell
Get-AuthenticodeSignature .\src-tauri\target\release\bundle\nsis\*.exe
```

## Manual Release Gate

- Clean install and first launch.
- Windows Credential Manager secret save/load.
- WASAPI loopback with speaker, wired/USB headset, and Bluetooth when available.
- Live translation, translated playback, transcript export, and summary.
- Look Through and Look & Help on a browser/PDF and a mixed-DPI second monitor.
- Installer upgrade over the previous release and clean uninstall.
- macOS CI remains green.
