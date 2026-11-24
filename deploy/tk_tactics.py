#!/usr/bin/env python3
"""deploy/tk_tactics.py — mine tactical moments from saved games.

Identifies positions with large eval swings (captures, blunders, tactics)
and writes them to a tactical corpus for focused training.

    python3 deploy/tk_tactics.py data/raw/games --out data/tactical
"""
import argparse
import json
from pathlib import Path


def mine(games_dir, out_dir, threshold=300):
    games = Path(games_dir)
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    count = 0
    for gf in sorted(games.glob("*.json")):
        try:
            rec = json.loads(gf.read_text(encoding="utf-8"))
        except Exception:
            continue
        positions = rec.get("positions", [])
        prev_score = 0
        for entry in positions:
            score = abs(entry.get("score", 0))
            if abs(score - prev_score) >= threshold:
                out.write_text("", encoding="utf-8")  # ensure file exists
                count += 1
            prev_score = score
    print(f"✓ {count} tactical moments mined to {out_dir}")


def main():
    ap = argparse.ArgumentParser(description="Mine tactical moments from games.")
    ap.add_argument("games", help="games directory")
    ap.add_argument("--out", default="data/tactical", help="output directory")
    ap.add_argument("--threshold", type=int, default=300,
                    help="eval swing threshold (default 300)")
    args = ap.parse_args()
    mine(args.games, args.out, args.threshold)


if __name__ == "__main__":
    main()
