#!/usr/bin/env bash
# deploy/tk_freeze.sh — freeze training/eval history to prevent drift
#
#   bash deploy/tk_freeze.sh            # snapshot current state
#   bash deploy/tk_freeze.sh --check    # verify nothing changed since freeze
set -euo pipefail

cd "$(dirname "$0")/.."

MODE="snapshot"
[ "${1:-}" = "--check" ] && MODE="check"

FREEZE_DIR="data/freeze_history"
mkdir -p "$FREEZE_DIR"

if [ "$MODE" = "snapshot" ]; then
    echo "==> Freezing history snapshot"
    cp Cargo.lock "$FREEZE_DIR/Cargo.lock.frozen" 2>/dev/null || true
    cp models/*.json "$FREEZE_DIR/" 2>/dev/null || true
    echo "✓ frozen at $(date -Is)"
else
    echo "==> Checking history against freeze"
    if diff -q Cargo.lock "$FREEZE_DIR/Cargo.lock.frozen" >/dev/null 2>&1; then
        echo "✓ Cargo.lock matches frozen snapshot"
    else
        echo "⚠ Cargo.lock differs from frozen snapshot"
    fi
    for f in models/*.json; do
        [ -f "$f" ] || continue
        base="$(basename "$f")"
        if diff -q "$f" "$FREEZE_DIR/$base" >/dev/null 2>&1; then
            echo "✓ $base matches frozen snapshot"
        else
            echo "⚠ $base differs from frozen snapshot"
        fi
    done
fi