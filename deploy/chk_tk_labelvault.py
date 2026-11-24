#!/usr/bin/env python3
"""deploy/chk_tk_labelvault.py — test content-addressed label storage."""
import subprocess
import sys
from pathlib import Path


def main():
    src = Path("/tmp/test_labels_src.jsonl")
    src.write_text('{"score": 100}\n{"score": 200}\n', encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_labelvault.py", "store", str(src)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ label_storage store failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_label_storage passed")


if __name__ == "__main__":
    main()
