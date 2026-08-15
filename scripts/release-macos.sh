#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="${1:-help}"
VERSION="${2:-}"
ALLOW_UNNOTARIZED=0
RESUME=0

for option in "${@:3}"; do
  case "$option" in
    --allow-unnotarized) ALLOW_UNNOTARIZED=1 ;;
    --resume) RESUME=1 ;;
    *) echo "Unknown option: $option" >&2; exit 1 ;;
  esac
done

usage() {
  cat <<'EOF'
Usage:
  scripts/release-macos.sh check <version> [--allow-unnotarized]
  scripts/release-macos.sh publish <version> [--allow-unnotarized] [--resume]
  scripts/release-macos.sh all <version> [--allow-unnotarized]

Modes:
  check    Validate main, run tests, build, sign, and verify app/DMG artifacts.
  publish  Push main, create/push v<version>, and publish the GitHub Release.
  all      Run check and then publish.

Public releases require Apple notarization credentials by default. Use
--allow-unnotarized only when you intentionally accept Gatekeeper warnings.
EOF
}

fail() {
  echo "release error: $*" >&2
  exit 1
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "missing required command: $1"
}

notarization_configured() {
  if [[ -n "${APPLE_ID:-}" && -n "${APPLE_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
    return 0
  fi
  if [[ -n "${APPLE_API_KEY:-}" && -n "${APPLE_API_ISSUER:-}" && -n "${APPLE_API_KEY_PATH:-}" ]]; then
    return 0
  fi
  return 1
}

ensure_notarization_policy() {
  if notarization_configured; then
    return
  fi
  if [[ "$ALLOW_UNNOTARIZED" -ne 1 ]]; then
    fail "Apple notarization credentials are missing; configure them or pass --allow-unnotarized"
  fi
}

project_version() {
  node -p 'JSON.parse(require("fs").readFileSync("package.json", "utf8")).version'
}

ensure_versions_match() {
  local package_version lock_version tauri_version cargo_version cargo_lock_version
  package_version="$(project_version)"
  lock_version="$(node -p 'JSON.parse(require("fs").readFileSync("package-lock.json", "utf8")).version')"
  tauri_version="$(node -p 'JSON.parse(require("fs").readFileSync("src-tauri/tauri.conf.json", "utf8")).version')"
  cargo_version="$(awk '/^\[package\]/{package=1; next} package && /^version = /{gsub(/version = |"/, ""); print; exit}' src-tauri/Cargo.toml)"
  cargo_lock_version="$(awk '/^name = "baka-trans"$/{getline; gsub(/version = |"/, ""); print; exit}' src-tauri/Cargo.lock)"

  for found in "$package_version" "$lock_version" "$tauri_version" "$cargo_version" "$cargo_lock_version"; do
    [[ "$found" == "$VERSION" ]] || fail "version mismatch: expected $VERSION but found $found"
  done
}

ensure_clean_synced_main() {
  [[ "$(git branch --show-current)" == "main" ]] || fail "release from main only"
  [[ -z "$(git status --porcelain)" ]] || fail "working tree is not clean"
  git fetch --prune origin
  [[ "$(git rev-parse HEAD)" == "$(git rev-parse origin/main)" ]] || fail "main must exactly match origin/main"
}

ensure_signing_identity() {
  local identity identity_line
  identity="${APPLE_SIGNING_IDENTITY:-$(node -p 'JSON.parse(require("fs").readFileSync("src-tauri/tauri.conf.json", "utf8")).bundle?.macOS?.signingIdentity || ""')}"
  [[ -n "$identity" && "$identity" != "-" ]] || fail "configure a stable Apple signing identity"
  identity_line="$(security find-identity -v -p codesigning | grep -F "$identity" | head -1 || true)"
  [[ -n "$identity_line" ]] || fail "signing identity is not available in Keychain: $identity"
  if notarization_configured && [[ "$identity_line" != *"Developer ID Application:"* ]]; then
    fail "notarized distribution requires a Developer ID Application identity"
  fi
}

tag_remote_commit() {
  local tag="$1" commit
  commit="$(git ls-remote origin "refs/tags/${tag}^{}" | awk '{print $1}')"
  if [[ -z "$commit" ]]; then
    commit="$(git ls-remote origin "refs/tags/${tag}" | awk '{print $1}')"
  fi
  echo "$commit"
}

