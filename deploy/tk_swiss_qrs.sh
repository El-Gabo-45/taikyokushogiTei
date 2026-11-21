#!/usr/bin/env bash
# deploy/tk_swiss_qrs.sh — q/rs Swiss tournament
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> q/rs Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ q/rs Swiss ready"