#!/usr/bin/env bash
# deploy/tk_native.sh — optimized native build for the host CPU
#
#   bash deploy/tk_native.sh           # release build tuned for this machine
#   bash deploy/tk_native.sh --debug   # debug build with native tuning
set -euo pipefail

cd "$(dirname "$0")/.."

MODE="release"
if [ "${1:-}" = "--debug" ]; then
    MODE="debug"
fi

echo "==> Building native ($MODE) for host CPU"
RUSTFLAGS="-C target-cpu=native" cargo build --$MODE \
    || { echo "✗ build failed"; exit 1; }

BIN="target/$MODE/taikyokushogi-usi"
[ -f "$BIN" ] || { echo "✗ binary not found at $BIN"; exit 1; }
echo "✓ native binary: $BIN"