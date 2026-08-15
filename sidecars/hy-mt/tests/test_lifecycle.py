from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from hy_mt_poc import lifecycle


def test_validate_model_rejects_symlink_and_bad_hash(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    content = b"trusted model input"
    artifact = lifecycle.Artifact("config.json", len(content), hashlib.sha256(content).hexdigest())
    monkeypatch.setattr(lifecycle, "ARTIFACTS", (artifact,))
    model = tmp_path / "active"
    model.mkdir()
    (model / "config.json").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({"modelId": "tencent/HY-MT1.5-1.8B", "revision": "172d98efc7f534e05c86d3d49ed9d12d9c2a733b"}),
        encoding="utf-8",
    )
    assert lifecycle.validate_model(model)["modelId"] == "tencent/HY-MT1.5-1.8B"
    (model / "config.json").write_bytes(b"tamperd model input")
    with pytest.raises(lifecycle.LifecycleError, match="failed verification"):
        lifecycle.validate_model(model)


def test_staging_or_active_symlinks_are_never_trusted(tmp_path: Path) -> None:
    external = tmp_path / "external"
    external.mkdir()
    root = tmp_path / "models"
    root.mkdir()
    try:
        (root / "active").symlink_to(external, target_is_directory=True)
    except OSError:
        pytest.skip("symlinks unavailable in this environment")
    with pytest.raises(lifecycle.LifecycleError):
        lifecycle.validate_model(root / "active")


def test_update_reserves_two_model_copies(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    monkeypatch.setattr(lifecycle, "TOTAL_MODEL_BYTES", 1000)
    root = tmp_path / "models"
    root.mkdir()
    assert lifecycle.required_free_bytes(root) == 1000 + 512 * 1024 * 1024
    (root / "active").mkdir()
    assert lifecycle.required_free_bytes(root) == 2000 + 512 * 1024 * 1024
