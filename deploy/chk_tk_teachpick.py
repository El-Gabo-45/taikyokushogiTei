#!/usr/bin/env python3
"""deploy/chk_tk_teachpick.py — test conditional teacher selection."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_cond.tkn").write_text("cond\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_teachpick.py",
         "--phase", "opening", "--model", "/tmp/test_cond.tkn"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ conditional_teachers failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_conditional_teachers passed")


if __name__ == "__main__":
    main()
