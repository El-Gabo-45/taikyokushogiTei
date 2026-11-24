#!/usr/bin/env python3
"""deploy/chk_tk_pgocorpus.py — test PGO corpus preparation."""
import subprocess
import sys
from pathlib import Path


def main():
    out = Path("/tmp/test_pgo_corpus")
    result = subprocess.run(
        ["python3", "deploy/tk_pgocorpus.py", "--games", "4", "--out", str(out)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ prepare_pgo_corpus failed")
        print(result.stderr)
        sys.exit(1)
    assert (out / "corpus_manifest.json").exists()
    print("✓ test_prepare_pgo_corpus passed")


if __name__ == "__main__":
    main()
