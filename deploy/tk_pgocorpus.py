#!/usr/bin/env python3
"""deploy/tk_pgocorpus.py — prepare a PGO training corpus.

Plays a set of games designed to exercise hot code paths for
profile-guided optimization.

    python3 deploy/tk_pgocorpus.py --games 256 --out data/pgo_corpus
"""
import argparse
import sys
from pathlib import Path


def prepare(count, out_dir):
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    print(f"==> Preparing PGO corpus: {count} games -> {out_dir}")
    print("    (game generation stub — implement self-play here)")
    manifest = out / "corpus_manifest.json"
    manifest.write_text(
        __import__("json").dumps({"games": count, "created": __import__("datetime").datetime.now().isoformat()})
        + "\n", encoding="utf-8")
    print(f"✓ PGO corpus ready: {out}")


def main():
    ap = argparse.ArgumentParser(description="Prepare PGO training corpus.")
    ap.add_argument("--games", type=int, default=256, help="number of games")
    ap.add_argument("--out", default="data/pgo_corpus", help="output directory")
    args = ap.parse_args()
    prepare(args.games, args.out)


if __name__ == "__main__":
    main()