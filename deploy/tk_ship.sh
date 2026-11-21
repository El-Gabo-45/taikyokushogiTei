#!/usr/bin/env bash
# deploy/tk_ship.sh — one-command release for taikyokushogiTei
#   bash deploy/tk_ship.sh 0.2.0              # bump, gate, commit, tag, push
#   bash deploy/tk_ship.sh 0.2.0 "msg"        # custom commit message
#   bash deploy/tk_ship.sh 0.2.0 --dry-run    # plan only, no changes
#   bash deploy/tk_ship.sh 0.2.0 --no-push    # stop before pushing
set -euo pipefail

die()  { echo "✗ RELEASE ABORTED: $*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$1"; }

REPO="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not inside a git repository"
cd "$REPO"

VERSION="${1:-}"; [ -n "$VERSION" ] || { echo "usage: bash deploy/tk_ship.sh <X.Y.Z> [msg] [--dry-run] [--no-push]" >&2; exit 2; }; shift
MSG=""; DRY_RUN=0; NO_PUSH=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1 ;;
        --no-push) NO_PUSH=1 ;;
        "") ;;
        *) [ -z "$MSG" ] || { echo "usage: bash deploy/tk_ship.sh <X.Y.Z> [msg] [--dry-run] [--no-push]" >&2; exit 2; }; MSG="$1" ;;
    esac
    shift
done
TAG="v$VERSION"
MSG="${MSG:-Release $VERSION}"

# ── preflight guards ──
step "preflight: release $VERSION (tag $TAG)"
echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' || die "'$VERSION' is not X.Y.Z"

CUR="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ -n "$CUR" ] || die "could not read version from Cargo.toml"

if [ "$VERSION" = "$CUR" ]; then
    echo "    note: Cargo.toml already at $VERSION"
else
    printf '%s\n%s\n' "$CUR" "$VERSION" | sort -Vc 2>/dev/null || die "$VERSION is not greater than $CUR"
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    TAG_HERE="$(git rev-parse "$TAG^{commit}")"
    [ "$TAG_HERE" = "$(git rev-parse HEAD)" ] || die "tag $TAG exists elsewhere"
    echo "    note: tag $TAG already at HEAD — reusing"
fi

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
case "$BRANCH" in main|master) ;; *) die "must be on main/master, on $BRANCH" ;; esac

git fetch origin 2>/dev/null || true
REMOTE="$(git rev-parse "origin/$BRANCH" 2>/dev/null || echo "")"
[ -z "$REMOTE" ] || [ "$REMOTE" = "$(git rev-parse HEAD)" ] || die "not in sync with origin/$BRANCH"

git diff --quiet || die "working tree has unstaged changes"

# ── local gates ──
step "local gates: fmt, clippy, check"
cargo fmt --check || die "cargo fmt --check failed"
cargo clippy --all-targets -- -D warnings || die "cargo clippy failed"
cargo check --all-targets || die "cargo check failed"

# ── bump ──
step "bump Cargo.toml -> $VERSION"
sed -i "s/^version = \".*\"$/version = \"$VERSION\"/" Cargo.toml \
    || die "sed failed on Cargo.toml"
cargo update -w 2>/dev/null || true
cargo metadata --format-version 1 >/dev/null 2>&1 || true

# ── commit ──
step "commit: $MSG"
git add Cargo.toml Cargo.lock
if git diff --cached --quiet; then
    echo "    nothing to commit (already at $VERSION)"
else
    git commit -m "$MSG"
fi

# ── tag ──
step "tag: $TAG"
[ "$DRY_RUN" -eq 1 ] && { echo "    (dry-run: would tag $TAG)"; exit 0; }
git tag -a "$TAG" -m "$TAG"

# ── push ──
step "push: origin $BRANCH + $TAG"
if [ "$NO_PUSH" -eq 1 ]; then
    echo "    --no-push: done locally. Finish with:"
    echo "      git push origin $BRANCH && git push origin $TAG"
    exit 0
fi
git push origin "$BRANCH"
git push origin "$TAG"
echo "✓ release $VERSION pushed"