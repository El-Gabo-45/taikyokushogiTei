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
    for f in ["game_result", "evaluate", "king_safety-x2", "legal_moves"] {
        let _ = f;
    }
    let t = Instant::now();
    let n = 2000;
    for _ in 0..n {
        let _ = b.game_result();
    }
    println!(
        "game_result: {:.2} us",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    let t = Instant::now();
    let n = 2000;
    for _ in 0..n {
        let _ = b.evaluate();
    }
    println!(
        "evaluate: {:.2} us",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b.legal_moves();
    }
    println!(
        "legal_moves: {:.1} us",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
}
