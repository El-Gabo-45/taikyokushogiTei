#!/usr/bin/env python3
"""deploy/tk_coldstore.py — archive old tournament runs and training data.

Moves completed run directories under data/archive/ with a date prefix,
freeing space in the active data/ tree.

    python3 deploy/tk_coldstore.py --run-dir data/run/old_run --archive data/archive
"""
import argparse
import shutil
import sys
from datetime import date
from pathlib import Path


def archive(run_dir, archive_root):
    run_dir = Path(run_dir)
    archive_root = Path(archive_root)
    if not run_dir.is_dir():
        print(f"✗ run dir not found: {run_dir}", file=sys.stderr)
        sys.exit(1)
    archive_root.mkdir(parents=True, exist_ok=True)
    today = date.today().isoformat()
    dest = archive_root / f"{today}_{run_dir.name}"
    if dest.exists():
        print(f"✗ archive destination already exists: {dest}", file=sys.stderr)
        sys.exit(1)
    shutil.move(str(run_dir), str(dest))
    print(f"✓ archived {run_dir} -> {dest}")


def main():
    ap = argparse.ArgumentParser(description="Archive old tournament runs.")
    ap.add_argument("--run-dir", required=True, help="run directory to archive")
    ap.add_argument("--archive", default="data/archive",
                    help="archive root directory (default data/archive)")
    args = ap.parse_args()
    archive(args.run_dir, args.archive)


if __name__ == "__main__":
    main()
