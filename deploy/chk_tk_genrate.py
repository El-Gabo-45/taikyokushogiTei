#!/usr/bin/env python3
"""deploy/chk_tk_genrate.py — test generation strength estimation."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_champ2.tkn").write_text("champ\n", encoding="utf-8")
    Path("/tmp/test_prior2.tkn").write_text("prior\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_genrate.py",
         "--champion", "/tmp/test_champ2.tkn",
         "--prior", "/tmp/test_prior2.tkn",
         "--games", "10"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ generation_strength failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_generation_strength passed")


if __name__ == "__main__":
    main()