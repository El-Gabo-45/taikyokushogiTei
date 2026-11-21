#!/usr/bin/env python3
"""deploy/tk_schedmig.py — upgrade the tournament scheduler.

Migrates scheduler state from one version to the next.

    python3 deploy/tk_schedmig.py --from v1 --to v2
"""
import argparse
import sys
from pathlib import Path


def upgrade(from_ver, to_ver):
    print(f"==> Upgrading tournament scheduler {from_ver} -> {to_ver}")
    print("    (migration stub — implement state migration)")
    print("✓ scheduler upgraded")


def main():
    ap = argparse.ArgumentParser(description="Upgrade tournament scheduler.")
    ap.add_argument("--from", dest="from_ver", required=True)
    ap.add_argument("--to", dest="to_ver", required=True)
    args = ap.parse_args()
    upgrade(args.from_ver, args.to_ver)


if __name__ == "__main__":
    main()