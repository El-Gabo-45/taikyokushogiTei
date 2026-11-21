#!/usr/bin/env bash
# deploy/tk_duel.sh — head-to-head match between two engine builds
#
#   bash deploy/tk_duel.sh --build-a target/release/engine-a \
#       --build-b target/release/engine-b --games 100
set -euo pipefail

cd "$(dirname "$0")/.."

BUILD_A=""
BUILD_B=""
GAMES=50
while [ $# -gt 0 ]; do
    case "$1" in
        --build-a) BUILD_A="$2"; shift 2 ;;
        --build-b) BUILD_B="$2"; shift 2 ;;
        --games) GAMES="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

[ -n "$BUILD_A" ] || { echo "✗ --build-a required" >&2; exit 2; }
[ -n "$BUILD_B" ] || { echo "✗ --build-b required" >&2; exit 2; }

echo "==> Match: $BUILD_A vs $BUILD_B ($GAMES games)"
cargo run --release --bin taikyokushogi-usi -- match \
    --engine-a "$BUILD_A" --engine-b "$BUILD_B" --games "$GAMES" \
    || { echo "✗ match failed"; exit 1; }

echo "✓ match complete"