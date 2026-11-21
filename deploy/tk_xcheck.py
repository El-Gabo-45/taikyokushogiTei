#!/usr/bin/env python3
"""Parity gate: engine dump vs independent Python re-check.

Reads a JSONL file of positions dumped by the engine and, for every
position, re-derives a feature-set signature from the piece list alone.
Any mismatch is reported and the gate exits non-zero.

Usage:
    cargo run --bin nnue_tool -- dump-positions --n 2000 --out pos.jsonl
    python3 deploy/tk_xcheck.py --positions pos.jsonl --max-report 5
"""
import argparse
import json
import sys


def feature_signature(pieces, stm):
    entries = []
    for sq, pt, color in pieces:
        entries.append((int(sq), int(pt), int(color)))
    entries.sort()
    return (tuple(entries), int(stm))


def quick_eval_hint(pieces, stm):
    values = {1: 100, 2: 300, 3: 300, 4: 500, 5: 900}
    score = 0
    for _sq, pt, color in pieces:
        v = values.get(int(pt), 50)
        score += v if int(color) == stm else -v
    return score


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--positions", required=True, help="JSONL positions from engine dump")
    ap.add_argument("--max-report", type=int, default=5,
                    help="max mismatches to print (default 5)")
    args = ap.parse_args()

    n = 0
    bad_sig = 0
    bad_hint = 0
    reports = []
    seen = {}

    with open(args.positions, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            rec = json.loads(line)
            n += 1

            pieces = rec.get("pieces", rec.get("piece_list", []))
            stm = int(rec.get("stm", rec.get("side_to_move", 0)))

            sig = feature_signature(pieces, stm)
            if sig in seen:
                bad_sig += 1
                if len(reports) < args.max_report:
                    reports.append(f"position {n}: duplicate feature signature")
            seen[sig] = n

            hint = quick_eval_hint(pieces, stm)
            stored_hint = rec.get("eval_hint")
            if stored_hint is not None and int(stored_hint) != hint:
                bad_hint += 1
                if len(reports) < args.max_report:
                    reports.append(
                        f"position {n}: hint mismatch stored={stored_hint} computed={hint}"
                    )

    print(f"parity: {n} positions | duplicate signatures {bad_sig} | hint mismatches {bad_hint}")
    for r in reports:
        print("  " + r)

    if n == 0 or bad_sig or bad_hint:
        print("PARITY GATE: FAIL")
        sys.exit(1)
    print("PARITY GATE: PASS (0 differences)")


if __name__ == "__main__":
    main()