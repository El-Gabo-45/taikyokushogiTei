#!/usr/bin/env python3
"""deploy/chk_tk_promote.py — test winner teacher promotion."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_winner.tkn").write_text("winner\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_promote.py", "--winner", "/tmp/test_winner.tkn"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ winner_teacher failed")
        print(result.stderr)
        sys.exit(1)
    assert Path("models/teacher.tkn").exists()
    print("✓ test_winner_teacher passed")


if __name__ == "__main__":
    main()
