#!/usr/bin/env bash
# deploy/tk_digest.sh — daily summary of training/tournament activity
#
#   bash deploy/tk_digest.sh
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Daily digest — $(date -Is)"
echo ""

echo "-- Games played today --"
find data/raw/games -name "*.json" -mtime -1 2>/dev/null | wc -l \
    | xargs -I{} echo "    {} games"

echo ""
echo "-- Model files --"
ls -lh models/ 2>/dev/null || echo "    no models/ directory"

echo ""
echo "-- Recent training runs --"
ls -lt data/run/ 2>/dev/null | head -5 || echo "    no data/run/ directory"

echo ""
echo "✓ digest complete"