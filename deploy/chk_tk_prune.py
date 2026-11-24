#!/usr/bin/env python3
"""deploy/chk_tk_prune.py — test run artifact pruning."""
import subprocess
import sys
from pathlib import Path


def main():
    run_dir = Path("/tmp/test_prune_runs")
    run_dir.mkdir(exist_ok=True)
    for i in range(5):
        (run_dir / f"run_{i}").mkdir(exist_ok=True)
    result = subprocess.run(
        ["python3", "deploy/tk_prune.py",
         "--run-dir", str(run_dir), "--keep", "2", "--older-than", "0"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ prune_run_artifacts failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_prune_run_artifacts passed")


if __name__ == "__main__":
    main()
