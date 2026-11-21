#!/usr/bin/env bash
# deploy/tk_swiss_filepst.sh — file-based PST Swiss tournament
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> File PST Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ File PST Swiss ready"