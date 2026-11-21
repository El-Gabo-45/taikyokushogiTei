#!/usr/bin/env bash
# deploy/tk_swiss_hangqa.sh — pull, build, run hang-q-ab Swiss
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Hang q/ab Swiss tournament"
cargo build --release || { echo "✗ build failed"; exit 1; }
echo "✓ Hang q/ab Swiss ready"