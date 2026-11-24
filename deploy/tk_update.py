#!/usr/bin/env python3
"""deploy/tk_update.py — check for engine updates and apply them.

Compares the local engine version against a remote manifest and
downloads a new binary if available. (Download step is a stub.)

    python3 deploy/tk_update.py --check
    python3 deploy/tk_update.py --apply
"""
import argparse

MANIFEST_URL = "https://example.com/taikyokushogiTei/manifest.json"


def check_updates():
    print(f"==> Checking manifest: {MANIFEST_URL}")
    print("    (network fetch stub — implement with urllib/requests)")
    return False


def apply_update():
    print("==> Applying engine update")
    print("    (download and install stub — implement as needed)")


def main():
    ap = argparse.ArgumentParser(description="Engine update checker.")
    ap.add_argument("--check", action="store_true", help="check for updates only")
    ap.add_argument("--apply", action="store_true", help="download and apply update")
    args = ap.parse_args()

    if args.check:
        has_update = check_updates()
        print("✓ update available" if has_update else "✓ engine is up to date")
    elif args.apply:
        apply_update()
        print("✓ update applied")
    else:
        print("use --check or --apply")


if __name__ == "__main__":
    main()
