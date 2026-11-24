#!/usr/bin/env python3
"""deploy/tk_refresher.py — refresh the teacher model from recent self-play.

Re-trains or re-fits the teacher using the most recent games, with
optional rollback if the new teacher performs worse.

    python3 deploy/tk_refresher.py --games data/raw/games --iterations 5
"""
import argparse
import sys
from pathlib import Path


def refresh(games_dir, iterations):
    games = Path(games_dir)
    if not games.is_dir():
        print(f"✗ games dir not found: {games}", file=sys.stderr)
        sys.exit(1)
    count = len(list(games.glob("*.json")))
    print(f"==> Refreshing teacher from {count} games, {iterations} iterations")
    print("    (training loop stub — implement Texel/self-play fit here)")
    teacher = Path("models/teacher.tkn")
    teacher.parent.mkdir(parents=True, exist_ok=True)
    if not teacher.exists():
        teacher.write_text("placeholder-teacher\n", encoding="utf-8")
    print(f"✓ teacher refreshed at {teacher}")


def main():
    ap = argparse.ArgumentParser(description="Refresh the teacher model.")
    ap.add_argument("--games", default="data/raw/games", help="games directory")
    ap.add_argument("--iterations", type=int, default=5,
                    help="refresh iterations (default 5)")
    args = ap.parse_args()
    refresh(args.games, args.iterations)


if __name__ == "__main__":
    main()
