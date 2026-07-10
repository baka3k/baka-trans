# Baka Trans

Real-time meeting translation desktop app built with Tauri, React, TypeScript, and Rust.

## Development

Install dependencies:

```bash
npm ci
```

Run the app in development mode:

```bash
npm run tauri -- dev
```

Build only the frontend assets:

```bash
npm run build
```

## Build Installer

The current Tauri bundle config is set up for macOS only:

- `.dmg` installer
- `.app` application bundle

Build the macOS installer from this repository root:

```bash
npm ci
npm run tauri -- build
```

Build only the `.dmg` installer:

```bash
npm run tauri -- build --bundles dmg
```

After a successful macOS build, artifacts are written under:

```text
src-tauri/target/release/bundle/dmg/
src-tauri/target/release/bundle/macos/
```

For local testing, open the generated `.dmg` or launch the generated `.app`.
For public distribution, add Apple code signing and notarization before sharing the installer.

## Windows Status

The generated macOS `.dmg` and `.app` files do not run on Windows.

Tauri can produce Windows installers, but this app is currently scoped and configured for the macOS MVP. The product docs assume macOS audio routing with BlackHole, so Windows support needs separate build configuration and real audio-routing validation.

To create a Windows build, use a Windows machine or Windows CI with Node.js, Rust, and Microsoft Visual Studio Build Tools installed. Then build with Windows bundle targets:

```powershell
npm ci
npm run tauri -- build --bundles nsis,msi
```

Expected Windows artifacts:

```text
src-tauri\target\release\bundle\nsis\
src-tauri\target\release\bundle\msi\
```

Before treating the Windows build as supported, verify device enumeration, microphone/system-audio routing, credential storage, realtime translation, playback, and transcript export on Windows.
