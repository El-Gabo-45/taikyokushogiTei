#!/usr/bin/env python3
"""deploy/tk_v4.py — v4 training pipeline (Texel-style fit).

Runs a v4 training iteration: featurize, fit, export checkpoint.

    python3 deploy/tk_v4.py --data data/derived/positions --out models/v4.json
"""
import argparse
import json
import sys
from pathlib import Path


def train(data_dir, out_path):
    data = Path(data_dir)
    out = Path(out_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    if not data.is_dir():
        print(f"✗ data dir not found: {data}", file=sys.stderr)
        sys.exit(1)
    files = list(data.glob("*.jsonl"))
    print(f"==> v4 training: {len(files)} position files")
    print("    (Texel fit stub — implement logistic regression here)")
    out.write_text(json.dumps({"version": 4, "trained_on": len(files)}) + "\n",
                   encoding="utf-8")
    print(f"✓ v4 model exported: {out}")


def main():
    ap = argparse.ArgumentParser(description="v4 training pipeline.")
    ap.add_argument("--data", required=True, help="position data directory")
    ap.add_argument("--out", required=True, help="output model file")
    args = ap.parse_args()
    train(args.data, args.out)


if __name__ == "__main__":
    main()