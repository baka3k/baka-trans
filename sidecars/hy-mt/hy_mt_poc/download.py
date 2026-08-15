"""Download the pinned candidate to a POC-owned directory and hash it."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from huggingface_hub import snapshot_download

from .constants import MODEL_ID, MODEL_REVISION, REPOSITORY_FILES
from .evidence import sha256_file


def download_model(model_dir: Path) -> dict[str, Any]:
    model_dir = model_dir.resolve()
    model_dir.mkdir(parents=True, exist_ok=True)
    snapshot_download(
        repo_id=MODEL_ID,
        revision=MODEL_REVISION,
        local_dir=model_dir,
        allow_patterns=list(REPOSITORY_FILES),
    )

    artifacts = []
    for relative in REPOSITORY_FILES:
        path = model_dir / relative
        if not path.is_file():
            raise RuntimeError(f"pinned model artifact is missing: {relative}")
        if path.is_symlink():
            raise RuntimeError(f"pinned model artifact must not be a symlink: {relative}")
        artifacts.append(
            {
                "path": relative,
                "sizeBytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )

    return {
        "modelId": MODEL_ID,
        "revision": MODEL_REVISION,
        "directory": str(model_dir),
        "totalBytes": sum(item["sizeBytes"] for item in artifacts),
        "artifacts": artifacts,
    }
