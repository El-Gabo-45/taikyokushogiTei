#!/usr/bin/env python3
"""Dataset coverage report — what does the self-play data actually cover?

    python3 deploy/tk_survey.py data/derived/positions
    python3 deploy/tk_survey.py data/derived/positions --min-share 2.0

Reads featurized position rows (JSONL) and reports coverage along the
dimensions that matter for a large board with many piece types:

  * piece-count buckets (opening / midgame / endgame)
  * game-phase buckets (ply since start)
  * side to move balance
  * result distribution (side-to-move POV)
  * game length (samples per game)
  * piece-type frequency (most common / rarest)
  * score |x| distribution (median / p90 / max, mate-like labels)

Under-represented slices are flagged with ⚠. Exit code 0 as long as the
data could be read — analyzer, not a gate.
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np


def load_rows(paths):
    rows = []
    for p in paths:
        p = Path(p)
        if p.is_dir():
            for f in sorted(p.glob("*.jsonl")):
                rows.extend(_read_jsonl(f))
        else:
            rows.extend(_read_jsonl(p))
    if not rows:
        raise SystemExit("coverage: no position rows found")
    return rows


def _read_jsonl(path):
    out = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def pct(x):
    return f"{100.0 * x:6.1f}%"


def bucketize(values, edges):
    vals = np.array(values)
    counts = []
    for lo, hi, label in edges:
        counts.append((label, int(np.sum((vals > lo) & (vals <= hi)))))
    return counts


def piece_count_buckets(_):
    return [
        (0, 100, "   1-100   (late endgame)"),
        (100, 300, "101-300  (endgame)"),
        (300, 500, "301-500  (midgame)"),
        (500, 700, "501-700  (early, captures started)"),
        (700, 10_000, "701+     (opening, barely touched)"),
    ]


def phase_buckets(_):
    return [
        (0, 25, "ply   1-25"),
        (25, 50, "ply  26-50"),
        (50, 100, "ply  51-100"),
        (100, 250, "ply 101-250"),
        (250, 10_000, "ply 251+"),
    ]


def report(name, counts, total, min_share):
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
    ap = argparse.ArgumentParser(description="Coverage report over featurized position rows.")
    ap.add_argument("paths", nargs="+", help="JSONL position files or directories")
    ap.add_argument("--min-share", type=float, default=1.0,
                    help="warn when a bucket holds less than this %% of samples (default 1.0)")
    ap.add_argument("--top-types", type=int, default=10,
                    help="piece types listed at each extreme (default 10)")
    args = ap.parse_args()

    rows = load_rows(args.paths)
    n = len(rows)
    print("=== Dataset coverage ===")
    print(f"samples: {n}")
    print()

    npc = [int(r.get("npieces", r.get("piece_count", 0))) for r in rows]
    warn = []
    warn += report("piece count (pieces on board)",
                   bucketize(npc, piece_count_buckets(npc)), n, args.min_share)

    ply = [int(r.get("ply", 0)) for r in rows]
    warn += report("game phase (ply since start)",
                   bucketize(ply, phase_buckets(ply)), n, args.min_share)

    stm_black = sum(1 for r in rows if int(r.get("stm", 0)) == 0)
    print("side to move")
    print(f"  black    {stm_black:>8}  {pct(stm_black / n)}")
    print(f"  white    {n - stm_black:>8}  {pct((n - stm_black) / n)}")
    print()

    results = [int(r.get("result", 0)) for r in rows]
    wins = sum(1 for x in results if x == 1)
    draws = sum(1 for x in results if x == 0)
    losses = sum(1 for x in results if x == -1)
    print("result (from side to move's point of view)")
    print(f"  win      {wins:>8}  {pct(wins / n)}")
    print(f"  draw     {draws:>8}  {pct(draws / n)}")
    print(f"  loss     {losses:>8}  {pct(losses / n)}")
    print()

    game_ids = {}
    for i, r in enumerate(rows):
        gid = r.get("game_id", i)
        game_ids.setdefault(gid, []).append(i)
    lengths = [len(v) for v in game_ids.values()]
    print(f"games: {len(lengths)}")
    print("game length (samples per game)")
    for name, v in (("min", min(lengths)), ("median", float(np.median(lengths))),
                    ("p90", float(np.percentile(lengths, 90))), ("max", max(lengths))):
        print(f"  {name:<7} {int(v):>8}")
    print()

    pt_counts = {}
    for r in rows:
        pt = r.get("pt", r.get("piece_type"))
        if pt is not None:
            pt_counts[int(pt)] = pt_counts.get(int(pt), 0) + 1
    seen = sorted(pt_counts)
    print(f"piece types: {len(seen)} seen")
    if seen:
        top = sorted(seen, key=lambda t: -pt_counts[t])[: args.top_types]
        rare = sorted(seen, key=lambda t: pt_counts[t])[: args.top_types]
        print("  most common:  " + ", ".join(f"#{t}×{pt_counts[t]}" for t in top))
        print("  rarest seen:  " + ", ".join(f"#{t}×{pt_counts[t]}" for t in rare))

    scores = [abs(float(r.get("score", 0))) for r in rows if "score" in r]
    if scores:
        a = np.array(scores)
        print("score |x| (engine units)")
        for name, v in (("median", np.median(a)), ("p90", np.percentile(a, 90)), ("max", a.max())):
            print(f"  {name:<7} {int(v):>8}")
        extreme = int(np.sum(a >= 100_000))
        if extreme:
            print(f"  ⚠ {extreme} mate-like labels (|score| ≥ 100k)")

    if warn:
        print()
        print("⚠ under-represented slices:", "; ".join(warn))
        print("  generate more games that reach those phases.")


if __name__ == "__main__":
    main()