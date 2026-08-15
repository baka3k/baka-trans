from __future__ import annotations

from types import SimpleNamespace

import pytest

from hy_mt_poc.device import select_device


def fake_torch(*, built: bool, available: bool):
    mps = SimpleNamespace(is_built=lambda: built, is_available=lambda: available)
    return SimpleNamespace(backends=SimpleNamespace(mps=mps))


def test_mps_is_selected_without_silent_fallback() -> None:
    decision = select_device(fake_torch(built=True, available=True), "mps")
    assert decision.selected == "mps"
    assert decision.dtype == "bfloat16"
    assert decision.fallback_used is False


@pytest.mark.parametrize(
    ("built", "available", "message"),
    [
        (False, False, "no MPS support"),
        (True, False, "unavailable"),
    ],
)
def test_mps_failure_is_explicit(built: bool, available: bool, message: str) -> None:
    with pytest.raises(RuntimeError, match=message):
        select_device(fake_torch(built=built, available=available), "mps")


def test_cpu_must_be_requested_explicitly() -> None:
    decision = select_device(fake_torch(built=True, available=True), "cpu")
    assert decision.selected == "cpu"
    assert decision.dtype == "float32"
    assert decision.fallback_used is False
