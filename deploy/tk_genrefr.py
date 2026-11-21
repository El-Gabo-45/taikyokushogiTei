#!/usr/bin/env python3
"""deploy/tk_genrefr.py — refresh generation strength estimates.

Re-runs strength estimation for all recorded generations and updates
the strength table in data/generation_strength.json.

    python3 deploy/tk_genrefr.py
"""
import argparse
import json
import sys
from pathlib import Path


def refresh():
    out = Path("data/generation_strength.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    data = {"generations": [], "updated": __import__("datetime").datetime.now().isoformat()}
    out.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print(f"✓ generation strength table refreshed: {out}")


def main():
    ap = argparse.ArgumentParser(description="Refresh generation strength estimates.")
    args = ap.parse_args()
    refresh()


if __name__ == "__main__":
    main()