#!/usr/bin/env python3
"""Parity gate: Rust engine vs the integer Python reference.

  cargo run --release --bin nnue -- dump-positions --net NET --n 2000 --out pos.jsonl
  python training/parity_gate.py --net NET --positions pos.jsonl

For every dumped position the Python reference recomputes, from the piece
list alone, the feature indices of both perspectives, the output bucket and
the evaluation. Any difference (even 1) fails with exit code 1.
"""

import argparse
import json
import sys

import numpy as np

import tkn
from arch import Arch
from quantized_forward import evaluate


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--net", required=True)
    ap.add_argument("--positions", required=True)
    ap.add_argument("--max-report", type=int, default=5)
    args = ap.parse_args()

    arch = Arch()
    net = tkn.read(args.net, arch)
    n = bad_feat = bad_bucket = bad_eval = 0
    reports = []
    with open(args.positions) as f:
        for line in f:
            rec = json.loads(line)
            pcs = np.array(rec["pieces"], dtype=np.int64)
            r = evaluate(arch, net, pcs[:, 0], pcs[:, 1], pcs[:, 2], rec["stm"])
            n += 1
            ok = True
            for p in (0, 1):
                if not np.array_equal(r["features"][p], np.array(rec["features"][p])):
                    bad_feat += 1
                    ok = False
                    break
            if r["bucket"] != rec["bucket"]:
                bad_bucket += 1
                ok = False
            if r["eval"] != rec["eval"]:
                bad_eval += 1
                ok = False
            if not ok and len(reports) < args.max_report:
                reports.append(f"position {n}: rust eval {rec['eval']} bucket {rec['bucket']} | python eval {r['eval']} bucket {r['bucket']}")
    print(f"parity: {n} positions | feature mismatches {bad_feat} | bucket mismatches {bad_bucket} | eval mismatches {bad_eval}")
    for r in reports:
        print("  " + r)
    if n == 0 or bad_feat or bad_bucket or bad_eval:
        print("PARITY GATE: FAIL")
        sys.exit(1)
    print("PARITY GATE: PASS (0 differences)")


if __name__ == "__main__":
    main()
