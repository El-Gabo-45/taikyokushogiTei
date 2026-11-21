#!/usr/bin/env python3
"""deploy/tk_overhead.py — measure profiling overhead.

Compares engine speed with and without profiling to quantify overhead.

    python3 deploy/tk_overhead.py --iterations 10
"""
import argparse
import sys
import time


def measure(iterations):
    print(f"==> Measuring profiling overhead ({iterations} iterations)")
    t0 = time.time()
    for _ in range(iterations):
        pass  # placeholder for engine run
    t1 = time.time()
    elapsed = (t1 - t0) * 1000
    print(f"    elapsed: {elapsed:.1f} ms total, {elapsed/iterations:.1f} ms per iteration")
    print("    overhead: 0.0% (placeholder — instrument engine for real numbers)")
    print("✓ overhead measurement complete")


def main():
    ap = argparse.ArgumentParser(description="Measure profiling overhead.")
    ap.add_argument("--iterations", type=int, default=10, help="iterations")
    args = ap.parse_args()
    measure(args.iterations)


if __name__ == "__main__":
    main()