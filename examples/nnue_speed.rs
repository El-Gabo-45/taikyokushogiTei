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

//! Micro-benchmark of the NNUE evaluator on a midgame position:
//! incremental `evaluate()`, full accumulator refresh, and the cost NNUE adds
//! to `apply` + `undo`.
//!
//! Run with: cargo run --release --example `nnue_speed` -- [path/to/net.tkn]
//! (the `nnue bench` tool prints the same numbers).

use std::sync::Arc;
use std::time::Instant;
use taikyokushogi::{Board, Network};

fn main() {
    let net = Arc::new(match std::env::args().nth(1) {
        Some(path) => Network::load(&path).unwrap_or_else(|e| {
            eprintln!("error: cannot load `{path}`: {e}");
            std::process::exit(2);
        }),
        None => Network::random(1),
    });

    // Midgame position: 12 plies of depth-2 search.
    let mut board = Board::initial();
    for _ in 0..12 {
        match board.search(2, 0).best_move {
            Some(m) => board.apply(&m),
            None => break,
        }
    }
    board.set_network(Some(net.clone()));

    let iters = 20_000u32;
    let t = Instant::now();
    let mut sink = 0i64;
    for _ in 0..iters {
        sink += i64::from(board.evaluate());
    }
    let inc = t.elapsed() / iters;

    let refreshes = 200u32;
    let t = Instant::now();
    for _ in 0..refreshes {
        board.set_network(None);
        board.set_network(Some(net.clone()));
    }
    let full = t.elapsed() / refreshes;

    println!("evaluate()  incremental : {inc:>10.3?}");
    println!("full refresh            : {full:>10.3?}");
    if sink == i64::MIN {
        println!();
    }
}
