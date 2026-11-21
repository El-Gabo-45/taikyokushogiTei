#!/usr/bin/env python3
"""deploy/tk_labelvault.py — content-addressed storage for training labels.

Stores label datasets under data/labels/ keyed by SHA-256 of contents,
so duplicate datasets are never stored twice.

    python3 deploy/tk_labelvault.py store data/derived/labels.jsonl
    python3 deploy/tk_labelvault.py list
"""
import argparse
import hashlib
import json
import sys
from pathlib import Path

STORE_DIR = Path("data/labels")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def store(src):
    src = Path(src)
    digest = sha256_file(src)
    dest = STORE_DIR / f"{digest}.jsonl"
    STORE_DIR.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        print(f"    already stored: {dest}")
    else:
        import shutil
        shutil.copy2(src, dest)
        print(f"    stored: {dest}")
    meta = STORE_DIR / f"{digest}.meta.json"
    meta.write_text(json.dumps({"source": str(src), "sha256": digest}) + "\n",
                    encoding="utf-8")
    print(f"digest: {digest}")


def list_labels():
    STORE_DIR.mkdir(parents=True, exist_ok=True)
    for f in sorted(STORE_DIR.glob("*.meta.json")):
        print(f.read_text(encoding="utf-8").strip())


def main():
    ap = argparse.ArgumentParser(description="Content-addressed label storage.")
    ap.add_argument("action", choices=["store", "list"])
    ap.add_argument("path", nargs="?", help="file to store (for 'store')")
    args = ap.parse_args()

    if args.action == "store":
        if not args.path:
            print("✗ path required for 'store'", file=sys.stderr)
            sys.exit(2)
        store(args.path)
    else:
        list_labels()


if __name__ == "__main__":
    main()