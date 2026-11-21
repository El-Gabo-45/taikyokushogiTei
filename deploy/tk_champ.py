#!/usr/bin/env python3
"""deploy/tk_champ.py — manage the champion teacher model.

Tracks the current champion and supports rollback to previous champions.

    python3 deploy/tk_champ.py --status
    python3 deploy/tk_champ.py --promote models/champion.tkn
    python3 deploy/tk_champ.py --rollback
"""
import argparse
import json
import shutil
import sys
from datetime import datetime
from pathlib import Path

CHAMPION = Path("models/champion.tkn")
PREVIOUS = Path("models/champion_previous.tkn")


def status():
    if CHAMPION.exists():
        print(f"champion: {CHAMPION} ({CHAMPION.stat().st_size} bytes)")
    else:
        print("champion: none")
    if PREVIOUS.exists():
        print(f"previous: {PREVIOUS} ({PREVIOUS.stat().st_size} bytes)")


def promote(path):
    src = Path(path)
    if not src.exists():
        print(f"✗ champion model not found: {src}", file=sys.stderr)
        sys.exit(1)
    if CHAMPION.exists():
        shutil.copy2(CHAMPION, PREVIOUS)
    shutil.copy2(src, CHAMPION)
    print(f"✓ champion promoted: {src} -> {CHAMPION}")


def rollback():
    if not PREVIOUS.exists():
        print("✗ no previous champion to roll back to", file=sys.stderr)
        sys.exit(1)
    shutil.copy2(PREVIOUS, CHAMPION)
    print(f"✓ rolled back to previous champion")


def main():
    ap = argparse.ArgumentParser(description="Champion teacher manager.")
    ap.add_argument("--status", action="store_true", help="show champion status")
    ap.add_argument("--promote", help="promote a model as champion")
    ap.add_argument("--rollback", action="store_true", help="rollback to previous")
    args = ap.parse_args()

    if args.status:
        status()
    elif args.promote:
        promote(args.promote)
    elif args.rollback:
        rollback()
    else:
        ap.print_help()


if __name__ == "__main__":
    main()