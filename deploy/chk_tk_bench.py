#!/usr/bin/env python3
"""deploy/chk_tk_bench.py — test search benchmark."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_bench.py", "--depths", "6,8"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ benchmark_search failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_benchmark_search passed")


if __name__ == "__main__":
    main()
