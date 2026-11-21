#!/usr/bin/env python3
"""deploy/tk_sched.py — analyze adaptive scheduler behavior.

Reads scheduler state and reports on allocation, exposure, and
retirement decisions for debugging generation transitions.

    python3 deploy/tk_sched.py state.json
"""
import argparse
import json
import sys
from pathlib import Path


def analyze(state_path):
    state = Path(state_path)
    if not state.exists():
        print(f"✗ state file not found: {state}", file=sys.stderr)
        sys.exit(1)
    data = json.loads(state.read_text(encoding="utf-8"))
    print(f"==> Adaptive analysis: {state_path}")
    print(f"    entrants: {len(data.get('entrants', []))}")
    print(f"    active:   {len(data.get('active_agents', []))}")
    print(f"    inactive: {len(data.get('inactive_agents', []))}")
    print(f"    matches:  {len(data.get('matches', []))}")
    print("✓ analysis complete")


def main():
    ap = argparse.ArgumentParser(description="Adaptive scheduler analysis.")
    ap.add_argument("state", help="scheduler state JSON file")
    args = ap.parse_args()
    analyze(args.state)


if __name__ == "__main__":
    main()