ensure_release_target_available() {
  local tag="v${VERSION}" remote_commit
  remote_commit="$(tag_remote_commit "$tag")"

  if [[ "$RESUME" -eq 1 ]]; then
    [[ -n "$remote_commit" ]] || fail "--resume requires an existing remote tag $tag"
    [[ "$remote_commit" == "$(git rev-parse HEAD)" ]] || fail "$tag does not point to HEAD"
  else
    ! git show-ref --verify --quiet "refs/tags/${tag}" || fail "local tag already exists: $tag"
    [[ -z "$remote_commit" ]] || fail "remote tag already exists: $tag"
  fi

  ! gh release view "$tag" >/dev/null 2>&1 || fail "GitHub Release already exists: $tag"
}

artifact_paths() {
  APP_PATH="$ROOT/src-tauri/target/release/bundle/macos/Baka Trans.app"
  DMG_PATH="$ROOT/src-tauri/target/release/bundle/dmg/Baka Trans_${VERSION}_aarch64.dmg"
}

verify_artifacts() {
  artifact_paths
  [[ -d "$APP_PATH" ]] || fail "missing app bundle: $APP_PATH"
  [[ -f "$DMG_PATH" ]] || fail "missing DMG: $DMG_PATH"
  codesign --verify --deep --strict --verbose=2 "$APP_PATH"
  codesign --verify --verbose=2 "$DMG_PATH"
  hdiutil verify "$DMG_PATH"
  if notarization_configured; then
    xcrun stapler validate "$DMG_PATH"
  fi
}

common_preflight() {
  [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || fail "invalid semantic version: $VERSION"
  for command in git gh node npm cargo codesign hdiutil security shasum; do
    require_command "$command"
  done
  if notarization_configured; then
    require_command xcrun
  fi
  gh auth status
  ensure_clean_synced_main
  ensure_versions_match
  ensure_signing_identity
  ensure_notarization_policy
}

run_check() {
  common_preflight
  [[ "$RESUME" -eq 0 ]] || fail "--resume is only valid with publish"
  ensure_release_target_available

  npm ci
  npm test
  (
    cd src-tauri
    cargo fmt --check
    cargo check
    cargo test
  )
  scripts/build-vieneu-sidecar.sh
  scripts/build-hy-mt-sidecar.sh
  npm run tauri -- build --bundles app,dmg
  verify_artifacts

  echo "DMG SHA-256: $(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
  echo "Release check completed for v${VERSION}."
}

run_publish() {
  local tag previous_tag sha notes release_url
  common_preflight
  ensure_release_target_available
  verify_artifacts

  tag="v${VERSION}"
  sha="$(shasum -a 256 "$DMG_PATH" | awk '{print $1}')"
  previous_tag="$(git tag --sort=-version:refname | grep -E '^v[0-9]' | grep -v "^${tag}$" | head -1 || true)"

  git push origin main
  if [[ "$RESUME" -eq 0 ]]; then
    git tag -a "$tag" -m "Baka Trans ${tag}"
    git push origin "$tag"
  fi

  notes="## macOS download

- Apple Silicon (aarch64) signed DMG
- SHA-256: \`${sha}\`"
  if ! notarization_configured; then
    notes="${notes}

> This build is signed but not notarized. macOS may require approval in Privacy & Security on first launch."
  fi

  release_args=(release create "$tag" "${DMG_PATH}#Baka Trans ${VERSION} — macOS Apple Silicon DMG" --verify-tag --target main --title "Baka Trans ${tag}" --generate-notes --latest --notes "$notes")
  if [[ -n "$previous_tag" ]]; then
    release_args+=(--notes-start-tag "$previous_tag")
  fi

  if ! release_url="$(gh "${release_args[@]}")"; then
    echo "The tag was pushed but GitHub Release creation failed." >&2
    echo "Fix the cause, then rerun: npm run release:publish -- ${VERSION} --resume" >&2
    exit 1
  fi

  gh release view "$tag" --json tagName,url,isDraft,isPrerelease,assets
  echo "Published: $release_url"
}

case "$MODE" in
  check) run_check ;;
  publish) run_publish ;;
  all) run_check; run_publish ;;
  help|-h|--help) usage ;;
  *) usage >&2; exit 1 ;;
esac
