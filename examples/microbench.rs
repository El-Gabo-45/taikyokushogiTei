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
    let b0 = Board::initial();
    let moves = b0.legal_moves();
    println!("legal moves from start: {}", moves.len());
    // legal_moves timing
    let t = Instant::now();
    let n = 50;
    for _ in 0..n {
        let _ = b0.legal_moves();
    }
    println!(
        "legal_moves: {:.1} us/call",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    // evaluate timing
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.evaluate();
    }
    println!(
        "evaluate: {:.1} us/call",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    // apply+undo timing
    let m = moves[0];
    let t = Instant::now();
    let n = 500;
    let mut b = Board::initial();
    for _ in 0..n {
        b.apply(&m);
        b.undo();
    }
    println!(
        "apply+undo: {:.1} us/call",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    // material_score
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.material_score();
    }
    println!(
        "material_score: {:.1} us/call",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
    // clone
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.clone();
    }
    println!(
        "clone: {:.1} us/call",
        t.elapsed().as_micros() as f64 / f64::from(n)
    );
}
