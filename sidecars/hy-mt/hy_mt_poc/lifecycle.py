"""Verified, app-owned HY-MT model installation and activation."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import time
import uuid
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Callable

from .constants import MODEL_ARTIFACTS, MODEL_ID, MODEL_REVISION, RUNTIME_VERSION, TOTAL_MODEL_BYTES, TRUST_REMOTE_CODE

MANIFEST_NAME = "install-manifest.json"
ACTIVE_NAME = "active"
STAGING_NAME = ".staging"


@dataclass(frozen=True)
class Artifact:
    path: str
    size_bytes: int
    sha256: str


ARTIFACTS = tuple(Artifact(*artifact) for artifact in MODEL_ARTIFACTS)


class LifecycleError(RuntimeError):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def _managed_root(root: Path) -> Path:
    candidate = root.expanduser().resolve(strict=False)
    if candidate.exists() and candidate.is_symlink():
        raise LifecycleError("Managed model root cannot be a symbolic link.")
    candidate.mkdir(parents=True, exist_ok=True)
    return candidate.resolve()


def _contained(root: Path, relative: str) -> Path:
    candidate = root / relative
    resolved_parent = candidate.parent.resolve(strict=False)
    if resolved_parent != root and root not in resolved_parent.parents:
        raise LifecycleError("Model artifact escaped its managed directory.")
    return candidate


def _assert_safe_tree(root: Path) -> None:
    if root.is_symlink():
        raise LifecycleError("Managed model directory cannot be a symbolic link.")
    for entry in root.rglob("*"):
        if entry.is_symlink():
            raise LifecycleError("Managed model directory contains a symbolic link.")


def active_path(root: Path) -> Path:
    return _managed_root(root) / ACTIVE_NAME


def required_free_bytes(root: Path) -> int:
    # Updating retains the last verified active model until the new staging copy
    # activates. Reserve both full copies plus a small filesystem overhead.
    active = active_path(root)
    copies = 2 if active.exists() else 1
    return TOTAL_MODEL_BYTES * copies + max(512 * 1024 * 1024, TOTAL_MODEL_BYTES // 10)


def check_free_space(root: Path) -> None:
    available = shutil.disk_usage(_managed_root(root)).free
    required = required_free_bytes(root)
    if available < required:
        raise LifecycleError(f"Insufficient disk space: need {required} bytes, have {available} bytes.")


def validate_model(model_dir: Path) -> dict[str, Any]:
    # Do not resolve the final component before checking it: resolving first
    # would hide an active-model symlink and make it look like a normal dir.
    model_dir = model_dir.expanduser().absolute()
    if not model_dir.is_dir() or model_dir.is_symlink():
        raise LifecycleError("HY-MT model installation is incomplete.")
    _assert_safe_tree(model_dir)
    manifest_path = model_dir / MANIFEST_NAME
    if not manifest_path.is_file() or manifest_path.is_symlink():
        raise LifecycleError("HY-MT model manifest is missing.")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise LifecycleError("HY-MT model manifest is invalid.") from exc
    if manifest.get("modelId") != MODEL_ID or manifest.get("revision") != MODEL_REVISION:
        raise LifecycleError("HY-MT model manifest does not match the pinned model.")
    for artifact in ARTIFACTS:
        path = _contained(model_dir, artifact.path)
        if not path.is_file() or path.is_symlink() or path.stat().st_size != artifact.size_bytes:
            raise LifecycleError(f"HY-MT model artifact is invalid: {artifact.path}")
        if sha256_file(path) != artifact.sha256:
            raise LifecycleError(f"HY-MT model artifact failed verification: {artifact.path}")
    return manifest


def status(root: Path) -> dict[str, Any]:
    try:
        manifest = validate_model(active_path(root))
    except LifecycleError as exc:
        return {"state": "not_installed", "message": str(exc), "totalBytes": TOTAL_MODEL_BYTES}
    return {"state": "installed", "totalBytes": TOTAL_MODEL_BYTES, "manifest": manifest}


def _manifest() -> dict[str, Any]:
    return {
        "modelId": MODEL_ID,
        "revision": MODEL_REVISION,
        "runtimeVersion": RUNTIME_VERSION,
        "trustRemoteCode": TRUST_REMOTE_CODE,
        "verifiedAt": int(time.time()),
        "totalBytes": TOTAL_MODEL_BYTES,
        "artifacts": [asdict(artifact) for artifact in ARTIFACTS],
    }


def install(root: Path, progress: Callable[[dict[str, Any]], None] | None = None) -> dict[str, Any]:
    """Download to a versioned staging directory, verify, then atomically activate."""
    from huggingface_hub import snapshot_download

    managed = _managed_root(root)
    check_free_space(managed)
    staging_parent = managed / STAGING_NAME
    staging_parent.mkdir(exist_ok=True)
    if staging_parent.is_symlink():
        raise LifecycleError("Model staging root cannot be a symbolic link.")
    # A failed download remains in this version-scoped directory. The Hub
    # downloader can resume it, but it can never be selected by serve mode.
    staging = staging_parent / MODEL_REVISION
    if staging.exists() and (not staging.is_dir() or staging.is_symlink()):
        raise LifecycleError("Model staging directory is not safe to resume.")
    staging.mkdir(exist_ok=True)
    if progress:
        progress({"type": "progress", "state": "downloading", "downloadedBytes": 0, "totalBytes": TOTAL_MODEL_BYTES})
    try:
        snapshot_download(
            MODEL_ID,
            revision=MODEL_REVISION,
            local_dir=staging,
            allow_patterns=[artifact.path for artifact in ARTIFACTS],
            max_workers=2,
        )
        # Hub bookkeeping is not an executable model input and must not become
        # part of the trusted active tree.
        cache = staging / ".cache"
        if cache.exists():
            shutil.rmtree(cache)
        _assert_safe_tree(staging)
        manifest_path = staging / MANIFEST_NAME
        manifest_path.write_text(json.dumps(_manifest(), ensure_ascii=False, sort_keys=True), encoding="utf-8")
        validate_model(staging)
        if progress:
            progress({"type": "progress", "state": "verifying", "downloadedBytes": TOTAL_MODEL_BYTES, "totalBytes": TOTAL_MODEL_BYTES})
        active = managed / ACTIVE_NAME
        backup = managed / f".previous-{MODEL_REVISION}-{uuid.uuid4().hex}"
        if active.exists():
            os.replace(active, backup)
        try:
            os.replace(staging, active)
        except OSError:
            if backup.exists() and not active.exists():
                os.replace(backup, active)
            raise
        if backup.exists():
            shutil.rmtree(backup)
        manifest = validate_model(active)
        if progress:
            progress({"type": "complete", "state": "installed", "downloadedBytes": TOTAL_MODEL_BYTES, "totalBytes": TOTAL_MODEL_BYTES})
        return manifest
    except Exception:
        # Keep failed staging data for an explicit repair/resume attempt; it is
        # never active or loadable by serve mode.
        raise
