#!/usr/bin/env bash
# deploy/tk_milestone.sh — check if a game-count milestone was reached
#
#   bash deploy/tk_milestone.sh --count 1000 --file data/games.count
set -euo pipefail

COUNT=0
FILE=""
while [ $# -gt 0 ]; do
    case "$1" in
        --count) COUNT="$2"; shift 2 ;;
        --file) FILE="$2"; shift 2 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

CURRENT=0
[ -f "$FILE" ] && CURRENT="$(cat "$FILE" 2>/dev/null || echo 0)"

if [ "$CURRENT" -ge "$COUNT" ]; then
    echo "✓ milestone reached: $CURRENT >= $COUNT"
    exit 0
else
    echo "milestone not reached: $CURRENT < $COUNT"
    exit 1
fi