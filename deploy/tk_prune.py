#!/usr/bin/env python3
"""deploy/tk_prune.py — prune old run artifacts to free space.

Removes run directories older than a cutoff, keeping the most recent N.

    python3 deploy/tk_prune.py --keep 10 --older-than 30d
"""
import argparse
import shutil
import sys
from datetime import datetime, timedelta
from pathlib import Path


def prune(run_dir, keep, older_than_days):
    rdir = Path(run_dir)
    if not rdir.is_dir():
        print(f"✗ run dir not found: {rdir}", file=sys.stderr)
        sys.exit(1)
    cutoff = datetime.now() - timedelta(days=older_than_days)
    dirs = sorted(rdir.iterdir(), key=lambda p: p.stat().st_mtime, reverse=True)
    removed = 0
    for i, d in enumerate(dirs):
        too_old = d.stat().st_mtime < cutoff.timestamp()
        too_many = i >= keep
        if too_old or too_many:
            shutil.rmtree(d)
            removed += 1
            print(f"    removed: {d.name}")
    print(f"✓ pruned {removed} run directories")


def main():
    ap = argparse.ArgumentParser(description="Prune old run artifacts.")
    ap.add_argument("--run-dir", default="data/run", help="run directory")
    ap.add_argument("--keep", type=int, default=10, help="keep most recent N")
    ap.add_argument("--older-than", type=int, default=30,
                    help="remove dirs older than N days")
    args = ap.parse_args()
    prune(args.run_dir, args.keep, args.older_than)


if __name__ == "__main__":
    main()
