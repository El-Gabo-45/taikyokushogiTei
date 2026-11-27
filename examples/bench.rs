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

use std::time::Instant;
use taikyokushogi::Board;

fn main() {
    let b = Board::initial();

    // 1. Medir movegen
    let t = Instant::now();
    let mut total = 0;
    for _ in 0..100 {
        let moves = b.legal_moves();
        total += moves.len();
    }
    let us = t.elapsed().as_micros() as f64 / 100.0;
    println!("movegen: {} moves  {:.1}us each", total / 100, us);

    // 2. Medir eval
    let t = Instant::now();
    for _ in 0..1000 {
        let _s = b.evaluate();
    }
    let us = t.elapsed().as_micros() as f64 / 1000.0;
    println!("eval: {us:.1}us each");

    // 3. Medir depth-1 search
    let mut board = Board::initial();
    let t = Instant::now();
    let result = board.search(1, 0);
    let ms = t.elapsed().as_micros() as f64 / 1000.0;
    println!(
        "depth-1 search: {} nodes  {:.2}ms  {:.0} nps",
        result.nodes,
        ms,
        result.nodes as f64 / (ms / 1000.0)
    );

    // 4. Simular selfplay: 10 movimientos
    let mut board = Board::initial();
    let t = Instant::now();
    let mut total_nodes = 0u64;
    for _i in 0..10 {
        let result = board.search(1, 0);
        if let Some(mv) = result.best_move {
            board.apply(&mv);
            total_nodes += result.nodes;
        } else {
            break;
        }
    }
    let ms = t.elapsed().as_micros() as f64 / 1000.0;
    println!(
        "selfplay (10 moves, depth-1): {:.2}ms  {:.0} nps",
        ms,
        total_nodes as f64 / (ms / 1000.0)
    );
}
