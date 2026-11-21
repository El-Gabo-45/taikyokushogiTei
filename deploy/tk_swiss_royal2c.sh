#!/usr/bin/env bash
# deploy/tk_swiss_royal2c.sh — royal S2 champions Swiss
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Royal S2 champions Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ Royal S2 champions Swiss ready"