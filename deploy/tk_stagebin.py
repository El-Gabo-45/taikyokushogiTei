#!/usr/bin/env python3
"""deploy/tk_stagebin.py — update the engine used in running tournaments.

Stages a new binary and schedules it for the next tournament round
without interrupting in-progress games.

    python3 deploy/tk_stagebin.py --binary target/release/taikyokushogi-usi
"""
import argparse
import json
import shutil
import sys
from pathlib import Path


def stage_binary(binary_path):
    src = Path(binary_path)
    if not src.exists():
        print(f"✗ binary not found: {src}", file=sys.stderr)
        sys.exit(1)
    staged = Path("data/tourney/staged_engine")
    staged.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, staged)
    meta = staged.with_suffix(".meta.json")
    meta.write_text(json.dumps({
        "source": str(src),
        "staged_at": __import__("datetime").datetime.now().isoformat(),
    }) + "\n", encoding="utf-8")
    print(f"✓ staged engine at {staged}")


def main():
    ap = argparse.ArgumentParser(description="Stage a tournament engine update.")
    ap.add_argument("--binary", required=True, help="path to new engine binary")
    args = ap.parse_args()
    stage_binary(args.binary)


if __name__ == "__main__":
    main()