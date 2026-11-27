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

//! Deterministic fixed-depth benchmark: search from the initial position
//! with no time limit. Node counts are directly comparable between engine
//! versions because the search tree is identical at the root.
use std::io::Write;
use taikyokushogi::Board;

fn main() {
    let depths: Vec<u32> = std::env::args()
        .skip(1)
        .map(|a| a.parse().unwrap())
        .collect();
    let depths = if depths.is_empty() {
        vec![4, 5, 6]
    } else {
        depths
    };
    for depth in depths {
        let mut board = Board::initial();
        let t = std::time::Instant::now();
        let r = board.search(depth, 0);
        let ms = t.elapsed().as_millis() as f64;
        println!(
            "depth={} nodes={} score={} best={:?} time={:.0}ms nps={:.0}",
            depth,
            r.nodes,
            r.score,
            r.best_move
                .as_ref()
                .map(|m| format!("{}->{}", m.from(), m.to()))
                .unwrap_or_default(),
            ms,
            r.nodes as f64 / (ms / 1000.0)
        );
        let _ = std::io::stdout().flush();
    }
}
