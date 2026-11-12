//! Taikyoku Shogi HTTP Server + Web GUI.
//!
//! Serves the REST API (implemented in `taikyokushogi::server`) and the
//! static files of the web frontend.
//!
//! Run with: `cargo run --features server --bin taikyokushogi-server`
//! Bind address: `TAIKYOKU_BIND` (default `127.0.0.1:8000`).
//! Frontend dir: `TAIKYOKU_DIST` (default `web/dist`).
//! NNUE network: `TAIKYOKU_NET` (e.g. `nets/experimental-v1.tkn` or
//! `nets/experimental-v1.tkn.zst`; `none` to disable). The engine panel reports
//! `Backend: nnue` when one is loaded.
//!
//! Session mode: `--sessions` (or `TAIKYOKU_SESSIONS=1`) gives every visitor
//! an isolated game instead of one shared board — see `docs/WEB.md`.

#[tokio::main]
async fn main() {
    apply_flags();
    taikyokushogi::server::run().await;
}

/// Minimal flag handling: everything else is configured through the
/// `TAIKYOKU_*` environment variables (see README / docs/WEB.md). Flags
/// override the variables of their kind.
fn apply_flags() {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // Per-session game state: every visitor gets an isolated game.
            "--sessions" => std::env::set_var("TAIKYOKU_SESSIONS", "1"),
            "--bind" => {
                if let Some(v) = args.next() {
                    std::env::set_var("TAIKYOKU_BIND", v);
                }
            }
            "--net" => {
                if let Some(v) = args.next() {
                    std::env::set_var("TAIKYOKU_NET", v);
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other}\n");
                print_help();
                std::process::exit(2);
            }
        }
    }
}

fn print_help() {
    println!(
        "taikyokushogi-server — Taikyoku Shogi HTTP server + web GUI\n\n\
         Usage: taikyokushogi-server [--sessions] [--bind ADDR] [--net PATH|none]\n\n\
         Options:\n  \
         --sessions    Every visitor gets an isolated game (per-session\n  \
                       game state instead of one shared board)\n  \
         --bind ADDR   Bind address (default 127.0.0.1:8000; TAIKYOKU_BIND)\n  \
         --net PATH    NNUE network .tkn/.tkn.zst, or `none` (TAIKYOKU_NET)\n\n\
         Every option can also be set via the environment: TAIKYOKU_SESSIONS,\n\
         TAIKYOKU_BIND, TAIKYOKU_NET, TAIKYOKU_DIST, TAIKYOKU_MAX_SESSIONS,\n\
         TAIKYOKU_CORS_ORIGIN. See docs/WEB.md for details."
    );
}
