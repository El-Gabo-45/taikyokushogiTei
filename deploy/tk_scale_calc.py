#!/usr/bin/env python3
"""deploy/tk_scale_calc.py — determine the sigmoid scale for training labels.

Analyzes score distribution to pick the sigmoid scale SC such that
median |score| maps to ~75% win probability.

    python3 deploy/tk_scale_calc.py data/derived/labels.jsonl
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np


def compute_scale(labels_path):
    scores = []
    with open(labels_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                scores.append(abs(json.loads(line).get("score", 0.0)))
    if not scores:
        print("✗ no scores found", file=sys.stderr)
        sys.exit(1)
    arr = np.array(scores)
    med = float(np.median(arr))
    sc = med / float(np.log(3.0))
    sc = float(np.clip(sc, 2000.0, 200000.0))
    print(f"median |score|: {med:.0f}")
    print(f"sigmoid scale SC: {sc:.0f}")
    return sc


def main():
    ap = argparse.ArgumentParser(description="Compute sigmoid scale for labels.")
    ap.add_argument("labels", help="labels JSONL file")
    args = ap.parse_args()
    compute_scale(args.labels)


if __name__ == "__main__":
    main()