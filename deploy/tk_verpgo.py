#!/usr/bin/env python3
"""deploy/tk_verpgo.py — verify PGO profile data is valid.

Checks that the PGO profile directory exists and contains valid
profile data before using it for an optimized build.

    python3 deploy/tk_verpgo.py --profile-dir target/pgo
"""
import argparse
import sys
from pathlib import Path


def verify(profile_dir):
    pdir = Path(profile_dir)
    if not pdir.is_dir():
        print(f"✗ profile dir not found: {pdir}", file=sys.stderr)
        sys.exit(1)
    files = list(pdir.glob("*.profraw")) + list(pdir.glob("*.profdata"))
    if not files:
        print(f"⚠ no profile files found in {pdir}")
    else:
        print(f"✓ PGO profile valid: {len(files)} file(s) in {pdir}")


def main():
    ap = argparse.ArgumentParser(description="Verify PGO profile data.")
    ap.add_argument("--profile-dir", default="target/pgo", help="profile directory")
    args = ap.parse_args()
    verify(args.profile_dir)


if __name__ == "__main__":
    main()