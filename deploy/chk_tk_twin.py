#!/usr/bin/env python3
"""deploy/chk_tk_twin.py — test dual label sidecar."""
import subprocess
import sys


def main():
    result = subprocess.run(
        ["python3", "deploy/tk_twin.py", "/tmp"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ dual_label_sidecar failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_dual_label_sidecar passed")


if __name__ == "__main__":
    main()