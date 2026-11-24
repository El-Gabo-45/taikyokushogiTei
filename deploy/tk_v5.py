#!/usr/bin/env python3
"""deploy/tk_v5.py — v5 training pipeline (NNUE with gradient descent).

Runs a v5 training iteration: featurize, NNUE fit, export TKN.

    python3 deploy/tk_v5.py --data data/derived/positions --out net_v5.tkn
"""
import argparse
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
    print(f"==> v5 training: {len(files)} position files")
    print("    (NNUE gradient descent stub — implement PyTorch training here)")
    out.write_bytes(b"TKN1 placeholder\n")
    print(f"✓ v5 model exported: {out}")


def main():
    ap = argparse.ArgumentParser(description="v5 training pipeline.")
    ap.add_argument("--data", required=True, help="position data directory")
    ap.add_argument("--out", required=True, help="output TKN file")
    args = ap.parse_args()
    train(args.data, args.out)


if __name__ == "__main__":
    main()
