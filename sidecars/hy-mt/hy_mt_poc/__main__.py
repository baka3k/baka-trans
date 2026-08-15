"""Command-line entry point for the Phase 09 POC."""

from __future__ import annotations

import argparse
import json
import os
import socket
from pathlib import Path
from typing import Any

from .evidence import environment_manifest, runtime_manifest, write_json


def deny_network() -> None:
    def blocked(*args: Any, **kwargs: Any) -> Any:
        del args, kwargs
        raise RuntimeError("network access disabled by HY-MT POC")

    socket.create_connection = blocked
    socket.socket.connect = blocked
    socket.socket.connect_ex = blocked
    socket.socket.sendto = blocked


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description="Hy-MT2 M5 proof-of-concept")
    result.add_argument("--deny-network", action="store_true")
    subparsers = result.add_subparsers(dest="command", required=True)

    download = subparsers.add_parser("download")
    download.add_argument("--model-dir", type=Path, required=True)
    download.add_argument("--manifest", type=Path, required=True)

    probe = subparsers.add_parser("probe")
    probe.add_argument("--output", type=Path)

    environment = subparsers.add_parser("environment")
    environment.add_argument("--output", type=Path, required=True)

    prompt = subparsers.add_parser("prompt")
    prompt.add_argument("--model-dir", type=Path, required=True)
    prompt.add_argument("--device", choices=("mps", "cpu"), default="mps")
    prompt.add_argument("--output", type=Path)
    prompt.add_argument("text")

    translate = subparsers.add_parser("translate")
    translate.add_argument("--model-dir", type=Path, required=True)
    translate.add_argument("--device", choices=("mps", "cpu"), default="mps")
    translate.add_argument("--mode", choices=("greedy", "recommended"), default="greedy")
    translate.add_argument("--timeout-seconds", type=float)
    translate.add_argument("--output", type=Path)
    translate.add_argument("text")

    benchmark = subparsers.add_parser("benchmark")
    benchmark.add_argument("--model-dir", type=Path, required=True)
    benchmark.add_argument("--device", choices=("mps", "cpu"), default="mps")
    benchmark.add_argument("--corpus", type=Path, required=True)
    benchmark.add_argument("--output-dir", type=Path, required=True)
    benchmark.add_argument(
        "--modes",
        nargs="+",
        choices=("greedy", "recommended"),
        default=("greedy", "recommended"),
    )

    soak = subparsers.add_parser("soak")
    soak.add_argument("--model-dir", type=Path, required=True)
    soak.add_argument("--device", choices=("mps", "cpu"), default="mps")
    soak.add_argument("--corpus", type=Path, required=True)
    soak.add_argument("--output-dir", type=Path, required=True)
    soak.add_argument("--duration-seconds", type=float, default=1_800)
    soak.add_argument("--interval-seconds", type=float, default=5)

    return result


def emit(payload: Any, output: Path | None = None) -> None:
    if output is not None:
        write_json(output, payload)
    print(json.dumps(payload, ensure_ascii=False, indent=2))


def main() -> None:
    args = parser().parse_args()
    if args.deny_network:
        os.environ["HF_HUB_OFFLINE"] = "1"
        os.environ["TRANSFORMERS_OFFLINE"] = "1"
        os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
        deny_network()

    if args.command == "download":
        from .download import download_model

        payload = download_model(args.model_dir)
        write_json(args.manifest, payload)
        emit(payload)
        return

    if args.command == "probe":
        import torch

        from .device import select_device

        payload = {
            "runtime": runtime_manifest(),
            "mps": select_device(torch, "mps").to_dict(),
        }
        emit(payload, args.output)
        return

    if args.command == "environment":
        emit(environment_manifest(), args.output)
        return

    from .runner import HyMtRunner

    runner = HyMtRunner(args.model_dir, requested_device=args.device)
    if args.command == "prompt":
        emit(
            {"runner": runner.metadata(), "prompt": runner.render_prompt(args.text)},
            args.output,
        )
    elif args.command == "translate":
        result = runner.translate(
            args.text,
            generation_mode=args.mode,
            timeout_seconds=args.timeout_seconds,
        )
        emit({"runner": runner.metadata(), "result": result.to_dict()}, args.output)
    elif args.command == "benchmark":
        from .benchmark import run_corpus

        payload = run_corpus(runner, args.corpus, args.output_dir, list(args.modes))
        emit(payload)
    elif args.command == "soak":
        from .benchmark import run_soak

        payload = run_soak(
            runner,
            args.corpus,
            args.output_dir,
            args.duration_seconds,
            args.interval_seconds,
        )
        emit(payload)


if __name__ == "__main__":
    main()
