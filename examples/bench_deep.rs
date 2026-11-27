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

// Stress-tests the engine at higher depths and longer thinking times,
// closer to how it would actually be used in a real game.
// Run with: cargo run --release --example bench_deep
use std::time::Instant;
use taikyokushogi::Board;

fn run(depth: u32, limit_ms: u64, n_moves: usize) {
    let mut board = Board::initial();
    let mut max_move_ms = 0f64;
    let t = Instant::now();
    let mut played = 0;
    let mut total_nodes = 0u64;

    for _ in 0..n_moves {
        let mt = Instant::now();
        let result = board.search(depth, limit_ms);
        let ms = mt.elapsed().as_secs_f64() * 1000.0;
        if ms > max_move_ms {
            max_move_ms = ms;
        }
        total_nodes += result.nodes;
        if let Some(mv) = result.best_move {
            board.apply(&mv);
            played += 1;
        } else {
            break;
        }
    }

    let overshoot = max_move_ms - limit_ms as f64;
    let total_s = t.elapsed().as_secs_f64();
    println!(
        "depth={:<2} limit={:>6}ms  moves={:<3} total={:>8.0}ms  max_move={:>8.1}ms  overshoot={:>+7.1}ms  nodes={:>10}  nps={:>9.0}",
        depth,
        limit_ms,
        played,
        t.elapsed().as_secs_f64() * 1000.0,
        max_move_ms,
        overshoot,
        total_nodes,
        total_nodes as f64 / total_s
    );
}

fn main() {
    println!("Stress test: higher depths, longer thinking times.\n");

    println!("-- Fixed 1000ms budget across depths (checks deadline holds as depth grows) --");
    run(4, 1000, 6);
    run(6, 1000, 6);
    run(8, 1000, 5);
    run(10, 1000, 4);

    println!("\n-- Longer 3000ms budget, where Lazy SMP threads matter more --");
    run(6, 3000, 4);
    run(8, 3000, 4);
    run(10, 3000, 3);

    println!("\n-- Very long 8000ms budget, deep search --");
    run(8, 8000, 2);
    run(10, 8000, 2);
    run(12, 8000, 2);
}
