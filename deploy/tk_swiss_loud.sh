#!/usr/bin/env bash
# deploy/tk_swiss_loud.sh — pull, build, and run a "loud" Swiss tournament
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Loud Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ Loud Swiss ready"