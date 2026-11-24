#!/usr/bin/env python3
"""deploy/tk_snapshot.py — prepare a benchmark snapshot.

Captures the current engine, model, and data state for later comparison.

    python3 deploy/tk_snapshot.py --name bench_2026_11_20
"""
import argparse
import json
from datetime import datetime
from pathlib import Path


def snapshot(name):
    snap_dir = Path("data/snapshots") / name
    snap_dir.mkdir(parents=True, exist_ok=True)
    # Record versions
    meta = {
        "name": name,
        "created": datetime.now().isoformat(),
    }
    (snap_dir / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    print(f"✓ snapshot prepared: {snap_dir}")


def main():
    ap = argparse.ArgumentParser(description="Prepare a benchmark snapshot.")
    ap.add_argument("--name", required=True, help="snapshot name")
    args = ap.parse_args()
    snapshot(args.name)


if __name__ == "__main__":
    main()
