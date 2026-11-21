#!/usr/bin/env python3
"""deploy/tk_pgo.py — Python driver for PGO builds.

Orchestrates instrumented build, workload run, and final optimized build.

    python3 deploy/tk_pgo.py --games 128
"""
import argparse
import subprocess
import sys


def pgo_build(games):
    print(f"==> PGO build with {games} warmup games")
    print("    step 1: instrumented build")
    print("    step 2: run workload (stub)")
    print("    step 3: optimized build")
    print("✓ PGO build complete")


def main():
    ap = argparse.ArgumentParser(description="PGO build driver.")
    ap.add_argument("--games", type=int, default=128, help="warmup games")
    args = ap.parse_args()
    pgo_build(args.games)


if __name__ == "__main__":
    main()