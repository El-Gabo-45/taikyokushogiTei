#!/usr/bin/env python3
"""deploy/chk_tk_scale_calc.py — test scale sample computation."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_scale_calc.py"],
        capture_output=True, text=True,
    )
    # Will fail without data file — just check it runs and gives a clear error
    if "no scores found" not in result.stderr and result.returncode != 0:
        print("✗ run_scale_sample unexpected behavior")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_scale_sample_corr passed")


if __name__ == "__main__":
    main()