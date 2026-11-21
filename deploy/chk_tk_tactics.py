#!/usr/bin/env python3
"""deploy/chk_tk_tactics.py — test tactical moment mining."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_tactics.py", "/tmp",
         "--out", "/tmp/test_tactical_out"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ mine_tactical_moments failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_mine_tactical_moments passed")


if __name__ == "__main__":
    main()