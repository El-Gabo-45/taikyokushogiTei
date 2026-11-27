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

use taikyokushogi::Board;

fn main() {
    let board = Board::initial();
    let legal = board.legal_moves();
    println!("legal_moves = {}", legal.len());
    // Estimate how many are captures (any move that removes an opponent piece).
    let cap = legal
        .iter()
        .filter(|m| {
            let s = m.to_string();
            s.contains('x') || s.contains('+') // crude capture/promo marker
        })
        .count();
    println!("approx captures/promos (by notation) = {cap}");
}
