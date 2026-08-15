"""Small helpers for reproducible JSON/CSV evidence."""

from __future__ import annotations

import csv
import hashlib
import json
import platform
import shutil
import socket
import subprocess
import sys
from importlib import metadata
from pathlib import Path
from typing import Any, Iterable

import psutil

from .constants import MODEL_ID, MODEL_REVISION


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(4 * 1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(sanitize_evidence(payload), ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )


def sanitize_evidence(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: sanitize_evidence(item) for key, item in value.items()}
    if isinstance(value, list):
        return [sanitize_evidence(item) for item in value]
    if isinstance(value, tuple):
        return [sanitize_evidence(item) for item in value]
    if isinstance(value, str):
        home = str(Path.home())
        hostname = socket.gethostname()
        normalized = value.replace(home, "$HOME") if home else value
        return normalized.replace(hostname, "$HOST") if hostname else normalized
    return value


def write_csv(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    materialized = list(rows)
    path.parent.mkdir(parents=True, exist_ok=True)
    if not materialized:
        path.write_text("", encoding="utf-8")
        return
    fieldnames: list[str] = []
    for row in materialized:
        for key in row:
            if key not in fieldnames:
                fieldnames.append(key)
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fieldnames, lineterminator="\n")
        writer.writeheader()
        writer.writerows(materialized)


def runtime_manifest() -> dict[str, Any]:
    packages = {}
    for package in (
        "huggingface-hub",
        "psutil",
        "safetensors",
        "torch",
        "transformers",
        "pyinstaller",
    ):
        try:
            packages[package] = metadata.version(package)
        except metadata.PackageNotFoundError:
            packages[package] = None
    return {
        "modelId": MODEL_ID,
        "modelRevision": MODEL_REVISION,
        "python": sys.version,
        "pythonExecutable": sys.executable,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "packages": packages,
    }


def environment_manifest() -> dict[str, Any]:
    resident = []
    for process in psutil.process_iter(["pid", "name"]):
        try:
            name = (process.info["name"] or "").lower()
        except (psutil.AccessDenied, psutil.NoSuchProcess):
            continue
        if any(keyword in name for keyword in ("ollama", "whisper", "vieneu")):
            resident.append(process.info)
    return {
        "runtime": runtime_manifest(),
        "uname": command_output(["uname", "-a"]),
        "macOS": command_output(["sw_vers"]),
        "cpu": command_output(["sysctl", "-n", "machdep.cpu.brand_string"]),
        "physicalMemoryBytes": command_output(["sysctl", "-n", "hw.memsize"]),
        "memoryPressure": command_output(["memory_pressure", "-Q"]),
        "ollamaExecutable": shutil.which("ollama"),
        "residentCandidateRuntimes": resident,
    }


def memory_snapshot(torch_module: Any | None = None) -> dict[str, Any]:
    process = psutil.Process()
    virtual = psutil.virtual_memory()
    result: dict[str, Any] = {
        "rssBytes": process.memory_info().rss,
        "systemTotalBytes": virtual.total,
        "systemAvailableBytes": virtual.available,
        "systemUsedPercent": virtual.percent,
    }
    if torch_module is not None and torch_module.backends.mps.is_available():
        result["mpsCurrentAllocatedBytes"] = torch_module.mps.current_allocated_memory()
        result["mpsDriverAllocatedBytes"] = torch_module.mps.driver_allocated_memory()
        recommended = getattr(torch_module.mps, "recommended_max_memory", None)
        if recommended is not None:
            result["mpsRecommendedMaxBytes"] = recommended()
    return result


def command_output(command: list[str]) -> str | None:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired):
        return None
    output = (completed.stdout or completed.stderr).strip()
    return output or None
