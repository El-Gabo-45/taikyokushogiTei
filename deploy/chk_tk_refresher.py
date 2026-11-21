#!/usr/bin/env python3
"""deploy/chk_tk_refresher.py — test teacher refresh."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_refresher.py", "--games", "/tmp", "--iterations", "1"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ teacher_refresh failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_teacher_refresh passed")


if __name__ == "__main__":
    main()