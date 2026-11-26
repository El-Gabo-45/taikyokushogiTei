#![allow(clippy::all)]
//! USI front-end binary for the Taikyoku Shogi engine.
//!
//! Speaks the Universal Shogi Interface over stdin/stdout so the engine can
//! be loaded by USI GUIs (`ShogiGUI`, `ShogiHome`, Tama, ...) and USI test
//! harnesses. The protocol implementation lives in
//! [`taikyokushogi::usi`](taikyokushogi::usi) — see its docs for the
//! Taikyoku Shogi profile (TSFEN positions, 36x36 coordinate notation,
//! supported `go` parameters and options).
//!
//! Run with: `cargo run --bin taikyokushogi-usi`
//! NNUE network: `TAIKYOKU_NET` (e.g. `nets/experimental-v1.tkn` or
//! `nets/experimental-v1.tkn.zst`; `none` disables), changeable at runtime
//! with `setoption name NetworkFile value <path>`.
//!
//! A minimal session:
//!
//! ```text
//! $ printf 'usi\nisready\nposition startpos\ngo depth 2\nquit\n' \
//!     | cargo run --release --bin taikyokushogi-usi
//! id name TaikyokuShogi <version>
//! ...
//! usiok
//! readyok
//! info depth 1 ...
//! info depth 2 ...
//! bestmove ...
//! ```

fn main() {
    // GUIs start the binary with no arguments, so these flags cannot affect
    // the USI handshake. They are conveniences for humans/tests only.
    match std::env::args().nth(1).as_deref() {
        Some("--version" | "-V") => {
            println!("taikyokushogi-usi {}", taikyokushogi::VERSION);
            return;
        }
        Some("--help" | "-h") => {
            println!(
                "taikyokushogi-usi — USI front-end for Taikyoku Shogi\n\n\
                 Usage: taikyokushogi-usi [--version] [--help]\n\n\
                 With no arguments it speaks USI over stdin/stdout. Configure\n\
                 it with the `usi`/`setoption`/`position`/`go` commands (see\n\
                 docs/USI.md) or the TAIKYOKU_NET environment variable."
            );
            return;
        }
        _ => taikyokushogi::usi::run(),
    }
}
