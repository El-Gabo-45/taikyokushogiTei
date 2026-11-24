#!/usr/bin/env python3
"""deploy/tk_promote.py — promote a tournament winner as the teacher model.

Copies the winning model to models/teacher.json and records the
promotion in data/teacher_history/.

    python3 deploy/tk_promote.py --winner models/champion.tkn
"""
import argparse
import json
import shutil
import sys
from datetime import datetime
from pathlib import Path


def promote(winner_path):
    winner = Path(winner_path)
    if not winner.exists():
        print(f"✗ winner model not found: {winner}", file=sys.stderr)
        sys.exit(1)

    teacher = Path("models/teacher.tkn")
    teacher.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(winner, teacher)

    history_dir = Path("data/teacher_history")
    history_dir.mkdir(parents=True, exist_ok=True)
    record = history_dir / f"{datetime.now().isoformat()}.json"
    record.write_text(json.dumps({
        "winner": str(winner),
        "promoted_to": str(teacher),
        "at": datetime.now().isoformat(),
    }) + "\n", encoding="utf-8")

    print(f"✓ teacher promoted: {winner} -> {teacher}")


def main():
    ap = argparse.ArgumentParser(description="Promote tournament winner as teacher.")
    ap.add_argument("--winner", required=True, help="winning model file")
    args = ap.parse_args()
    promote(args.winner)


if __name__ == "__main__":
    main()
