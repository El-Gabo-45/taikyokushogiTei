#!/usr/bin/env bash
# Local mirror of .github/workflows/ci.yml — run before every push.
#
#   bash scripts/ci-check.sh        # or: make ci-local
#
# What it runs (same commands as CI):
#   1. rustup update stable        -> toolchain sync with CI (anti clippy-drift)
#   2. cargo fmt --check           -> 'test' job
#   3. cargo clippy -D warnings    -> 'test' job
#   4. cargo check --all-targets   -> 'test' job
#   5. USI protocol smoke test     -> 'test' job
#   6. lib/doc/smoke/server_api    -> 'test' job
#   7. web (vitest/build/lint)     -> 'web-build' job (skipped without node_modules)
#   8. ruff + pyright              -> 'python-lint' job (skipped if not installed)
#
# Deliberately NOT run locally (delegated to CI, which has its own caches):
#   - 'msrv' (needs a second Rust toolchain installed)
#   - 'pyo3' and 'nnue-python' (maturin wheel + torch matrix)
#   - 'build' release job
#
# NOTE: there is NO `cargo clean` here on purpose. CI builds *with* caches
# (actions/cache on target/), so a clean local build is not "more CI-like" —
# it is just minutes of recompiling for nothing.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$1"; }

# 0) System C compiler (cc). Rust needs `cc` to link native dependencies
#    (e.g. parking_lot_core, libc, zstd-sys, rusqlite). Without it every
#    `cargo build/check/clippy/test` fails with "linker 'cc' not found".
if ! command -v cc >/dev/null 2>&1; then
    step "Installing system C compiler (build-essential) — Rust needs cc to link"
    if command -v apt-get >/dev/null 2>&1; then
        sudo apt-get update -qq && sudo apt-get install -y -qq build-essential pkg-config
    elif command -v apk >/dev/null 2>&1; then
        sudo apk add --no-cache build-base
    elif command -v yum >/dev/null 2>&1; then
        sudo yum install -y gcc
    else
        echo "    ERROR: 'cc' not found and no supported package manager detected." >&2
        echo "    Please install a C compiler (e.g. build-essential on Ubuntu/Debian) and re-run." >&2
        exit 1
    fi
fi

# 1) Toolchain sync. The #1 cause of "passed locally, failed in CI" is clippy
#    drift: CI installs the latest stable on every run while a stale local
#    toolchain lags behind (this repo was bitten by clippy 1.96 vs 1.100).
#    `rustup update stable` REPLACES the existing stable toolchain in place —
#    it does not install a second one. Falls back gracefully when offline.
step "rustup update stable (sync clippy/rustc with CI)"
if rustup update stable >/dev/null 2>&1; then
    echo "    toolchain up to date"
else
    echo "    (offline or rustup unavailable — using the installed toolchain)"
fi
rustc --version
cargo clippy --version

# 2-6) Rust gates, exactly as the CI 'test' job runs them.
step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --all-targets --features server -- -D warnings"
cargo clippy --all-targets --features server -- -D warnings

step "cargo check --all-targets --features server"
cargo check --all-targets --features server

step "USI protocol smoke test"
cargo build --bin taikyokushogi-usi
{
    printf 'usi\n';    sleep 2
    printf 'isready\n'; sleep 2
    printf 'position startpos\ngo depth 1\n'; sleep 20
    printf 'quit\n'
} | ./target/debug/taikyokushogi-usi > usi_smoke.log
grep -q '^id name TaikyokuShogi' usi_smoke.log
grep -q '^usiok$' usi_smoke.log
grep -q '^readyok$' usi_smoke.log
grep -q '^bestmove ' usi_smoke.log
rm -f usi_smoke.log
echo "    usiok/readyok/bestmove OK"

step "cargo test (lib + doc + smoke + server_api)"
cargo test --features server --lib
cargo test --features server --doc
cargo test --test smoke
cargo test --features server --test server_api

# 7) Web gates, same as the CI 'web-build' job (plus eslint, which CI omits).
if [ -d web/node_modules ]; then
    step "web: vitest + tsc + vite build + eslint"
    (cd web && npm test && npm run build && npm run lint)
else
    step "web checks SKIPPED (web/node_modules missing — CI covers them)"
fi

# 8) Python gates, same commands as the CI 'python-lint' job.
if command -v ruff >/dev/null 2>&1 && command -v pyright >/dev/null 2>&1; then
    step "python: ruff check . && pyright"
    ruff check .
    pyright
else
    step "python checks SKIPPED (ruff/pyright not installed — CI covers them)"
fi

printf '\nALL GREEN — safe to push.\n'
