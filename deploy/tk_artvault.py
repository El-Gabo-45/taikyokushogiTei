#!/usr/bin/env python3
"""deploy/tk_artvault.py — shared content-addressed NNUE artifact store.

Snapshots are stored under data/artifacts/ keyed by content hash.
Read-only to normal writers; used to share models between hosts.

    python3 deploy/tk_artvault.py store models/net.tkn
    python3 deploy/tk_artvault.py list
"""
import argparse
import hashlib
import json
import shutil
import sys
from pathlib import Path

STORE_DIR = Path("data/artifacts")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def store(src):
    src = Path(src)
    digest = sha256_file(src)
    dest = STORE_DIR / f"{digest}.tkn"
    STORE_DIR.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        print(f"    already stored: {dest}")
    else:
        shutil.copy2(src, dest)
        print(f"    stored: {dest}")
    print(f"digest: {digest}")


def list_artifacts():
    STORE_DIR.mkdir(parents=True, exist_ok=True)
    for f in sorted(STORE_DIR.glob("*.tkn")):
        print(f"{f.name}  ({f.stat().st_size} bytes)")


def main():
    ap = argparse.ArgumentParser(description="Content-addressed artifact store.")
    ap.add_argument("action", choices=["store", "list"])
    ap.add_argument("path", nargs="?", help="file to store (for 'store')")
    args = ap.parse_args()

    if args.action == "store":
        if not args.path:
            print("✗ path required for 'store'", file=sys.stderr)
            sys.exit(2)
        store(args.path)
    else:
        list_artifacts()


if __name__ == "__main__":
    main()