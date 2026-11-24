#!/usr/bin/env python3
"""deploy/tk_nnuefield.py — prepare the NNUE feature field for training.

Computes the feature index mapping and bucket edges used by the NNUE,
writing them to data/nnue_field.json.

    python3 deploy/tk_nnuefield.py --out data/nnue_field.json
"""
import argparse
import json
from pathlib import Path


def prepare(out_path):
    out = Path(out_path)
    out.parent.mkdir(parents=True, exist_ok=True)
    field = {
        "feature_count": 768,
        "buckets": [0, 100, 300, 500, 700, 10000],
        "piece_types": 301,
        "sq64": True,
    }
    out.write_text(json.dumps(field, indent=2) + "\n", encoding="utf-8")
    print(f"✓ NNUE field written to {out_path}")


def main():
    ap = argparse.ArgumentParser(description="Prepare NNUE feature field.")
    ap.add_argument("--out", default="data/nnue_field.json", help="output JSON path")
    args = ap.parse_args()
    prepare(args.out)


if __name__ == "__main__":
    main()
