# macOS Release Guide

This guide describes how Baka Trans creates a versioned macOS Apple Silicon release and uploads its DMG to GitHub Releases.

## How v0.1.0 was published

GitHub did not build the installer. Tauri created the files locally:

```text
src-tauri/target/release/bundle/macos/Baka Trans.app
src-tauri/target/release/bundle/dmg/Baka Trans_0.1.0_aarch64.dmg
```

The release process then:

1. Ran frontend and Rust tests.
2. Built the signed `.app` and `.dmg` with `tauri build`.
3. Verified the code signatures and DMG checksum.
4. Fast-forwarded and pushed `main`.
5. Created and pushed annotated tag `v0.1.0`.
6. Used `gh release create` to create the GitHub Release and upload the local DMG as its asset.

The GitHub asset URL is generated after upload. GitHub changes spaces in the uploaded filename to dots, which is why the local `Baka Trans_0.1.0_aarch64.dmg` became `Baka.Trans_0.1.0_aarch64.dmg` in the download URL.

## Prerequisites

- macOS on Apple Silicon.
- Node.js, npm, Rust, Cargo, and the Tauri prerequisites.
- GitHub CLI authenticated with repository write access:

  ```bash
  gh auth login
  gh auth status
  ```

- A stable Apple signing identity available in Keychain:

  ```bash
  security find-identity -v -p codesigning
  ```

  `src-tauri/tauri.conf.json` contains the default development identity. Override it without editing the repository when needed:

  ```bash
  export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"
  ```

## Notarization

Public releases should be notarized. The release script requires notarization credentials unless `--allow-unnotarized` is explicitly supplied.

Use either Apple ID credentials:

```bash
export APPLE_ID="your-apple-id@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"
```

Or App Store Connect API credentials:

```bash
export APPLE_API_KEY="KEYID"
export APPLE_API_ISSUER="ISSUER_UUID"
export APPLE_API_KEY_PATH="/absolute/path/to/AuthKey_KEYID.p8"
```

Never commit these credentials. When they are configured, Tauri notarizes the bundle and the script verifies the stapled ticket.
The script also requires the selected signing certificate to be a `Developer ID Application` identity for notarized distribution; the development identity in the repository is only suitable for local or explicitly unnotarized builds.

## Prepare a version

Start from a clean, synchronized `main` branch:

```bash
git switch main
git pull --ff-only origin main
npm ci
```

Update every version source together:

```bash
npm run version:set -- 0.2.0
```

This updates:

- `package.json`
- `package-lock.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- the Baka Trans package entry in `src-tauri/Cargo.lock`

Review and commit the version change before releasing:

```bash
git diff --check
git diff
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: prepare v0.2.0"
git push origin main
```

## Recommended two-step release

The safe workflow separates reversible verification from irreversible publishing.

Build and verify without creating a tag or GitHub Release:

```bash
npm run release:check -- 0.2.0
```

After reviewing the output, publish the already verified artifact:

```bash
npm run release:publish -- 0.2.0
```

The publish step:

- confirms the working tree is clean and `main` equals `origin/main`;
- confirms all five version sources match;
- re-verifies the app, DMG, signing, and notarization ticket;
- refuses to overwrite an existing tag or GitHub Release;
- pushes `main` without force;
- creates and pushes annotated tag `v<version>`;
- generates release notes from the previous version tag;
- uploads the DMG with its SHA-256 in the release notes;
- marks the release as Latest.

## One-command release

To run verification and publishing in one command:

```bash
npm run release:macos -- all 0.2.0
```

For an intentional signed-but-unnotarized internal release:

```bash
npm run release:macos -- all 0.2.0 --allow-unnotarized
```

Unnotarized public downloads may trigger Gatekeeper warnings. Do not instruct users to disable Gatekeeper globally.

## Generated artifacts

For version `0.2.0`, the scripts expect:

```text
src-tauri/target/release/bundle/macos/Baka Trans.app
src-tauri/target/release/bundle/dmg/Baka Trans_0.2.0_aarch64.dmg
```

The DMG hash can be checked manually with:

```bash
shasum -a 256 "src-tauri/target/release/bundle/dmg/Baka Trans_0.2.0_aarch64.dmg"
```

## Recovery

The script never deletes or force-moves a tag. If the tag was pushed but GitHub Release creation or asset upload failed, fix the external issue and resume publishing:

```bash
npm run release:publish -- 0.2.0 --resume
```

Add `--allow-unnotarized` again if that release intentionally has no notarization credentials.

If a GitHub Release already exists but an asset is missing, upload only the asset:

```bash
gh release upload v0.2.0 \
  "src-tauri/target/release/bundle/dmg/Baka Trans_0.2.0_aarch64.dmg#Baka Trans 0.2.0 — macOS Apple Silicon DMG"
```

Do not delete and recreate a published tag unless the release itself is being intentionally withdrawn.
