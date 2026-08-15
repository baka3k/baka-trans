# Sidecar-specific setup

The Python package manifest requires Python 3.10+ and pins VieNeu. Windows installer builds invoke `scripts/build-vieneu-sidecar.ps1` before Tauri packaging. End users do not need to start the bridge manually; the desktop app installs, verifies, and supervises it.
