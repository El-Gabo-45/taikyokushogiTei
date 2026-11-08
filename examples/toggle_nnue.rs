//! Shows the evaluator selection: the hand-crafted evaluator by default, an
//! NNUE network once one is attached to the board — per board, no globals.
//!
//! Usage: cargo run --release --example toggle_nnue -- [path/to/net.tkn]
//! Without a path a deterministic random network is used (NOT a playing
//! network; it only demonstrates the switch).

use std::sync::Arc;
use std::time::Instant;
use taikyokushogi::{Board, Network};

fn main() {
    let net = match std::env::args().nth(1) {
        Some(path) => Network::load(&path).unwrap_or_else(|e| {
            eprintln!("error: cannot load `{path}`: {e}");
            std::process::exit(2);
        }),
        None => Network::random(1),
    };

    let mut board = Board::initial();
    let t = Instant::now();
    println!(
        "hand-crafted eval: {:>8}  ({:?})",
        board.evaluate(),
        t.elapsed()
    );

    board.set_network(Some(Arc::new(net)));
    let t = Instant::now();
    println!(
        "NNUE eval:         {:>8}  ({:?})",
        board.evaluate(),
        t.elapsed()
    );

    board.set_network(None);
    println!("detached again:    {:>8}", board.evaluate());
}
