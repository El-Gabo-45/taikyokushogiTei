// Benchmarks/tools: panic-on-failure is intentional for a diagnostic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::unreadable_literal,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::type_complexity,
    clippy::format_push_string,
    clippy::items_after_statements,
    clippy::doc_markdown,
    clippy::needless_borrow,
    clippy::redundant_closure,
    clippy::similar_names,
    clippy::must_use_candidate,
    clippy::map_unwrap_or,
    clippy::unnecessary_cast
)]

//! Shows the evaluator selection: the hand-crafted evaluator by default, an
//! NNUE network once one is attached to the board — per board, no globals.
//!
//! Usage: cargo run --release --example `toggle_nnue` -- [path/to/net.tkn]
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
