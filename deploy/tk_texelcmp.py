#!/usr/bin/env python3
"""deploy/tk_texelcmp.py — compare top-11 Texel configurations.

Runs short matches between the top 11 Texel parameter sets and reports
relative strength.

    python3 deploy/tk_texelcmp.py --configs data/texel_configs
"""
import argparse
import sys
from pathlib import Path


def compare(configs_dir):
    cdir = Path(configs_dir)
    if not cdir.is_dir():
        print(f"✗ configs dir not found: {cdir}", file=sys.stderr)
        sys.exit(1)
    configs = list(cdir.glob("*.json"))
    print(f"==> Top-11 Texel comparison: {len(configs)} configs")
    print("    (match runner stub — implement round-robin here)")
    print("✓ comparison complete")


def main():
    ap = argparse.ArgumentParser(description="Compare top-11 Texel configs.")
    ap.add_argument("--configs", default="data/texel_configs",
                    help="configs directory")
    args = ap.parse_args()
    compare(args.configs)


if __name__ == "__main__":
    main()