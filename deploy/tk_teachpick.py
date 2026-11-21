#!/usr/bin/env python3
"""deploy/tk_teachpick.py — select a teacher based on conditions.

Chooses which teacher model to use depending on game phase, rating,
or experiment configuration.

    python3 deploy/tk_teachpick.py --phase opening --model models/opening.tkn
"""
import argparse
import json
import sys
from pathlib import Path


def select(phase, model_path):
    model = Path(model_path)
    if not model.exists():
        print(f"✗ model not found: {model}", file=sys.stderr)
        sys.exit(1)
    config = Path("data/teacher_config.json")
    config.parent.mkdir(parents=True, exist_ok=True)
    data = {}
    if config.exists():
        try:
            data = json.loads(config.read_text(encoding="utf-8"))
        except Exception:
            data = {}
    data[phase] = str(model)
    config.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"✓ teacher for phase '{phase}': {model}")


def main():
    ap = argparse.ArgumentParser(description="Conditional teacher selector.")
    ap.add_argument("--phase", required=True, help="game phase or condition")
    ap.add_argument("--model", required=True, help="teacher model file")
    args = ap.parse_args()
    select(args.phase, args.model)


if __name__ == "__main__":
    main()