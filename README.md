# Baka Trans

Real-time meeting translation desktop app for macOS and Windows, built with Tauri, React, TypeScript, and Rust.

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

For the versioned build, verification, tag, and GitHub Release workflow, see
[the macOS release guide](docs/RELEASE_GUIDE.md).

## Windows Development

The generated macOS `.dmg` and `.app` files do not run on Windows.

Windows uses its built-in WASAPI loopback capture so users can leave Teams on their normal speaker or headset. No virtual audio driver is required for the default workflow. VB-CABLE is only a troubleshooting fallback.

To create a Windows build, use a Windows machine or Windows CI with Node.js, Rust, and Microsoft Visual Studio Build Tools installed. Then build with Windows bundle targets:

```powershell
npm ci
npm run release:windows
```

Expected Windows artifacts:

```text
src-tauri\target\release\bundle\nsis\
```

Run the Windows checks without producing an installer with `npm run release:windows:check`. See [the Windows user guide](docs/WINDOWS_TEAMS_USER_GUIDE.md) and [release guide](docs/WINDOWS_RELEASE_GUIDE.md).
