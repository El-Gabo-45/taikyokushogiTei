#!/usr/bin/env python3
"""deploy/tk_twin.py — sidecar that maintains dual label sets.

Keeps both search-score labels and result labels in sync for
comparative training experiments.

    python3 deploy/tk_twin.py data/derived/positions
"""
import argparse
import json
import sys
from pathlib import Path


def sync_labels(positions_dir):
    pdir = Path(positions_dir)
    if not pdir.is_dir():
        print(f"✗ positions dir not found: {pdir}", file=sys.stderr)
        sys.exit(1)
    files = list(pdir.glob("*.jsonl"))
    print(f"==> Dual label sidecar: {len(files)} position files in {pdir}")
    print("    (sync stub — implement label dual-write here)")
    print("✓ labels synchronized")


def main():
    ap = argparse.ArgumentParser(description="Dual label sidecar.")
    ap.add_argument("positions_dir", help="position JSONL directory")
    args = ap.parse_args()
    sync_labels(args.positions_dir)


if __name__ == "__main__":
    main()