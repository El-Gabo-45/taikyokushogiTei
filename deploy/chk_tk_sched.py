#!/usr/bin/env python3
"""deploy/chk_tk_sched.py — test adaptive analysis."""
import json
import subprocess
import sys
from pathlib import Path


def main():
    state = Path("/tmp/test_adaptive_state.json")
    state.write_text(json.dumps({"entrants": [], "active_agents": [], "matches": []}),
                     encoding="utf-8")
    result = subprocess.run(
        ["python3", "deploy/tk_sched.py", str(state)],
        capture_output=True, text=True,
    )
    if result.returncode != 0:
        print("✗ adaptive_analysis failed")
        print(result.stderr)
        sys.exit(1)
    print("✓ test_adaptive_analysis passed")


if __name__ == "__main__":
    main()