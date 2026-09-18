from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from hy_mt_poc import lifecycle
from hy_mt_poc.constants import HY_MT2_SPEC, ModelArtifact, ModelSpec

HY_MT2_MANIFEST_IDENTITY = {
    "modelId": HY_MT2_SPEC.model_id,
    "revision": HY_MT2_SPEC.revision,
}


def synthetic_spec(artifact: ModelArtifact) -> ModelSpec:
    return ModelSpec(
        key="synthetic",
        model_id="example/synthetic",
        revision="a" * 40,
        prompt_style="hy_mt",
        artifacts=(artifact,),
    )


def test_validate_model_rejects_symlink_and_bad_hash(tmp_path: Path) -> None:
    content = b"trusted model input"
    artifact = ModelArtifact("config.json", len(content), hashlib.sha256(content).hexdigest())
    spec = synthetic_spec(artifact)
    model = tmp_path / "active"
    model.mkdir()
    (model / "config.json").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({"modelId": spec.model_id, "revision": spec.revision}),
        encoding="utf-8",
    )
    assert lifecycle.validate_model(model, spec)["modelId"] == spec.model_id
    (model / "config.json").write_bytes(b"tamperd model input")
    with pytest.raises(lifecycle.LifecycleError, match="failed verification"):
        lifecycle.validate_model(model, spec)


def test_validate_model_verifies_git_blob_sha1_when_sha256_is_unpinned(tmp_path: Path) -> None:
    content = b"unpinned but blob-pinned input"
    blob_sha1 = hashlib.sha1(b"blob %d\0" % len(content) + content).hexdigest()
    artifact = ModelArtifact("config.json", len(content), None, blob_sha1)
    spec = synthetic_spec(artifact)
    model = tmp_path / "active"
    model.mkdir()
    (model / "config.json").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({"modelId": spec.model_id, "revision": spec.revision}),
        encoding="utf-8",
    )
    assert lifecycle.validate_model(model, spec)["modelId"] == spec.model_id
    (model / "config.json").write_bytes(b"x" * len(content))
    with pytest.raises(lifecycle.LifecycleError, match="failed verification"):
        lifecycle.validate_model(model, spec)


def test_validate_model_accepts_size_only_weights_without_hashes(tmp_path: Path) -> None:
    content = b"x" * 64
    artifact = ModelArtifact("model.safetensors", len(content))
    spec = synthetic_spec(artifact)
    model = tmp_path / "active"
    model.mkdir()
    (model / "model.safetensors").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({"modelId": spec.model_id, "revision": spec.revision}),
        encoding="utf-8",
    )
    assert lifecycle.validate_model(model, spec)["modelId"] == spec.model_id
    # Size-only artifacts reject any size mismatch; same-size content cannot be
    # detected without a pinned digest.
    (model / "model.safetensors").write_bytes(b"y" * 63)
    with pytest.raises(lifecycle.LifecycleError, match="invalid"):
        lifecycle.validate_model(model, spec)


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


def test_update_reserves_two_model_copies(tmp_path: Path) -> None:
    spec = synthetic_spec(ModelArtifact("model.safetensors", 1000))
    root = tmp_path / "models"
    root.mkdir()
    assert lifecycle.required_free_bytes(root, spec) == 1000 + 512 * 1024 * 1024
    (lifecycle.active_path(root, spec)).mkdir()
    assert lifecycle.required_free_bytes(root, spec) == 2000 + 512 * 1024 * 1024


def test_hy_mt2_model_base_keeps_legacy_layout_and_other_models_nest(tmp_path: Path) -> None:
    hy_mt2_base = lifecycle.model_base(tmp_path, HY_MT2_SPEC)
    assert hy_mt2_base == tmp_path.resolve()
    nested_base = lifecycle.model_base(tmp_path, synthetic_spec(ModelArtifact("m", 1)))
    assert nested_base == (tmp_path / "synthetic").resolve()
    assert lifecycle.active_path(tmp_path, HY_MT2_SPEC) == (tmp_path / "active").resolve()
    assert lifecycle.active_path(tmp_path, synthetic_spec(ModelArtifact("m", 1))) == (
        tmp_path / "synthetic" / "active"
    ).resolve()


def test_hy_mt2_pinned_artifacts_are_fully_hash_pinned() -> None:
    assert HY_MT2_SPEC.total_bytes == 4_086_796_766
    assert all(artifact.sha256 is not None for artifact in HY_MT2_SPEC.artifacts)


def test_translategemma_registry_pins_public_identity() -> None:
    from hy_mt_poc.constants import TRANSLATEGEMMA_4B_SPEC

    assert TRANSLATEGEMMA_4B_SPEC.model_id == "google/translategemma-4b-it"
    assert TRANSLATEGEMMA_4B_SPEC.revision == "10042cb0e6e7fdce748996a71dc3dc432a4e0c89"
    assert TRANSLATEGEMMA_4B_SPEC.total_bytes == 8_639_637_704
    assert TRANSLATEGEMMA_4B_SPEC.prompt_style == "translategemma"
    weights = [
        artifact
        for artifact in TRANSLATEGEMMA_4B_SPEC.artifacts
        if artifact.path.endswith(".safetensors")
    ]
    assert len(weights) == 2
    assert all(artifact.sha256 is None for artifact in weights)
    small_files = [artifact for artifact in TRANSLATEGEMMA_4B_SPEC.artifacts if artifact not in weights]
    assert all(
        artifact.git_blob_sha1 is not None
        for artifact in small_files
        if artifact.path not in {"tokenizer.json", "tokenizer.model"}
    )
