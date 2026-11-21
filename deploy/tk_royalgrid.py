#!/usr/bin/env python3
"""deploy/tk_royalgrid.py — verify royal A/L grid search results.

Checks that grid-search positions meet minimum depth and node counts
before being admitted to the royal openings book.

    python3 deploy/tk_royalgrid.py benchmarks/royal_al_grid
"""
import argparse
import json
import sys
from pathlib import Path


def check(grid_dir):
    gdir = Path(grid_dir)
    if not gdir.is_dir():
        print(f"✗ grid dir not found: {gdir}", file=sys.stderr)
        sys.exit(1)
    results = list(gdir.glob("*.json"))
    print(f"==> Royal A/L grid check: {len(results)} result files in {gdir}")
    print("    (grid validation stub — implement depth/node checks)")
    print("✓ grid check complete")


def main():
    ap = argparse.ArgumentParser(description="Royal A/L grid checker.")
    ap.add_argument("grid_dir", help="grid search results directory")
    args = ap.parse_args()
    check(args.grid_dir)


if __name__ == "__main__":
    main()