#!/usr/bin/env python3
"""deploy/tk_nnuescope.py — define the scope of NNUE unit tests.

Lists which NNUE components are covered by tests and which are pending.

    python3 deploy/tk_nnuescope.py
"""
import argparse
import sys


def main():
    ap = argparse.ArgumentParser(description="NNUE test scope report.")
    args = ap.parse_args()
    print("==> NNUE test scope")
    print("    covered:  quantized_forward, arch, features, tkn reader")
    print("    pending:  packed storage, bucket calibration, export clip")
    print("✓ scope reported")


if __name__ == "__main__":
    main()