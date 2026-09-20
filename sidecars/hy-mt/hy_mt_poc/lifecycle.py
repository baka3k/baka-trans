"""Verified, app-owned offline translation model installation and activation."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import time
import uuid
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from .constants import HY_MT2_SPEC, RUNTIME_VERSION, TRUST_REMOTE_CODE, ModelArtifact, ModelSpec

MANIFEST_NAME = "install-manifest.json"
ACTIVE_NAME = "active"
STAGING_NAME = ".staging"


@dataclass(frozen=True)
class Artifact:
    path: str
    size_bytes: int
    sha256: str | None = None
    git_blob_sha1: str | None = None


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


def model_base(root: Path, spec: ModelSpec) -> Path:
    """Per-model directory under the shared cache root.

    The host passes ``root`` as the per-model directory (the Hy-MT2 cache
    root or the nested directory named after ``spec.key`` for later
    models). The Hy-MT2 model keeps the original unscoped layout so
    existing verified installs stay valid without migration.
    """
    if spec.key == HY_MT2_SPEC.key:
        return _managed_root(root)
    # Newer models: the host already passes the per-model directory, so we
    # must not append ``spec.key`` again or we would create a doubly-nested
    # layout (``translategemma-4b/translategemma-4b/...``) that nothing
    # else can locate.
    base = root.expanduser().absolute()
    if base.exists() and base.is_symlink():
        raise LifecycleError("Managed model directory cannot be a symbolic link.")
    base.mkdir(parents=True, exist_ok=True)
    return base


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


def active_path(root: Path, spec: ModelSpec = HY_MT2_SPEC) -> Path:
    return model_base(root, spec) / ACTIVE_NAME


def required_free_bytes(root: Path, spec: ModelSpec = HY_MT2_SPEC) -> int:
    # Updating retains the last verified active model until the new staging copy
    # activates. Reserve both full copies plus a small filesystem overhead.
    active = active_path(root, spec)
    copies = 2 if active.exists() else 1
    return spec.total_bytes * copies + max(512 * 1024 * 1024, spec.total_bytes // 10)


def check_free_space(root: Path, spec: ModelSpec) -> None:
    available = shutil.disk_usage(model_base(root, spec)).free
    required = required_free_bytes(root, spec)
    if available < required:
        raise LifecycleError(f"Insufficient disk space: need {required} bytes, have {available} bytes.")


def _verify_artifact(path: Path, artifact: ModelArtifact) -> None:
    if not path.is_file() or path.is_symlink() or path.stat().st_size != artifact.size_bytes:
        raise LifecycleError(f"Model artifact is invalid: {artifact.path}")
    if artifact.sha256 is not None:
        if sha256_file(path) != artifact.sha256:
            raise LifecycleError(f"Model artifact failed verification: {artifact.path}")
    elif artifact.git_blob_sha1 is not None:
        digest = hashlib.sha1()
        digest.update(b"blob %d\0" % artifact.size_bytes)
        with path.open("rb") as stream:
            while chunk := stream.read(1024 * 1024):
                digest.update(chunk)
        if digest.hexdigest() != artifact.git_blob_sha1:
            raise LifecycleError(f"Model artifact failed verification: {artifact.path}")


def validate_model(model_dir: Path, spec: ModelSpec = HY_MT2_SPEC) -> dict[str, Any]:
    # Do not resolve the final component before checking it: resolving first
    # would hide an active-model symlink and make it look like a normal dir.
    model_dir = model_dir.expanduser().absolute()
    if not model_dir.is_dir() or model_dir.is_symlink():
        raise LifecycleError("Offline model installation is incomplete.")
    _assert_safe_tree(model_dir)
    manifest_path = model_dir / MANIFEST_NAME
    if not manifest_path.is_file() or manifest_path.is_symlink():
        raise LifecycleError("Offline model manifest is missing.")
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise LifecycleError("Offline model manifest is invalid.") from exc
    if manifest.get("modelId") != spec.model_id or manifest.get("revision") != spec.revision:
        raise LifecycleError("Model manifest does not match the pinned model.")
    for artifact in spec.artifacts:
        _verify_artifact(_contained(model_dir, artifact.path), artifact)
    return manifest


def status(root: Path, spec: ModelSpec = HY_MT2_SPEC) -> dict[str, Any]:
    try:
        manifest = validate_model(active_path(root, spec), spec)
    except LifecycleError as exc:
        return {"state": "not_installed", "message": str(exc), "totalBytes": spec.total_bytes}
    return {"state": "installed", "totalBytes": spec.total_bytes, "manifest": manifest}


def _manifest(spec: ModelSpec) -> dict[str, Any]:
    return {
        "modelKey": spec.key,
        "modelId": spec.model_id,
        "revision": spec.revision,
        "runtimeVersion": RUNTIME_VERSION,
        "trustRemoteCode": TRUST_REMOTE_CODE,
        "verifiedAt": int(time.time()),
        "totalBytes": spec.total_bytes,
        "artifacts": [
            {"path": artifact.path, "size_bytes": artifact.size_bytes, "sha256": artifact.sha256}
            for artifact in spec.artifacts
        ],
    }


def install(root: Path, progress: Callable[[dict[str, Any]], None] | None = None, spec: ModelSpec = HY_MT2_SPEC) -> dict[str, Any]:
    """Download to a versioned staging directory, verify, then atomically activate."""
    # The Hub client snapshots offline mode from the environment at import
    # time; clear it before the lazy import below so the pinned download can
    # reach the network no matter which process invoked the installer. A
    # Hugging Face token (needed for gated repositories) is consumed from the
    # environment by the Hub client and is never accepted in serve mode.
    for name in ("HF_HUB_OFFLINE", "TRANSFORMERS_OFFLINE"):
        os.environ.pop(name, None)
    from huggingface_hub import snapshot_download

    managed = model_base(root, spec)
    check_free_space(root, spec)
    staging_parent = managed / STAGING_NAME
    staging_parent.mkdir(exist_ok=True)
    if staging_parent.is_symlink():
        raise LifecycleError("Model staging root cannot be a symbolic link.")
    # A failed download remains in this version-scoped directory. The Hub
    # downloader can resume it, but it can never be selected by serve mode.
    staging = staging_parent / spec.revision
    if staging.exists() and (not staging.is_dir() or staging.is_symlink()):
        raise LifecycleError("Model staging directory is not safe to resume.")
    staging.mkdir(exist_ok=True)
    if progress:
        progress({"type": "progress", "state": "downloading", "downloadedBytes": 0, "totalBytes": spec.total_bytes})
    try:
        snapshot_download(
            spec.model_id,
            revision=spec.revision,
            local_dir=staging,
            allow_patterns=list(spec.paths),
            max_workers=2,
        )
        # Hub bookkeeping is not an executable model input and must not become
        # part of the trusted active tree.
        cache = staging / ".cache"
        if cache.exists():
            shutil.rmtree(cache)
        _assert_safe_tree(staging)
        manifest_path = staging / MANIFEST_NAME
        manifest_path.write_text(json.dumps(_manifest(spec), ensure_ascii=False, sort_keys=True), encoding="utf-8")
        validate_model(staging, spec)
        if progress:
            progress({"type": "progress", "state": "verifying", "downloadedBytes": spec.total_bytes, "totalBytes": spec.total_bytes})
        active = managed / ACTIVE_NAME
        backup = managed / f".previous-{spec.revision}-{uuid.uuid4().hex}"
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
        manifest = validate_model(active, spec)
        if progress:
            progress({"type": "complete", "state": "installed", "downloadedBytes": spec.total_bytes, "totalBytes": spec.total_bytes})
        return manifest
    except Exception:
        # Keep failed staging data for an explicit repair/resume attempt; it is
        # never active or loadable by serve mode.
        raise
