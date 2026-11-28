#![allow(clippy::all)]
//! ATSI v0 front-end binary for Taikyoku Shogi.
//!
//! Speaks the protocol used by `taikyoku-shogi.github.io/play` over
//! stdin/stdout, so the engine can be plugged into that web UI through
//! `websocketd` without running the HTTP server. The adapter lives in
//! [`taikyokushogi::atsi`]; see `docs/ATSI.md`.
//!
//! Run with: `cargo run --bin taikyokushogi-atsi`

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("taikyokushogi-atsi {}", taikyokushogi::VERSION);
        }
        Some("--help" | "-h") => {
            println!(
                "taikyokushogi-atsi — ATSI v0 front-end for Taikyoku Shogi\n\n\
                 Usage: taikyokushogi-atsi [--version] [--help]\n\n\
                 With no arguments it speaks ATSI v0 over stdin/stdout.\n\
                 Set TAIKYOKU_ATSI_DEPTH to choose the search depth (default 3)."
            );
        }
        _ => taikyokushogi::atsi::run(),
    }
}
