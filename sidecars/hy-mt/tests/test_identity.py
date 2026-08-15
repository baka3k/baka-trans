"""Phase 16.1: runtime identity, trust_remote_code audit, and wrong-identity rejection."""

from __future__ import annotations

import hashlib
import io
import json
from pathlib import Path

import pytest

from hy_mt_poc import constants, lifecycle
from hy_mt_poc.server import ServeLoop


class FakeRunner:
    def metadata(self):
        return {"actualDevice": "mps:0", "actualDtype": "bfloat16", "modelLoadMs": 1.0}

    def translate(self, text, **kwargs):
        from hy_mt_poc.runner import TranslationResult

        return TranslationResult("xin chào", 1, 2, 10.0, 200.0, "greedy", False, {})


def frames(output: io.StringIO) -> list[dict[str, object]]:
    return [json.loads(line) for line in output.getvalue().splitlines()]


def test_ready_message_declares_exact_pinned_identity() -> None:
    output = io.StringIO()
    loop = ServeLoop(FakeRunner(), output)
    loop.ready()
    ready = frames(output)[0]
    assert ready["type"] == "ready"
    assert ready["modelId"] == constants.MODEL_ID
    assert ready["revision"] == constants.MODEL_REVISION
    assert ready["protocolVersion"] == constants.PROTOCOL_VERSION
    assert ready["runtimeVersion"] == constants.RUNTIME_VERSION
    assert ready["trustRemoteCode"] is False


def test_runtime_identity_constant_matches_pinned_model() -> None:
    assert constants.RUNTIME_IDENTITY["modelId"] == "tencent/Hy-MT2-1.8B"
    assert constants.RUNTIME_IDENTITY["revision"] == "9a341cd1b679d3efd23b46e847b01745a71ed792"
    assert constants.RUNTIME_IDENTITY["trustRemoteCode"] is False
    assert constants.RUNTIME_IDENTITY["protocolVersion"] == constants.PROTOCOL_VERSION


def test_install_manifest_declares_trust_remote_code_false(monkeypatch: pytest.MonkeyPatch) -> None:
    manifest = lifecycle._manifest()
    assert manifest["trustRemoteCode"] is False
    assert manifest["modelId"] == constants.MODEL_ID
    assert manifest["revision"] == constants.MODEL_REVISION


def test_validate_model_rejects_wrong_model_id(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    content = b"trusted model input"
    artifact = lifecycle.Artifact("config.json", len(content), hashlib.sha256(content).hexdigest())
    monkeypatch.setattr(lifecycle, "ARTIFACTS", (artifact,))
    model = tmp_path / "active"
    model.mkdir()
    (model / "config.json").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({
            "modelId": "wrong-org/wrong-model",
            "revision": constants.MODEL_REVISION,
        }),
        encoding="utf-8",
    )
    with pytest.raises(lifecycle.LifecycleError, match="does not match"):
        lifecycle.validate_model(model)


def test_validate_model_rejects_wrong_revision(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    content = b"trusted model input"
    artifact = lifecycle.Artifact("config.json", len(content), hashlib.sha256(content).hexdigest())
    monkeypatch.setattr(lifecycle, "ARTIFACTS", (artifact,))
    model = tmp_path / "active"
    model.mkdir()
    (model / "config.json").write_bytes(content)
    (model / lifecycle.MANIFEST_NAME).write_text(
        json.dumps({
            "modelId": constants.MODEL_ID,
            "revision": "0000000000000000000000000000000000000000",
        }),
        encoding="utf-8",
    )
    with pytest.raises(lifecycle.LifecycleError, match="does not match"):
        lifecycle.validate_model(model)


def test_runner_uses_trust_remote_code_false() -> None:
    """Verify that the runner source code never enables trust_remote_code."""
    import inspect

    from hy_mt_poc import runner as runner_module

    source = inspect.getsource(runner_module)
    assert "trust_remote_code=True" not in source, (
        "Runner must never enable trust_remote_code"
    )
    assert "trust_remote_code=False" in source, (
        "Runner must explicitly set trust_remote_code=False"
    )


def test_serve_hardened_environment_blocks_hub_tokens(monkeypatch: pytest.MonkeyPatch) -> None:
    from hy_mt_poc.server import hardened_environment

    for token_name in ("HF_TOKEN", "HUGGING_FACE_HUB_TOKEN", "HUGGINGFACE_HUB_TOKEN"):
        monkeypatch.setenv(token_name, "secret-value")
        with pytest.raises(lifecycle.LifecycleError, match="credentials"):
            hardened_environment()
        monkeypatch.delenv(token_name)
