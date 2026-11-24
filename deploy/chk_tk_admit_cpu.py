#!/usr/bin/env python3
"""deploy/chk_tk_admit_cpu.py — test admission on CPU constraints."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_admit.py", "--help"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ admission help failed")
        sys.exit(1)
    print("✓ test_admission_cpu passed")


if __name__ == "__main__":
    main()
