# macOS Release Automation — 2026-07-10

## Context

The v0.1.0 DMG was built locally by Tauri and uploaded with GitHub CLI, so the repeatable process and its irreversible steps needed to be captured in the repository (`docs/RELEASE_GUIDE.md:5`).

## Change

- Added npm entry points for synchronized version updates, release checks, publishing, and the combined flow (`package.json:12`).
- Added a version updater that requires a clean `main` and writes the same version to the Node, Tauri, and Cargo sources (`scripts/set-version.mjs:13`, `scripts/set-version.mjs:49`).
- Added a macOS release script with separate `check`, `publish`, and `all` modes (`scripts/release-macos.sh:20`). `check` installs dependencies, runs frontend/Rust validation, builds the app and DMG, verifies signatures, and prints the SHA-256 (`scripts/release-macos.sh:156`). `publish` re-verifies artifacts, pushes `main`, creates the annotated tag, and uploads the local DMG through `gh release create` (`scripts/release-macos.sh:176`).
- Added an operator guide for version preparation, the recommended two-step workflow, generated artifacts, and recovery (`docs/RELEASE_GUIDE.md:71`, `docs/RELEASE_GUIDE.md:105`, `docs/RELEASE_GUIDE.md:149`, `docs/RELEASE_GUIDE.md:164`).

## Impact

**Risk level: medium.** Maintainers now have a consistent Apple Silicon release path with preflight checks for branch state, synchronized versions, signing, artifacts, and existing tags/releases (`scripts/release-macos.sh:69`, `scripts/release-macos.sh:82`, `scripts/release-macos.sh:109`). The remaining risk is external Apple/GitHub failure after a tag is pushed; the recovery path avoids deleting or force-moving release tags.

## Decision

Use `release:check` before `release:publish` so tests, build, signing, and artifact verification remain reviewable before any tag or GitHub Release is created (`docs/RELEASE_GUIDE.md:105`). Public releases require notarization credentials by default; `--allow-unnotarized` is an explicit exception for intentionally signed-but-unnotarized builds that may trigger Gatekeeper warnings (`scripts/release-macos.sh:46`, `scripts/release-macos.sh:56`, `docs/RELEASE_GUIDE.md:48`). If publishing fails after the tag reaches GitHub, `--resume` requires that remote tag to exist and still point to `HEAD`, then retries Release creation without moving the tag (`scripts/release-macos.sh:109`, `scripts/release-macos.sh:186`, `docs/RELEASE_GUIDE.md:164`).

## References

- commit: d7bfc76835df01ef611e7b6c231e33956f8eb12f
- `scripts/release-macos.sh:20`
- `scripts/set-version.mjs:49`
- `docs/RELEASE_GUIDE.md:105`
- `package.json:12`
