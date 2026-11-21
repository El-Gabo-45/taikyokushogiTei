#!/usr/bin/env python3
"""deploy/tk_regmodel.py — register trained models in a manifest.

Appends model metadata (path, hash, training run) to models/manifest.json.

    python3 deploy/tk_regmodel.py models/net.tkn --run run_001
"""
import argparse
import hashlib
import json
import sys
from datetime import datetime
from pathlib import Path


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def register(model_path, run_id):
    model = Path(model_path)
    if not model.exists():
        print(f"✗ model not found: {model}", file=sys.stderr)
        sys.exit(1)
    manifest = Path("models/manifest.json")
    manifest.parent.mkdir(parents=True, exist_ok=True)
    entries = []
    if manifest.exists():
        try:
            entries = json.loads(manifest.read_text(encoding="utf-8"))
        except Exception:
            entries = []
    entries.append({
        "model": str(model),
        "sha256": sha256_file(model),
        "run": run_id,
        "registered_at": datetime.now().isoformat(),
    })
    manifest.write_text(json.dumps(entries, indent=2) + "\n", encoding="utf-8")
    print(f"✓ registered {model} in {manifest}")


def main():
    ap = argparse.ArgumentParser(description="Register trained models.")
    ap.add_argument("model", help="model file to register")
    ap.add_argument("--run", default="unknown", help="training run id")
    args = ap.parse_args()
    register(args.model, args.run)


if __name__ == "__main__":
    main()