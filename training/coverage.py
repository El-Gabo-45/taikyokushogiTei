#!/usr/bin/env python3
"""Dataset Coverage Analyzer — what does the training data actually cover?

    python3 training/coverage.py data                      # TKD1 dir or files
    python3 training/coverage.py data --min-share 2.0

Reads the same "TKD1" files the trainer consumes (`dataset.load`) and reports
coverage along the dimensions that matter for a 36×36 board with 804 initial
pieces: piece-count buckets, game phase, side to move, results, game length,
piece-type frequencies and the score distribution that drives the sigmoid
scale. Under-represented slices are flagged with ⚠ so you know *which* games
to generate next (more endgames? more promotions? more captures?).

Exit code is 0 as long as the data could be read — this is an analyzer, not a
gate; the CI gate for label units is `check_label_units.py`.
"""

import argparse

import dataset
import numpy as np


def pct(x):
    """Fixed-width percentage string."""
    return f"{100.0 * x:6.1f}%"


def bucketize(values, edges, labels):
    """Counts of `values` falling in each (lo, hi] bucket with a label."""
    counts = []
    for lo, hi, label in edges:
        counts.append((label, int(np.sum((values > lo) & (values <= hi)))))
    return counts


def piece_count_buckets(npc):
    """Bucket edges for pieces-on-board. 804 is the initial total."""
    return [
        (0, 200, "  1-200   (late endgame)"),
        (200, 400, "201-400  (endgame)"),
        (400, 600, "401-600  (midgame)"),
        (600, 704, "601-704  (early, captures started)"),
        (704, 10_000, "705-804  (opening, barely touched)"),
    ]


def phase_buckets(ply):
    """Bucket edges for the ply since game start."""
    return [
        (0, 25, "ply   1-25"),
        (25, 50, "ply  26-50"),
        (50, 100, "ply  51-100"),
        (100, 250, "ply 101-250"),
        (250, 10_000, "ply 251+"),
    ]


def report(name, counts, total, min_share):
    """Print one coverage table and collect the under-share labels."""
    print(name)
    warn = []
    for label, count in counts:
        share = count / max(total, 1)
        flag = ""
        if share < min_share / 100.0:
            flag = "  ⚠ under-represented"
            warn.append(label.strip())
        print(f"  {label}  {count:>8}  {pct(share)}{flag}")
    print()
    return warn


def main():
    ap = argparse.ArgumentParser(
        description="Dataset coverage report over TKD1 training files."
    )
    ap.add_argument("paths", nargs="+", help="TKD1 files or directories of samples_*.bin")
    ap.add_argument(
        "--min-share",
        type=float,
        default=1.0,
        help="warn when a bucket holds less than this %% of the samples (default 1.0)",
    )
    ap.add_argument("--top-types", type=int, default=10, help="piece types listed at each extreme")
    args = ap.parse_args()

    d = dataset.load(args.paths)
    n = len(d)
    npc = d.npieces
    games = np.unique(d.game_id)
    print("=== Dataset coverage ===")
    print(f"samples: {n}   games: {len(games)}   pieces: {int(npc.sum())}")
    print()

    warn = []
    warn += report("piece count (pieces on the board)", bucketize(npc, piece_count_buckets(npc), None), n, args.min_share)
    warn += report("game phase (ply since game start)", bucketize(d.ply, phase_buckets(d.ply), None), n, args.min_share)

    # Side to move.
    black = int(np.sum(d.stm == 0))
    print("side to move")
    print(f"  black    {black:>8}  {pct(black / n)}")
    print(f"  white    {n - black:>8}  {pct((n - black) / n)}")
    print()

    # Results from the side to move's point of view.
    wins = int(np.sum(d.result == 1))
    draws = int(np.sum(d.result == 0))
    losses = int(np.sum(d.result == -1))
    print("result (from the side to move's point of view)")
    print(f"  win      {wins:>8}  {pct(wins / n)}")
    print(f"  draw     {draws:>8}  {pct(draws / n)}")
    print(f"  loss     {losses:>8}  {pct(losses / n)}")
    print()

    # Game length in samples (≈ moves − opening plies, one sample per ply).
    samples_per_game = np.diff(np.concatenate([[0], np.searchsorted(np.sort(d.game_id), np.unique(d.game_id), side="right")]))
    print("game length (samples per game; ≈ moves − opening plies)")
    for name, v in (("min", samples_per_game.min()), ("median", np.median(samples_per_game)), ("p90", np.percentile(samples_per_game, 90)), ("max", samples_per_game.max())):
        print(f"  {name:<7} {int(v):>8}")
    print()

    # Piece-type frequencies over the 301 types (9-bit cell field).
    counts = np.bincount(d.pt, minlength=302)[1:]
    seen = np.flatnonzero(counts)
    print(f"piece types: {len(seen)} of 301 seen")
    top = sorted(range(1, 302), key=lambda t: -counts[t - 1])[: args.top_types]
    rare = sorted((t for t in range(1, 302) if counts[t - 1] > 0), key=lambda t: counts[t - 1])[: args.top_types]
    print("  most common:  " + ", ".join(f"#{t}×{int(counts[t - 1])}" for t in top))
    print("  rarest seen:  " + ", ".join(f"#{t}×{int(counts[t - 1])}" for t in rare))
    if len(seen) < 301:
        print(f"  ⚠ {301 - len(seen)} of 301 piece types never appear — cross-check with `nnue classes`")
    print()

    # Score distribution: drives --scale in train.py (median |score| → 75 %).
    a = np.abs(d.score)
    print("score |x| (engine units; a pawn is 500)")
    for name, v in (("median", np.median(a)), ("p90", np.percentile(a, 90)), ("max", a.max())):
        print(f"  {name:<7} {int(v):>8}")
    extreme = int(np.sum(a >= 100_000))
    if extreme:
        print(f"  ⚠ {extreme} mate-like labels (|score| ≥ 100k) — train.py drops these by default")

    if warn:
        print()
        print("⚠ under-represented slices:", "; ".join(warn))
        print("  generate more games that reach those phases (more --max-moves, deeper selfplay, more captures).")


if __name__ == "__main__":
    main()
