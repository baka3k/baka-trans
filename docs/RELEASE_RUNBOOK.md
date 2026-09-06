## 1. Overview

| Platform | Main Command | Artifact | Auto-Publish? |
| --- | --- | --- | --- |
| macOS (Apple Silicon) | `npm run release:check` → `npm run release:publish` | `.app` + `.dmg` | Yes (tag + GitHub Release + upload DMG) |
| Windows | `npm run release:windows` | `.exe` (NSIS) + `.sha256` | No (manual upload or retrieved from CI) |
| CI (every push/PR) | [.github/workflows/desktop.yml](https://www.google.com/search?q=../.github/workflows/desktop.yml) | NSIS `.exe` as artifact | No (build check only) |

**Golden Rule:** Always build on the OS you intend to ship. macOS builds on Apple Silicon; Windows builds on Windows (the script automatically blocks if the platform is incorrect).

The version must match across all 5 places (the `npm run version:set` command updates all of them):

1. `package.json`
2. `package-lock.json`
3. `src-tauri/tauri.conf.json`
4. `src-tauri/Cargo.toml`
5. `src-tauri/Cargo.lock` (`baka-trans` entry)

---

## 2. Prerequisites (One-time Setup)

```bash
# Node 22+, npm, Rust/Cargo + Tauri prerequisites
# GitHub CLI with repo write permissions:
gh auth login && gh auth status

```

**macOS — Code Signing & Notarization (Optional):**

If you **do not** have a Developer ID certificate / do not need code signing, use the `--allow-unsigned` flag for all release commands (see Step 3). Note: Unsigned DMGs will be blocked by Gatekeeper on the downloader's machine—users must right-click → Open, or run `xattr -cr "/Applications/Baka Trans.app"` on first launch.

If you have a certificate, verify it:

```bash
security find-identity -v -p codesigning    # Requires type "Developer ID Application"

```

Notarization — choose ONE of the two credential sets (env vars, do not commit):

```bash
# Method 1: Apple ID
export APPLE_ID="your-apple-id@example.com"
export APPLE_PASSWORD="app-specific-password"    # Generated at appleid.apple.com
export APPLE_TEAM_ID="TEAMID"

# Method 2: App Store Connect API
export APPLE_API_KEY="KEYID"
export APPLE_API_ISSUER="ISSUER_UUID"
export APPLE_API_KEY_PATH="/absolute/path/AuthKey_KEYID.p8"

```

Override the signing identity when needed (do not modify the repo): `export APPLE_SIGNING_IDENTITY="Developer ID Application: ..."`.

---

## 3. macOS Workflow (Standard)

### Step 0 — Pre-flight

```bash
git switch main
git pull --ff-only origin main
npm ci
git status          # Must be completely clean — publish will abort otherwise

```

### Step 1 — Bump Version (Skip if version is already bumped)

> `version:set` automatically blocks if you are not on the `main` branch or if the working tree has uncommitted changes.

```bash
npm run version:set -- 0.4.0
git diff                                    # Review all 5 sources
git add package.json package-lock.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: prepare v0.4.0"
git push origin main

```

### Step 2 — Build + Verify (Nothing published yet; safe to retry)

```bash
npm run release:check -- 0.4.0

```

The script automatically: runs tests → `tauri build` → verifies `.app`/`.dmg` signatures → verifies notarization tickets.

### Step 3 — Publish (Irreversible)

```bash
npm run release:publish -- 0.4.0

```

The script will **automatically abort** if: the working tree is unclean · `main` ≠ `origin/main` · the 5 version sources do not match · the tag/GitHub Release already exists · notarization credentials are missing (unless `--allow-unnotarized` is passed).

When successful, it: pushes `main` (no force) → creates + pushes tag `v0.4.0` → creates a GitHub Release with notes generated from the previous tag → uploads the DMG along with SHA-256 hashes in the release notes → marks as **Latest**.

> **Single command** (combining steps 2 & 3): `npm run release:macos -- all 0.4.0`
> **No signing certificate:** Add `--allow-unsigned` (which implicitly includes `--allow-unnotarized`):
> ```bash
> npm run release:macos -- all 0.4.0 --allow-unsigned
> 
> ```
> 
> 
> The DMG will not be signed—Gatekeeper will block first-time downloads; release notes will include instructions for `xattr -cr`.
> **Internal release with signing but skipped notarization:** Add `--allow-unnotarized`. Users will see Gatekeeper warnings—do not instruct users to disable Gatekeeper globally.

### Step 4 — Post-Release Verification

```bash
shasum -a 256 "src-tauri/target/release/bundle/dmg/Baka Trans_0.4.0_aarch64.dmg"    # Compare with SHA-256 in release notes

```

Note: GitHub converts spaces in file names to periods—`Baka Trans_0.4.0_aarch64.dmg` becomes `Baka.Trans_0.4.0_aarch64.dmg` in download URLs.

### Troubleshooting

| Situation | Resolution |
| --- | --- |
| `configure a stable Apple signing identity` | No certificate available/needed: add `--allow-unsigned` to the release command (see Step 3) |
| Tag pushed but Release creation/asset upload failed | Fix external issues (network, `gh` permissions) then run `npm run release:publish -- 0.4.0 --resume` (keep the same `--allow-*` flags used initially) |
| Release exists but assets are missing | `gh release upload v0.4.0 "src-tauri/target/release/bundle/dmg/Baka Trans_0.4.0_aarch64.dmg#Baka Trans 0.4.0 — macOS Apple Silicon DMG"` |
| Incorrect version / need re-release | Scripts **never** delete or force-move tags. Only delete releases if intentionally rolling back; never delete a published tag |

---

## 4. Windows Workflow

### Local Build (On a Windows Machine)

```powershell
git switch main
git pull --ff-only origin main
npm ci

npm run release:windows         # Test (npm build/test + cargo test) → build VieNeu & Hy-MT sidecars → tauri build nsis → generate .sha256 file
npm run release:windows:check   # Run checks only, do not build

```

Artifact: `src-tauri\target\release\bundle\nsis\*.exe` (with accompanying `.sha256`).

### Manual Windows Publishing

Windows does not yet have an automated publish script like macOS:

```bash
gh release create v0.4.0 "src-tauri/target/release/bundle/nsis/Baka.Trans_0.4.0_x64-setup.exe" --notes "..."

```

(Include the matching `.sha256` file if desired). You can publish macOS first and then upload Windows assets to the same release: `gh release upload v0.4.0 <file-exe>`.

### Fetching Builds from CI (Alternative to Local Builds)

On every push/PR, [desktop.yml](https://www.google.com/search?q=../.github/workflows/desktop.yml) automatically builds NSIS on `windows-latest` and uploads the `baka-trans-windows-nsis` artifact. Suitable for internal test builds; official releases should still be built locally for signing/notarization (if configured).

---

## 5. Quick Pre-Publish Checklist

* [ ] Working tree is clean, `main` = `origin/main`
* [ ] `npm ci` has run on the build machine
* [ ] Version matches across all 5 sources (open `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` to verify—the publish script will also re-verify)
* [ ] macOS: Either Developer ID Application certificate + notarization env vars are present, or `--allow-unsigned` will be used
* [ ] `npm run release:check` passed successfully beforehand
* [ ] Tag `v0.4.0` does not yet exist (locally or remotely)
* [ ] `gh auth status` is OK

---

## 6. Artifact Location Summary

```text
macOS:    src-tauri/target/release/bundle/macos/Baka Trans.app
          src-tauri/target/release/bundle/dmg/Baka Trans_<version>_aarch64.dmg
Windows:  src-tauri/target/release/bundle/nsis/*.exe (+ .sha256)

```
