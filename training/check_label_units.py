#!/usr/bin/env python3
"""Unit gate: do the training labels live on the same scale as the engine?

Regresses the search score stored in the samples against the engine's
hand-crafted evaluation of the same positions (both in internal units,
pawn = 500). A slope far from 1 means the labels are in the wrong unit and
the network would learn a scale the search margins (futility, razoring, ...)
do not expect. This cannot be caught by the integer parity gate.

Passes when slope is in [0.8, 1.25] and correlation > 0.9 (on positions
whose score is not a mate score).
"""

import argparse
import sys

import numpy as np

import dataset

SLOPE_RANGE = (0.8, 1.25)
MIN_CORR = 0.9


def check(data, max_abs=100_000, verbose=True):
    m = (np.abs(data.score) < max_abs) & (np.abs(data.classic) < max_abs)
    if m.sum() < 20:
        raise SystemExit("unit gate: fewer than 20 usable samples")
    x, y = data.classic[m].astype(np.float64), data.score[m].astype(np.float64)
    if x.std() == 0 or y.std() == 0:
        raise SystemExit("unit gate: constant labels or constant classic eval (degenerate data)")
    slope = float(np.dot(x - x.mean(), y - y.mean()) / np.dot(x - x.mean(), x - x.mean()))
    corr = float(np.corrcoef(x, y)[0, 1])
    ok = SLOPE_RANGE[0] <= slope <= SLOPE_RANGE[1] and corr > MIN_CORR
    if verbose:
        print(f"unit gate: {int(m.sum())} samples | slope(score ~ classic) = {slope:.3f} | corr = {corr:.3f} "
              f"| {'PASS' if ok else 'FAIL'}")
    return ok, slope, corr


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data", nargs="+", help="samples_*.bin files or directories")
    args = ap.parse_args()
    ok, _, _ = check(dataset.load(args.data))
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
