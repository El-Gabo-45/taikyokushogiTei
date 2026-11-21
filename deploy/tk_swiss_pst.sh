#!/usr/bin/env bash
# deploy/tk_swiss_pst.sh — pull, build, and run a PST Swiss tournament
#
#   bash deploy/tk_swiss_pst.sh --rounds 5
set -euo pipefail

cd "$(dirname "$0")/.."

ROUNDS=5
while [ $# -gt 0 ]; do
    case "$1" in
        --rounds) ROUNDS="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

echo "==> PST Swiss tournament: $ROUNDS rounds"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ PST Swiss ready (run with cargo run --release -- swiss --rounds $ROUNDS)"