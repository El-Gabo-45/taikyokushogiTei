#!/usr/bin/env python3
"""Label-unit gate: do the training labels live on the engine's scale?

Regresses the search score stored in the samples against the engine's
hand-crafted (classic) evaluation of the same positions — both in
internal units where a pawn is 500. A slope far from 1 means the labels
are in the wrong unit and the network would learn a scale the search
margins (futility, razoring, ...) do not expect. This cannot be caught
by the parity gate.

Passes when slope is in [0.8, 1.25] and correlation > 0.9 (on positions
whose score is not a mate score).

Usage:
    python3 deploy/tk_scale.py data/derived/positions/*.jsonl
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np

SLOPE_RANGE = (0.8, 1.25)
MIN_CORR = 0.9
MAX_ABS = 100_000


def load_samples(paths):
    rows = []
    for p in paths:
        p = Path(p)
        if p.is_dir():
            for f in sorted(p.glob("*.jsonl")):
                rows.extend(_read_jsonl(f))
        else:
            rows.extend(_read_jsonl(p))
    if not rows:
        raise SystemExit("unit gate: no samples found")
    return rows


def _read_jsonl(path):
    out = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def check(samples, max_abs=MAX_ABS, verbose=True):
    classic = np.array([float(s.get("classic", s.get("classic_eval", 0.0))) for s in samples],
                       dtype=np.float64)
    score = np.array([float(s.get("score", 0.0)) for s in samples], dtype=np.float64)

    m = (np.abs(score) < max_abs) & (np.abs(classic) < max_abs)
    if int(m.sum()) < 20:
        raise SystemExit("unit gate: fewer than 20 usable samples")

    x = classic[m]
    y = score[m]
    if x.std() == 0 or y.std() == 0:
        raise SystemExit("unit gate: constant labels or constant classic eval (degenerate data)")

    xc = x - x.mean()
    yc = y - y.mean()
    slope = float(np.dot(xc, yc) / np.dot(xc, xc))
    corr = float(np.dot(xc, yc) / (np.sqrt(np.dot(xc, xc) * np.dot(yc, yc))))

    ok = SLOPE_RANGE[0] <= slope <= SLOPE_RANGE[1] and corr > MIN_CORR
    if verbose:
        status = "PASS" if ok else "FAIL"
        print(f"unit gate: {int(m.sum())} samples | "
              f"slope(score ~ classic) = {slope:.3f} | "
              f"corr = {corr:.3f} | {status}")
    return ok, slope, corr


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data", nargs="+", help="samples JSONL files or directories")
    ap.add_argument("--max-abs-score", type=float, default=MAX_ABS,
                    help="drop labels with |score| >= this (default 100000)")
    args = ap.parse_args()

    samples = load_samples(args.data)
    ok, _, _ = check(samples, max_abs=args.max_abs_score)
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
