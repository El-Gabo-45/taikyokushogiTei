#!/usr/bin/env python3
"""deploy/test_v4_training.py — test v4 training pipeline."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_v4_data").mkdir(exist_ok=True)
    (Path("/tmp/test_v4_data") / "pos1.jsonl").write_text('{"score": 10}\n', encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_v4.py",
         "--data", "/tmp/test_v4_data",
         "--out", "/tmp/test_v4_out.json"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ v4_training failed")
        print(result.stderr)
        sys.exit(1)
    assert Path("/tmp/test_v4_out.json").exists()
    print("✓ test_v4_training passed")


if __name__ == "__main__":
    main()