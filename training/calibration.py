#!/usr/bin/env python3
"""Evaluation Calibration Tool — do engine scores mean win probabilities?

    python3 training/calibration.py data
    python3 training/calibration.py data --sc 400 --bins 15

Loads TKD1 training files (`dataset.load`) and compares, per equal-count bin
of the predicted win rate, the prediction against the empirical outcome:

    predicted win rate = sigmoid(score / SC)
    empirical outcome  = (result + 1) / 2     # result is from stm's POV

`SC` defaults to the same auto rule as `train.py` (median |score| → 75 % win
probability), so this tool answers "is that scale right?" *before* spending
GPU hours: a network whose scores are not calibrated against results turns
the sigmoid target into noise.

Output: a reliability table plus ECE (expected calibration error, the
mean |predicted − empirical| weighted by bin size) and the overall bias.
Interpretation: ECE < 0.03 well calibrated · < 0.08 acceptable · ≥ 0.08 the
scale (or the labels) need work — re-run `calibrate_buckets.py`, check label
units, or fit `--sc` by minimizing the printed ECE.
"""

import argparse

import dataset
import numpy as np


def sigmoid(x):
    return 1.0 / (1.0 + np.exp(-x))


def auto_scale(score):
    """Same rule as train.py: median |score| maps to a 75 % win probability."""
    a = np.abs(score)
    median = np.median(a[a > 0]) if np.any(a > 0) else 500.0
    return float(median / np.log(3.0))


def main():
    ap = argparse.ArgumentParser(
        description="Score → win-rate calibration report over TKD1 files."
    )
    ap.add_argument("paths", nargs="+", help="TKD1 files or directories of samples_*.bin")
    ap.add_argument(
        "--sc",
        type=float,
        default=None,
        help="sigmoid scale; default = median |score| / ln 3 (train.py's rule)",
    )
    ap.add_argument("--bins", type=int, default=10, help="equal-count bins (default 10)")
    args = ap.parse_args()
    if args.bins < 2:
        raise SystemExit("--bins must be at least 2")

    d = dataset.load(args.paths)
    n = len(d)
    sc = args.sc if args.sc is not None else auto_scale(d.score)
    pred = sigmoid(np.asarray(d.score, dtype=np.float64) / sc)
    emp = (np.asarray(d.result, dtype=np.float64) + 1.0) / 2.0

    print("=== Evaluation calibration ===")
    print(f"samples: {n}   games: {len(np.unique(d.game_id))}")
    print(f"scale SC = {sc:.1f}" + ("  (--sc override)" if args.sc is not None else "  (auto: median |score| / ln 3)"))
    print()

    # Equal-count bins over the predicted win rate: edges at quantiles. With
    # concentrated predictions several quantiles coincide — dedupe them.
    edges = np.quantile(pred, np.linspace(0.0, 1.0, args.bins + 1))
    edges = np.unique(edges)
    if len(edges) - 1 < 2:
        raise SystemExit("predictions are too concentrated to bin; try fewer --bins")

    idx = np.clip(np.searchsorted(edges, pred, side="right") - 1, 0, len(edges) - 2)
    print("  bin (predicted)      n   pred    emp    diff")
    ece = 0.0
    rows = []
    for b in range(len(edges) - 1):
        mask = idx == b
        m = int(mask.sum())
        if m == 0:
            continue
        p = float(pred[mask].mean())
        e = float(emp[mask].mean())
        ece += m / n * abs(p - e)
        rows.append((edges[b], edges[b + 1], m, p, e))
    for lo, hi, m, p, e in rows:
        lo_s = f"{lo:5.2f}" if lo > 0 else "  0   "
        bar = "#" * int(round(e * 30))
        print(f"  {lo_s}-{hi:5.2f}  {m:>8}  {p:.3f}  {e:.3f}  {p - e:+.3f}  {bar}")
    print()

    bias = float(pred.mean() - emp.mean())
    print(f"ECE  = {ece:.4f}   (well calibrated < 0.03, acceptable < 0.08, needs work ≥ 0.08)")
    print(f"bias = {bias:+.4f}   (mean predicted − mean empirical; positive = scores too optimistic)")
    games = np.unique(d.game_id)
    if len(games) == 1:
        print("⚠ only one game in the data — empirical rates are unreliable; generate more games")


if __name__ == "__main__":
    main()
