#!/usr/bin/env bash
# deploy/tk_pgo.sh — Profile-Guided Optimization build
#
#   bash deploy/tk_pgo.sh              # instrument, run workload, optimize
#   bash deploy/tk_pgo.sh --no-run     # only instrument, skip workload
set -euo pipefail

cd "$(dirname "$0")/.."

NO_RUN=0
[ "${1:-}" = "--no-run" ] && NO_RUN=1

echo "==> PGO build: instrumenting"
RUSTFLAGS="-C profile-generate=target/pgo" cargo build --release \
    || { echo "✗ instrumented build failed"; exit 1; }

if [ "$NO_RUN" -eq 0 ]; then
    echo "==> PGO build: running training workload for profiling"
    ./target/release/taikyokushogi-usi --pgo-warmup --games 64 \
        || echo "    (workload failed — continuing with raw instrumented binary)"
fi

echo "==> PGO build: final optimized build"
RUSTFLAGS="-C profile-use=target/pgo" cargo build --release \
    || { echo "✗ optimized build failed"; exit 1; }

echo "✓ PGO binary ready: target/release/taikyokushogi-usi"