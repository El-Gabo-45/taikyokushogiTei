#!/usr/bin/env bash
# deploy/tk_tourney_start.sh — launch a self-play tournament batch
#
#   bash deploy/tk_tourney_start.sh --count 64 --jobs 4
#   bash deploy/tk_tourney_start.sh --count 128 --jobs 8 --model models/seed.json
set -euo pipefail

cd "$(dirname "$0")/.."

COUNT=64
JOBS=4
MODEL=""
while [ $# -gt 0 ]; do
    case "$1" in
        --count) COUNT="$2"; shift 2 ;;
        --jobs) JOBS="$2"; shift 2 ;;
        --model) MODEL="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

echo "==> Launching tournament: $COUNT games, $JOBS jobs"
ARGS="--count $COUNT --jobs $JOBS"
[ -n "$MODEL" ] && ARGS="$ARGS --model $MODEL"

cargo run --release --bin taikyokushogi-usi -- tournament $ARGS \
    || { echo "✗ tournament failed"; exit 1; }

echo "✓ tournament complete"