#!/usr/bin/env bash
# One-command release: CI gates -> version bump -> commit -> tag -> push.
#
#   bash scripts/release.sh 1.3.0                          # commit msg: "Release 1.3.0"
#   bash scripts/release.sh 1.3.0 "Release 1.3.0: fixed X" # custom commit message
#   bash scripts/release.sh 1.3.0 [MSG] --dry-run          # plan only, changes nothing
#   bash scripts/release.sh 1.3.0 [MSG] --no-push          # stop right before pushing
#   make release VERSION=1.3.0 [MSG="..."]                 # same thing, via make
#
# What it does, in order — it aborts at the first problem, BEFORE touching
# anything, so a failed release never leaves a half-bumped tree:
#   0. guards: X.Y.Z format, strictly greater than Cargo.toml, tag unused,
#      on main/master, clean tree (untracked files are fine), not behind origin
#   1. full CI gates: scripts/ci-check.sh (never tag red code again)
#   2. bump in lockstep: Cargo.toml + Cargo.lock (verified!) + web/package.json
#      + web/package-lock.json  (pyproject.toml needs nothing: version is
#      dynamic and comes from Cargo.toml via maturin)
#   3. commit ONLY the version files, with your message
#   4. annotated tag vX.Y.Z (softprops/action-gh-release generates the notes)
#   5. push main + the tag (the tag push is what publishes the GitHub Release)
#
# It is idempotent: if a push fails, fix the network and run the same command
# again — the bump becomes a no-op and it re-tags/re-pushes safely.
set -euo pipefail

usage() { echo "usage: bash scripts/release.sh <X.Y.Z> [commit-message] [--dry-run] [--no-push]" >&2; exit 2; }
die()   { echo "✗ RELEASE ABORTED: $*" >&2; exit 1; }
step()  { printf '\n==> %s\n' "$1"; }

REPO="$(git rev-parse --show-toplevel 2>/dev/null)" || die "not inside a git repository"
cd "$REPO"

# ── arguments ──────────────────────────────────────────────────────────────
VERSION="${1:-}"; [ -n "$VERSION" ] || usage; shift
MSG=""; DRY_RUN=0; NO_PUSH=0
while [ $# -gt 0 ]; do
    case "$1" in
        --dry-run) DRY_RUN=1 ;;
        --no-push) NO_PUSH=1 ;;
        "") ;;
        *) [ -z "$MSG" ] || usage; MSG="$1" ;;
    esac
    shift
done
TAG="v$VERSION"
MSG="${MSG:-Release $VERSION}"

# ── 0) preflight guards ────────────────────────────────────────────────────
step "preflight: release $VERSION (tag $TAG)"

echo "$VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' \
    || die "'$VERSION' is not in X.Y.Z format"

CUR="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
[ -n "$CUR" ] || die "could not read the current version from Cargo.toml"
if [ "$VERSION" = "$CUR" ]; then
    echo "    note: Cargo.toml is already at $VERSION — nothing to bump (idempotent re-run)"
else
    printf '%s\n%s\n' "$CUR" "$VERSION" | sort -Vc 2>/dev/null \
        || die "$VERSION is not greater than the current $CUR"
fi

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    TAG_HERE="$(git rev-parse "$TAG^{commit}")"
    if [ "$TAG_HERE" = "$(git rev-parse HEAD)" ]; then
        echo "    note: tag $TAG already exists at HEAD — reusing it (idempotent re-run)"
    else
        die "tag $TAG already exists at $(git rev-parse --short "$TAG_HERE"), not HEAD — delete it first: git tag -d $TAG"
    fi
fi

BRANCH="$(git symbolic-ref --short HEAD 2>/dev/null)" || die "detached HEAD — check out main first"
case "$BRANCH" in
    main|master) ;;
    *) die "releases go out from main/master, you are on '$BRANCH'" ;;
esac

DIRTY="$(git status --porcelain | grep -v '^??' || true)"
if [ -n "$DIRTY" ]; then
    echo "$DIRTY" >&2
    die "uncommitted changes above (untracked files are fine) — commit or stash them first"
fi

[ -x scripts/ci-check.sh ] || die "scripts/ci-check.sh missing or not executable"

if git remote get-url origin >/dev/null 2>&1; then
    if git fetch origin --tags --quiet 2>/dev/null; then
        BEHIND="$(git rev-list --count "$BRANCH..origin/$BRANCH" 2>/dev/null || echo 0)"
        [ "$BEHIND" -gt 0 ] && die "local $BRANCH is $BEHIND commit(s) behind origin/$BRANCH — git pull first"
        if git ls-remote --tags origin "refs/tags/$TAG" 2>/dev/null | grep -q .; then
            die "tag $TAG already exists on origin"
        fi
    else
        echo "    note: cannot reach origin — skipping remote checks (push will fail if offline)"
    fi
