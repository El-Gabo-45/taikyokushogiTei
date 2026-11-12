//! USI front-end binary for the Taikyoku Shogi engine.
//!
//! Speaks the Universal Shogi Interface over stdin/stdout so the engine can
//! be loaded by USI GUIs (ShogiGUI, ShogiHome, Tama, ...) and USI test
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
//! id name TaikyokuShogi 0.1.0
//! ...
//! usiok
//! readyok
//! info depth 1 ...
//! info depth 2 ...
//! bestmove ...
//! ```

fn main() {
    taikyokushogi::usi::run();
}
