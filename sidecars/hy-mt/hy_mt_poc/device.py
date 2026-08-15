"""Explicit accelerator and dtype policy for the M5 gate."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from typing import Any


@dataclass(frozen=True)
class DeviceDecision:
    requested: str
    selected: str
    dtype: str
    mps_built: bool
    mps_available: bool
    fallback_used: bool

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


def select_device(torch_module: Any, requested: str = "mps") -> DeviceDecision:
    if requested not in {"mps", "cpu"}:
        raise ValueError("device must be either 'mps' or 'cpu'")

    mps_built = bool(torch_module.backends.mps.is_built())
    mps_available = bool(torch_module.backends.mps.is_available())

    if requested == "mps":
        if not mps_built:
            raise RuntimeError("MPS was requested but this PyTorch build has no MPS support")
        if not mps_available:
            raise RuntimeError("MPS was requested but is unavailable on this machine")
        return DeviceDecision(
            requested=requested,
            selected="mps",
            dtype="bfloat16",
            mps_built=mps_built,
            mps_available=mps_available,
            fallback_used=False,
        )

    return DeviceDecision(
        requested=requested,
        selected="cpu",
        dtype="float32",
        mps_built=mps_built,
        mps_available=mps_available,
        fallback_used=False,
    )


def torch_dtype(torch_module: Any, dtype_name: str) -> Any:
    return {
        "bfloat16": torch_module.bfloat16,
        "float32": torch_module.float32,
    }[dtype_name]