fi
echo "    $CUR -> $VERSION on $BRANCH, tree clean, tag $TAG free"

# ── dry run: show the plan and stop ────────────────────────────────────────
if [ "$DRY_RUN" -eq 1 ]; then
    step "--dry-run: plan (nothing has been changed)"
    cat <<PLAN
  1. bash scripts/ci-check.sh          # fmt, clippy -D warnings, check, USI smoke, tests, web, ruff+pyright
  2. Cargo.toml:            version $CUR -> $VERSION
     cargo update -w:       Cargo.lock refreshed and verified to say $VERSION (hard-fails otherwise)
     npm version:           web/package.json + web/package-lock.json -> $VERSION (lockstep)
  3. git add Cargo.toml Cargo.lock web/package.json web/package-lock.json
     git commit -m "$MSG"
  4. git tag -a $TAG -m "$TAG"
  5. git push origin $BRANCH && git push origin $TAG   # tag push publishes the GitHub Release
PLAN
    exit 0
fi

# ── 1) CI gates ────────────────────────────────────────────────────────────
step "CI gates (scripts/ci-check.sh)"
bash scripts/ci-check.sh

# ── 2) bump version files ──────────────────────────────────────────────────
step "bump: Cargo.toml $CUR -> $VERSION (+ Cargo.lock + web, lockstep)"

[ "$(grep -c '^version = ' Cargo.toml)" = "1" ] \
    || die "unexpected Cargo.toml layout (not exactly one '^version =' line) — bump manually"
sed -i "s/^version = \".*\"$/version = \"$VERSION\"/" Cargo.toml
grep -q "^version = \"$VERSION\"$" Cargo.toml || die "Cargo.toml bump failed (nothing committed)"

# Refresh the lock's own-package entry and VERIFY it — a stale Cargo.lock was
# a real past incident, so this hard-fails instead of shipping drift.
cargo update -w --quiet 2>/dev/null || true
if ! grep -A1 '^name = "taikyokushogi"$' Cargo.lock | grep -q "^version = \"$VERSION\"$"; then
    cargo metadata --format-version 1 >/dev/null 2>&1 || true
fi
grep -A1 '^name = "taikyokushogi"$' Cargo.lock | grep -q "^version = \"$VERSION\"$" \
    || die "Cargo.lock did not pick up $VERSION (stale lock). Aborting; fix manually with 'cargo update -w'"
echo "    Cargo.lock says $VERSION ✓"

if grep -q "\"version\": \"$VERSION\"" web/package.json; then
    echo "    web/package.json already at $VERSION ✓"
else
    ( cd web && npm version "$VERSION" --no-git-tag-version >/dev/null ) \
        || die "npm version failed inside web/"
    grep -q "\"version\": \"$VERSION\"" web/package.json \
        || die "web/package.json did not pick up $VERSION"
    echo "    web/package.json + package-lock.json say $VERSION ✓"
fi

# ── 3) commit ──────────────────────────────────────────────────────────────
step "commit: $MSG"
git add Cargo.toml Cargo.lock web/package.json web/package-lock.json
if git diff --cached --quiet; then
    echo "    nothing to commit (version files already at $VERSION) — continuing"
else
    git commit -m "$MSG"
fi

# ── 4) tag ─────────────────────────────────────────────────────────────────
if ! git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    step "tag: $TAG (annotated)"
    git tag -a "$TAG" -m "$TAG"
fi

# ── 5) push ────────────────────────────────────────────────────────────────
if [ "$NO_PUSH" -eq 1 ]; then
    step "--no-push: local release $TAG is ready. Finish manually with:"
    echo "  git push origin $BRANCH && git push origin $TAG"
    exit 0
fi

step "push: origin $BRANCH + $TAG (the tag push publishes the GitHub Release)"
git push origin "$BRANCH"
git push origin "$TAG"

REPO_SLUG="$(git remote get-url origin 2>/dev/null | sed -E 's#.*github\.com[:/]##; s/\.git$//' || true)"
step "release $VERSION pushed"
if [ -n "${REPO_SLUG:-}" ]; then
    echo "  CI (main):      https://github.com/$REPO_SLUG/actions"
    echo "  Release build:  https://github.com/$REPO_SLUG/actions/workflows/release.yml"
    echo "  Releases:       https://github.com/$REPO_SLUG/releases"
fi
echo "  next: watch the release.yml run — green there means the archives are up."
