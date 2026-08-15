"""Run a POC command and preserve auditable wall-time/sandbox provenance."""

from __future__ import annotations

import argparse
import os
import subprocess
import time
from pathlib import Path

from .evidence import write_json


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--label", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--sandbox-profile")
    parser.add_argument("--state-note", required=True)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if not args.command:
        parser.error("a command is required after '--'")
    command = args.command[1:] if args.command[0] == "--" else args.command
    started = time.monotonic()
    completed = subprocess.run(command, check=False, capture_output=True, text=True)
    payload = {
        "label": args.label,
        "workingDirectory": os.getcwd(),
        "command": command,
        "sandboxProfile": args.sandbox_profile,
        "stateNote": args.state_note,
        "wallSeconds": round(time.monotonic() - started, 6),
        "exitCode": completed.returncode,
        "stdout": completed.stdout.strip(),
        "stderr": completed.stderr.strip(),
    }
    write_json(args.output, payload)
    if completed.returncode:
        raise SystemExit(completed.returncode)
    print(
        f"{args.label}: exit={completed.returncode} wall={payload['wallSeconds']:.3f}s",
        flush=True,
    )


if __name__ == "__main__":
    main()
