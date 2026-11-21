#!/usr/bin/env bash
# deploy/tk_ci.sh — local mirror of CI, run before every push
#   bash deploy/tk_ci.sh
set -euo pipefail

cd "$(dirname "$0")/.."

die()  { echo "✗ CI CHECK ABORTED: $*" >&2; exit 1; }
step() { printf '\n==> %s\n' "$1"; }

step "rustup update stable (sync with CI toolchain)"
rustup update stable >/dev/null 2>&1 || echo "    (offline — keeping installed toolchain)"
rustc --version
cargo clippy --version

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

step "cargo check --all-targets --features server"
cargo check --all-targets --features server

step "cargo test (lib + doc + smoke + server_api)"
cargo test --features server --lib
cargo test --features server --doc
cargo test --test smoke
cargo test --features server --test server_api

step "engine smoke test (USI)"
cargo build --bin taikyokushogi-usi
{
    printf 'usi\n';    sleep 2
    printf 'isready\n'; sleep 2
    printf 'position startpos\ngo depth 1\n'; sleep 20
    printf 'quit\n'
} | ./target/debug/taikyokushogi-usi > ci_smoke.log 2>&1 || true

grep -q '^id name TaikyokuShogi' ci_smoke.log || { cat ci_smoke.log; die "usiok missing"; }
grep -q '^usiok$'   ci_smoke.log || { cat ci_smoke.log; die "usiok missing"; }
grep -q '^readyok$' ci_smoke.log || { cat ci_smoke.log; die "readyok missing"; }
grep -q '^bestmove ' ci_smoke.log || { cat ci_smoke.log; die "bestmove missing"; }
rm -f ci_smoke.log
echo "    usiok / readyok / bestmove OK"

printf '\nALL GREEN — safe to push.\n'