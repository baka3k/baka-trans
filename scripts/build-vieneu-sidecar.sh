#!/usr/bin/env bash
# Build the VieNeu-TTS bridge (PyInstaller one-folder) into
# sidecars/vieneu-tts/bundle/ so Tauri bundles it as a resource.
# macOS counterpart of scripts/build-vieneu-sidecar.ps1 — the bridge must be
# built on the same OS as the desktop release.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SIDECAR_DIR="$ROOT/sidecars/vieneu-tts"
DIST_DIR="$SIDECAR_DIR/dist/vieneu-bridge"
BUNDLE_DIR="$SIDECAR_DIR/bundle"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "Build the VieNeu bridge on the same operating system as the desktop release." >&2
  exit 1
fi
if ! command -v uv >/dev/null 2>&1; then
  echo "The VieNeu bridge build requires uv: https://docs.astral.sh/uv/" >&2
  exit 1
fi

cd "$SIDECAR_DIR"
uv sync --frozen --group build
uv run --frozen --group build pyinstaller --noconfirm --clean bridge.spec

EXECUTABLE="$DIST_DIR/vieneu-bridge"
if [ ! -f "$EXECUTABLE" ]; then
  echo "PyInstaller completed without creating $EXECUTABLE." >&2
  exit 1
fi

mkdir -p "$BUNDLE_DIR"
RESOLVED_BUNDLE="$(cd "$BUNDLE_DIR" && pwd -P)"
RESOLVED_SIDECAR="$(cd "$SIDECAR_DIR" && pwd -P)"
case "$RESOLVED_BUNDLE/" in
  "$RESOLVED_SIDECAR"/*) ;;
  *)
    echo "Refusing to replace a bundle directory outside the VieNeu sidecar workspace." >&2
    exit 1
    ;;
esac
find "$BUNDLE_DIR" -mindepth 1 -maxdepth 1 ! -name '.gitkeep' -exec rm -rf {} +
cp -R "$DIST_DIR/." "$BUNDLE_DIR/"

FILE_COUNT="$(find "$BUNDLE_DIR" -type f ! -name '.gitkeep' | wc -l | tr -d ' ')"
BUNDLE_BYTES="$(find "$BUNDLE_DIR" -type f ! -name '.gitkeep' -exec stat -f '%z' {} + | awk '{ total += $1 } END { print total + 0 }')"
SIZE_MIB="$(awk "BEGIN { printf \"%.1f\", $BUNDLE_BYTES / 1048576 }")"
echo "VieNeu bridge ready: $FILE_COUNT files, $SIZE_MIB MiB"
