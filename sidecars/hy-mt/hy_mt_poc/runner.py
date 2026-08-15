"""Single-load local-only HY-MT inference runner."""

from __future__ import annotations

import os
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")
os.environ.setdefault("PYTORCH_ENABLE_MPS_FALLBACK", "0")
os.environ.setdefault("TOKENIZERS_PARALLELISM", "false")

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer, StoppingCriteria, StoppingCriteriaList

from .constants import MAX_NEW_TOKENS
from .decoding import decode_generated_suffix
from .device import DeviceDecision, select_device, torch_dtype
from .evidence import memory_snapshot
from .prompting import tokenize_chat


class DeadlineCriteria(StoppingCriteria):
    def __init__(self, deadline: float | None) -> None:
        self.deadline = deadline
        self.triggered = False

    def __call__(self, input_ids: Any, scores: Any, **kwargs: Any) -> bool:
        del input_ids, scores, kwargs
        self.triggered = self.deadline is not None and time.monotonic() >= self.deadline
        return self.triggered


@dataclass(frozen=True)
class TranslationResult:
    text: str
    input_tokens: int
    output_tokens: int
    latency_ms: float
    tokens_per_second: float
    generation_mode: str
    cancelled: bool
    memory: dict[str, Any]

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


class HyMtRunner:
    def __init__(self, model_dir: Path, *, requested_device: str = "mps") -> None:
        self.model_dir = model_dir.resolve()
        if not self.model_dir.is_dir():
            raise RuntimeError(f"model directory does not exist: {self.model_dir}")
        self.device: DeviceDecision = select_device(torch, requested_device)
        started = time.monotonic()
        self.tokenizer = AutoTokenizer.from_pretrained(
            self.model_dir,
            local_files_only=True,
            trust_remote_code=False,
        )
        self.model = AutoModelForCausalLM.from_pretrained(
            self.model_dir,
            local_files_only=True,
            trust_remote_code=False,
            use_safetensors=True,
            dtype=torch_dtype(torch, self.device.dtype),
        )
        self.model.to(self.device.selected)
        self.model.eval()
        if self.device.selected == "mps":
            torch.mps.synchronize()
        self.load_ms = (time.monotonic() - started) * 1_000
        self.actual_dtype = str(next(self.model.parameters()).dtype).removeprefix("torch.")
        self.actual_device = str(next(self.model.parameters()).device)
        self.loaded_memory = memory_snapshot(torch)

    def metadata(self) -> dict[str, Any]:
        return {
            "device": self.device.to_dict(),
            "actualDtype": self.actual_dtype,
            "actualDevice": self.actual_device,
            "modelLoadMs": self.load_ms,
            "memoryAfterLoad": self.loaded_memory,
        }

    def render_prompt(self, source_text: str) -> dict[str, Any]:
        input_ids = tokenize_chat(self.tokenizer, source_text)
        return {
            "rendered": self.tokenizer.decode(input_ids[0], skip_special_tokens=False),
            "inputTokens": int(input_ids.shape[1]),
        }

    def translate(
        self,
        source_text: str,
        *,
        generation_mode: str,
        max_new_tokens: int = MAX_NEW_TOKENS,
        timeout_seconds: float | None = None,
        seed: int = 0,
    ) -> TranslationResult:
        if generation_mode not in {"greedy", "recommended"}:
            raise ValueError("generation mode must be 'greedy' or 'recommended'")
        if max_new_tokens <= 0 or max_new_tokens > MAX_NEW_TOKENS:
            raise ValueError(f"max new tokens must be between 1 and {MAX_NEW_TOKENS}")

        input_ids = tokenize_chat(self.tokenizer, source_text).to(self.device.selected)
        input_tokens = int(input_ids.shape[1])
        deadline = None if timeout_seconds is None else time.monotonic() + timeout_seconds
        deadline_criteria = DeadlineCriteria(deadline)
        generation: dict[str, Any] = {
            "max_new_tokens": max_new_tokens,
            "stopping_criteria": StoppingCriteriaList([deadline_criteria]),
            "pad_token_id": self.tokenizer.pad_token_id,
        }
        if generation_mode == "greedy":
            generation.update(
                do_sample=False,
                repetition_penalty=1.0,
                temperature=None,
                top_k=None,
                top_p=None,
            )
        else:
            generation.update(
                do_sample=True,
                repetition_penalty=1.05,
                temperature=0.7,
                top_k=20,
                top_p=0.6,
            )

        torch.manual_seed(seed)
        started = time.monotonic()
        with torch.inference_mode():
            generated = self.model.generate(input_ids, **generation)
        if self.device.selected == "mps":
            torch.mps.synchronize()
        latency_ms = (time.monotonic() - started) * 1_000
        output_tokens = int(generated.shape[1] - input_tokens)
        text = decode_generated_suffix(self.tokenizer, generated, input_tokens)
        return TranslationResult(
            text=text,
            input_tokens=input_tokens,
            output_tokens=output_tokens,
            latency_ms=round(latency_ms, 3),
            tokens_per_second=round(output_tokens / max(latency_ms / 1_000, 0.001), 3),
            generation_mode=generation_mode,
            cancelled=deadline_criteria.triggered,
            memory=memory_snapshot(torch),
        )
