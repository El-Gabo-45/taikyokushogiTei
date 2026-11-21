#!/usr/bin/env python3
"""deploy/tk_mklabels.py — generate training labels from self-play games.

Reads saved game JSON records and writes labeled position samples
(position, search score, result) suitable for NNUE training.

    python3 deploy/tk_mklabels.py data/raw/games --out data/derived/labels.jsonl
"""
import argparse
import json
import sys
from pathlib import Path


def extract_labels(game_record):
    """Yield (position_fen, score, result) tuples from a game record."""
    positions = game_record.get("positions", [])
    result = game_record.get("result", 0)
    for entry in positions:
        fen = entry.get("fen")
        score = entry.get("score", 0)
        if fen:
            yield fen, score, result


def main():
    ap = argparse.ArgumentParser(description="Generate training labels from game records.")
    ap.add_argument("games", nargs="+", help="game JSON files or directories")
    ap.add_argument("--out", required=True, help="output JSONL labels file")
    args = ap.parse_args()

    total = 0
    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    with open(out_path, "w", encoding="utf-8") as out:
        for p in args.games:
            p = Path(p)
            files = [p] if p.is_file() else sorted(p.glob("*.json"))
            for f in files:
                try:
                    rec = json.loads(f.read_text(encoding="utf-8"))
                except Exception as e:
                    print(f"    skip {f}: {e}", file=sys.stderr)
                    continue
                for fen, score, result in extract_labels(rec):
                    out.write(json.dumps({
                        "fen": fen,
                        "score": float(score),
                        "result": float(result),
                    }) + "\n")
                    total += 1

    print(f"✓ {total} labels written to {args.out}")


if __name__ == "__main__":
    main()