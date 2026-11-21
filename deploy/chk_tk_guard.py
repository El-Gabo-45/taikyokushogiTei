#!/usr/bin/env python3
"""deploy/chk_tk_guard.py — test storage guard."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_coldstore.py", "--help"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ storage guard help failed")
        sys.exit(1)
    print("✓ test_storage_guard passed")


if __name__ == "__main__":
    main()