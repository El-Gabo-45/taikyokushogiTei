#!/usr/bin/env python3
"""deploy/tk_dedupe.py — compact and deduplicate stored artifacts.

Scans data/artifacts/ and removes duplicate files (same content hash),
keeping one copy. Reports bytes saved.

    python3 deploy/tk_dedupe.py --apply
    python3 deploy/tk_dedupe.py            # dry run
"""
import argparse
import hashlib
import sys
from pathlib import Path


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def compact(store_dir, apply):
    store_dir = Path(store_dir)
    if not store_dir.is_dir():
        print(f"✗ store dir not found: {store_dir}", file=sys.stderr)
        sys.exit(1)

    seen = {}
    duplicates = []
    saved = 0

    for f in store_dir.iterdir():
        if not f.is_file():
            continue
        digest = sha256_file(f)
        if digest in seen:
            duplicates.append((f, seen[digest]))
            saved += f.stat().st_size
        else:
            seen[digest] = f

    print(f"files scanned: {len(seen) + len(duplicates)}")
    print(f"duplicates found: {len(duplicates)}")
    print(f"bytes that would be saved: {saved}")

    if apply and duplicates:
        for dup, _ in duplicates:
            dup.unlink()
        print(f"✓ removed {len(duplicates)} duplicates, saved {saved} bytes")
    elif not apply:
        print("(dry run — pass --apply to actually remove)")


def main():
    ap = argparse.ArgumentParser(description="Compact and deduplicate artifacts.")
    ap.add_argument("--store-dir", default="data/artifacts",
                    help="artifact store directory (default data/artifacts)")
    ap.add_argument("--apply", action="store_true",
                    help="actually remove duplicates (default: dry run)")
    args = ap.parse_args()
    compact(args.store_dir, args.apply)


if __name__ == "__main__":
    main()