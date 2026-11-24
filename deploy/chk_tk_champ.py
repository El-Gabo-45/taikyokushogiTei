#!/usr/bin/env python3
"""deploy/chk_tk_champ.py — test champion teacher promotion."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_champ.tkn").write_text("champ\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_champ.py", "--promote", "/tmp/test_champ.tkn"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ champion_teacher failed")
        print(result.stderr)
        sys.exit(1)
    assert Path("models/champion.tkn").exists()
    print("✓ test_champion_teacher passed")


if __name__ == "__main__":
    main()
