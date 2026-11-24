#!/usr/bin/env python3
"""deploy/chk_tk_admit.py — test training admission gate."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_candidate.tkn").write_text("candidate\n", encoding="utf-8")
    Path("/tmp/test_baseline.tkn").write_text("baseline\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_admit.py",
         "--candidate", "/tmp/test_candidate.tkn",
         "--baseline", "/tmp/test_baseline.tkn",
         "--min-elo", "0"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ training_admission failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_training_admission passed")


if __name__ == "__main__":
    main()
