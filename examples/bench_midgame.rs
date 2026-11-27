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

// Measures legal_moves() cost in a realistic midgame position (after some
// selfplay), not just the initial position where few pieces are near each
// other. More opposing pieces scattered around the board means more ray
// walks can reach a target, so this is the worst case for move generation.
use std::time::Instant;
use taikyokushogi::Board;

fn main() {
    let mut board = Board::initial();

    // Play out some moves with a shallow, fast search to reach a
    // representative midgame position.
    for _ in 0..12 {
        let result = board.search(2, 0);
        if let Some(mv) = result.best_move {
            board.apply(&mv);
        } else {
            break;
        }
    }

    let t = Instant::now();
    let mut total = 0;
    let iters = 100;
    for _ in 0..iters {
        let moves = board.legal_moves();
        total += moves.len();
    }
    let us = t.elapsed().as_micros() as f64 / iters as f64;
    println!(
        "midgame legal_moves: {} moves  {:.1}us each (after 12 plies of selfplay)",
        total / iters,
        us
    );
}
