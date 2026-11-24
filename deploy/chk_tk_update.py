#!/usr/bin/env python3
"""deploy/chk_tk_update.py — test engine update checker."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_update.py", "--check"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ engine_updates failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_engine_updates passed")


if __name__ == "__main__":
    main()
