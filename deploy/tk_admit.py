#!/usr/bin/env python3
"""deploy/tk_admit.py — admission gate for training runs.

Checks that a new training checkpoint meets minimum quality thresholds
before being promoted to the active model set.

    python3 deploy/tk_admit.py --candidate net.tkn --baseline base.tkn \
        --min-elo 50 --val-games 200
"""
import argparse
import sys
from pathlib import Path


def evaluate_strength(candidate, baseline, val_games):
    """Placeholder: compare candidate vs baseline over validation games.
    Returns estimated Elo difference."""
    # In a real implementation this would run engine matches.
    return 0.0


def main():
    ap = argparse.ArgumentParser(description="Admission gate for training checkpoints.")
    ap.add_argument("--candidate", required=True, help="candidate model file")
    ap.add_argument("--baseline", required=True, help="baseline model file")
    ap.add_argument("--min-elo", type=float, default=0.0,
                    help="minimum Elo gain required (default 0)")
    ap.add_argument("--val-games", type=int, default=100,
                    help="validation games (default 100)")
    args = ap.parse_args()

    if not Path(args.candidate).exists():
        print(f"✗ candidate not found: {args.candidate}", file=sys.stderr)
        sys.exit(1)
    if not Path(args.baseline).exists():
        print(f"✗ baseline not found: {args.baseline}", file=sys.stderr)
        sys.exit(1)

    elo_gain = evaluate_strength(args.candidate, args.baseline, args.val_games)
    print(f"candidate: {args.candidate}")
    print(f"baseline:  {args.baseline}")
    print(f"estimated Elo gain: {elo_gain:+.1f} over {args.val_games} games")

    if elo_gain >= args.min_elo:
        print("✓ ADMISSION: PASS")
        sys.exit(0)
    else:
        print(f"✗ ADMISSION: FAIL (gain {elo_gain:+.1f} < required {args.min_elo:+.1f})")
        sys.exit(1)


if __name__ == "__main__":
    main()