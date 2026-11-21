#!/usr/bin/env python3
"""deploy/tk_genrate.py — estimate strength of a training generation.

Compares generation champion against prior generation to estimate
Elo improvement.

    python3 deploy/tk_genrate.py --champion models/champion.tkn \
        --prior models/prior_champion.tkn --games 200
"""
import argparse
import sys
from pathlib import Path


def estimate(champion, prior, games):
    ch = Path(champion)
    pr = Path(prior)
    for p in (ch, pr):
        if not p.exists():
            print(f"✗ model not found: {p}", file=sys.stderr)
            sys.exit(1)
    print(f"==> Generation strength: {champion} vs {prior} ({games} games)")
    print("    (match simulation stub — implement engine comparison)")
    print(f"✓ estimated strength delta: +0.0 Elo (placeholder)")


def main():
    ap = argparse.ArgumentParser(description="Estimate generation strength.")
    ap.add_argument("--champion", required=True)
    ap.add_argument("--prior", required=True)
    ap.add_argument("--games", type=int, default=200)
    args = ap.parse_args()
    estimate(args.champion, args.prior, args.games)


if __name__ == "__main__":
    main()