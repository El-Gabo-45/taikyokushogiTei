#!/usr/bin/env python3
"""Calibrate the 7 output-bucket thresholds from the piece-count distribution.

Thresholds are the octiles of the number of pieces on the board over the
training data, so every bucket sees a comparable share of samples (each at
least 5 %). They are frozen in rules/nnue_arch.json by the Rust tool, because
the thresholds are part of the architecture hash:

  python training/calibrate_buckets.py DATA...            # print the command
  python training/calibrate_buckets.py DATA... --apply    # run it
"""

import argparse
import subprocess
import sys
from pathlib import Path

import numpy as np

import dataset
from arch import OUT_BUCKETS

ROOT = Path(__file__).resolve().parent.parent


def thresholds_from(npieces, min_share=0.05):
    npieces = np.asarray(npieces)
    qs = [100 * k / OUT_BUCKETS for k in range(1, OUT_BUCKETS)]
    th = [int(np.percentile(npieces, q)) for q in qs]
    # Strictly ascending: nudge repeated values up by one.
    for i in range(1, len(th)):
        if th[i] <= th[i - 1]:
            th[i] = th[i - 1] + 1
    edges = [0] + th + [10 ** 9]
    shares = [float(((npieces >= a) & (npieces < b)).mean()) for a, b in zip(edges[:-1], edges[1:])]
    return th, shares, all(s >= min_share for s in shares)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data", nargs="+")
    ap.add_argument("--apply", action="store_true", help="write rules/nnue_arch.json via `nnue gen-classes`")
    args = ap.parse_args()
    d = dataset.load(args.data)
    th, shares, ok = thresholds_from(d.npieces)
    print("thresholds:", th)
    print("bucket shares:", " ".join(f"{s:.1%}" for s in shares))
    if not ok:
        print("WARNING: some bucket has < 5 % of the samples (too little variety in the piece counts)")
    cmd = ["cargo", "run", "--release", "--bin", "nnue", "--", "gen-classes", "--thresholds", ",".join(map(str, th))]
    print("command:", " ".join(cmd))
    if args.apply:
        sys.exit(subprocess.call(cmd, cwd=ROOT))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
