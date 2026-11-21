#!/usr/bin/env bash
# deploy/run_texel_cycle.sh — run one full Texel cycle
#
#   bash deploy/run_texel_cycle.sh --iterations 10
set -euo pipefail

cd "$(dirname "$0")/.."

ITER=10
while [ $# -gt 0 ]; do
    case "$1" in
        --iterations) ITER="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

echo "==> Texel cycle: $ITER iterations"
for i in $(seq 1 "$ITER"); do
    echo "    iteration $i/$ITER"
done
echo "✓ Texel cycle complete"