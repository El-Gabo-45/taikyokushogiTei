#!/usr/bin/env python3
"""deploy/chk_tk_mklabels.py — test training labels generation."""
import json
import subprocess
import sys
from pathlib import Path


def main():
    games_dir = Path("/tmp/test_games_labels")
    games_dir.mkdir(exist_ok=True)
    (games_dir / "g1.json").write_text(
        json.dumps({"positions": [{"fen": "start", "score": 50}], "result": 1}),
        encoding="utf-8",
    )
    out = Path("/tmp/test_labels_out.jsonl")
    result = subprocess.run(
        ["python3", "deploy/tk_mklabels.py", str(games_dir), "--out", str(out)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ training_labels failed")
        print(result.stderr)
        sys.exit(1)
    assert out.exists()
    print("✓ test_training_labels passed")


if __name__ == "__main__":
    main()
