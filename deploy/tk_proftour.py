#!/usr/bin/env python3
"""deploy/tk_proftour.py — profile a tournament run for bottlenecks.

Runs the engine under a profiler during tournament play and writes
a flamegraph-compatible profile.

    python3 deploy/tk_proftour.py --games 64 --out data/profiles
"""
import argparse
from pathlib import Path


def profile(games, out_dir):
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    print(f"==> Profiling tournament: {games} games -> {out_dir}")
    print("    (profiler integration stub — implement with cargo-flamegraph/perf)")
    profile_file = out / "tournament.profile"
    profile_file.write_text("# profile stub\n", encoding="utf-8")
    print(f"✓ profile written: {profile_file}")


def main():
    ap = argparse.ArgumentParser(description="Profile a tournament run.")
    ap.add_argument("--games", type=int, default=64, help="number of games")
    ap.add_argument("--out", default="data/profiles", help="output directory")
    args = ap.parse_args()
    profile(args.games, args.out)


if __name__ == "__main__":
    main()
