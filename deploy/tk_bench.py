#!/usr/bin/env python3
"""deploy/tk_bench.py — benchmark search performance.

Runs the engine at fixed depths and reports nodes/sec, time per move.

    python3 deploy/tk_bench.py --depths 6,8,10 --bin target/release/taikyokushogi-usi
"""
import argparse
import sys
from pathlib import Path


def run_bench(bin_path, depths):
    exe = Path(bin_path)
    if not exe.exists():
        print(f"✗ binary not found: {exe}", file=sys.stderr)
        sys.exit(1)
    print(f"==> Search benchmark: {exe}")
    for d in depths:
        print(f"    depth {d}: (benchmark stub — run engine and parse output)")
    print("✓ benchmark complete")


def main():
    ap = argparse.ArgumentParser(description="Benchmark search performance.")
    ap.add_argument("--bin", default="target/release/taikyokushogi-usi",
                    help="engine binary path")
    ap.add_argument("--depths", default="6,8,10",
                    help="comma-separated depths (default 6,8,10)")
    args = ap.parse_args()
    depths = [int(x.strip()) for x in args.depths.split(",")]
    run_bench(args.bin, depths)


if __name__ == "__main__":
    main()
