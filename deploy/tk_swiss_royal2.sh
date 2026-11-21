#!/usr/bin/env bash
# deploy/tk_swiss_royal2.sh — royal S2 Swiss tournament
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Royal S2 Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ Royal S2 Swiss ready"