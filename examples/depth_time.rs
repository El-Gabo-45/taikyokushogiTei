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
    for d in 1..=5u32 {
        let mut b = Board::initial();
        let t = std::time::Instant::now();
        let r = b.search(d, 0);
        println!(
            "d={} nodes={} ms={} best={:?}",
            d,
            r.nodes,
            t.elapsed().as_millis(),
            r.best_move.map(|m| m.to())
        );
    }
}
