#!/usr/bin/env python3
"""deploy/chk_tk_regmodel.py — test model registration."""
import subprocess
import sys
from pathlib import Path


def main():
    Path("/tmp/test_model_reg.tkn").write_text("model\n", encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_regmodel.py", "/tmp/test_model_reg.tkn",
         "--run", "test_run"],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ register_training_models failed")
        print(result.stderr)
        sys.exit(1)
    assert Path("models/manifest.json").exists()
    print("✓ test_register_training_models passed")


if __name__ == "__main__":
    main()
