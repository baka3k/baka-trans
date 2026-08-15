"""Normalize sensitive local identifiers in already-generated JSON evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from .evidence import write_json


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args()
    for path in args.files:
        payload = json.loads(path.read_text(encoding="utf-8"))
        write_json(path, payload)
        print(f"normalized {path}")


if __name__ == "__main__":
    main()
