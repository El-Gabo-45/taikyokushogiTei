#!/usr/bin/env python3
"""deploy/tk_watch.py — sidecar process that monitors training progress.

Watches a training directory and writes status.json for the web GUI
and external consumers.

    python3 deploy/tk_watch.py data/run --interval 30
"""
import argparse
import json
import os
import sys
import time
from pathlib import Path


def read_status(run_dir):
    status = {"running": False, "epoch": 0, "games": 0, "elapsed_s": 0}
    st_file = Path(run_dir) / "status.json"
    if st_file.exists():
        try:
            status.update(json.loads(st_file.read_text(encoding="utf-8")))
        except Exception:
            pass
    return status


def main():
    ap = argparse.ArgumentParser(description="Training progress sidecar.")
    ap.add_argument("run_dir", help="training run directory")
    ap.add_argument("--interval", type=float, default=30.0,
                    help="poll interval seconds (default 30)")
    args = ap.parse_args()

    run_dir = Path(args.run_dir)
    if not run_dir.is_dir():
        print(f"✗ run dir not found: {run_dir}", file=sys.stderr)
        sys.exit(1)

    print(f"==> sidecar watching {run_dir} (interval {args.interval}s)")
    try:
        while True:
            status = read_status(run_dir)
            out_file = run_dir / "sidecar_status.json"
            out_file.write_text(json.dumps(status, indent=2) + "\n", encoding="utf-8")
            time.sleep(args.interval)
    except KeyboardInterrupt:
        print("\n✓ sidecar stopped")


if __name__ == "__main__":
    main()