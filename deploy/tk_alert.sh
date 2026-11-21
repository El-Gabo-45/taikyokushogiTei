#!/usr/bin/env bash
# deploy/tk_alert.sh — send a notification (SMTP or ntfy)
#
#   bash deploy/tk_alert.sh "tournament finished"
#   bash deploy/tk_alert.sh --subject "alert" "body text"
set -euo pipefail

SUBJECT="taikyokushogiTei notification"
BODY=""
while [ $# -gt 0 ]; do
    case "$1" in
        --subject) SUBJECT="$2"; shift 2 ;;
        *) BODY="$1"; shift ;;
    esac
done

[ -n "$BODY" ] || { echo "✗ message body required" >&2; exit 2; }

echo "==> Notify: $SUBJECT"
echo "    $BODY"

if command -v ntfy >/dev/null 2>&1 && [ -n "${NTFY_TOPIC:-}" ]; then
    ntfy publish "$NTFY_TOPIC" "$SUBJECT: $BODY" \
        || echo "    (ntfy publish failed)"
elif command -v msmtp >/dev/null 2>&1 && [ -n "${SMTP_TO:-}" ]; then
    {
        echo "Subject: $SUBJECT"
        echo ""
        echo "$BODY"
    } | msmtp "$SMTP_TO" \
        || echo "    (msmtp send failed)"
else
    echo "    (no ntfy or msmtp configured — message only printed)"
fi