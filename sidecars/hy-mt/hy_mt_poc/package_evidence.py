"""Measure the generated one-folder bundle without changing it."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path
from typing import Any

from .evidence import sha256_file, write_json


def command(command: list[str]) -> dict[str, Any]:
    completed = subprocess.run(command, check=False, capture_output=True, text=True)
    return {
        "command": command,
        "exitCode": completed.returncode,
        "stdout": completed.stdout.strip(),
        "stderr": completed.stderr.strip(),
    }


def measure(bundle_dir: Path) -> dict[str, Any]:
    bundle_dir = bundle_dir.resolve()
    executable = bundle_dir / "hy-mt-poc"
    files = [path for path in bundle_dir.rglob("*") if path.is_file()]
    native = [
        path
        for path in files
        if path.suffix in {".dylib", ".so"} or path == executable
    ]
    signature_failures = []
    for path in native:
        result = subprocess.run(
            ["codesign", "--verify", "--strict", str(path)],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode:
            signature_failures.append(
                {"path": str(path.relative_to(bundle_dir)), "error": result.stderr.strip()}
            )
    largest = sorted(files, key=lambda path: path.stat().st_size, reverse=True)[:20]
    return {
        "bundleDirectory": str(bundle_dir),
        "bundleBytes": sum(path.stat().st_size for path in files),
        "fileCount": len(files),
        "nativeFileCount": len(native),
        "nativeSignatureFailures": signature_failures,
        "allNativeFilesAdHocOrSigned": not signature_failures,
        "executableSha256": sha256_file(executable),
        "executableFile": command(["file", str(executable)]),
        "executableSignature": command(["codesign", "-dvvv", str(executable)]),
        "deepSignatureVerification": command(
            ["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(executable)]
        ),
        "linkedLibraries": command(["otool", "-L", str(executable)]),
        "largestFiles": [
            {"path": str(path.relative_to(bundle_dir)), "sizeBytes": path.stat().st_size}
            for path in largest
        ],
        "codeSigningImplication": (
            "PyInstaller applied ad-hoc signatures. Phase 14 must replace them with the "
            "application signing identity, preserve hardened-runtime entitlements, verify "
            "every nested Mach-O file, and notarize the containing application."
        ),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    payload = measure(args.bundle_dir)
    write_json(args.output, payload)
    print(f"measured {payload['fileCount']} files and {payload['bundleBytes']} bytes")


if __name__ == "__main__":
    main()
