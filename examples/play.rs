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

//! Play a real game of Taikyoku Shogi with actual search.
//! Plays a complete game until `game_result()` ends it (all royal pieces
//! captured, or a draw rule) or one side has no legal move.
//!
//! ```sh
//! cargo run --release --example play
//! ```

use std::time::Instant;
use taikyokushogi::Board;

fn main() {
    println!("=== Taikyoku Shogi Self-Play ===");
    println!("Board: 36x36 = {} squares", 36 * 36);
    println!();

    let mut board = Board::initial();
    println!(
        "Initial: Black={} White={}",
        board.piece_count(taikyokushogi::Color::Black),
        board.piece_count(taikyokushogi::Color::White)
    );

    let start = Instant::now();
    let mut total_nodes: u64 = 0;
    let mut total_moves: u32 = 0;
    let max_moves = 1000; // safety limit

    for move_no in 0..max_moves {
        if let Some(result) = board.game_result() {
            println!("\nGame over at move {move_no}: {result:?}");
            break;
        }

        // Full depth-1 search on every move (fast: ~0.1ms per move)
        let result = board.search(1, 0);
        total_nodes += result.nodes;

        if let Some(mv) = result.best_move {
            if move_no < 5 || move_no % 20 == 19 || move_no >= max_moves - 5 {
                println!(
                    "Move {:3}: {}->{} score={:6} nodes={:7} time={}ms",
                    move_no + 1,
                    mv.from(),
                    mv.to(),
                    result.score,
                    result.nodes,
                    result.time_ms
                );
            }
            board.apply(&mv);
            total_moves += 1;
        } else {
            println!("No legal moves at move {}", move_no + 1);
            break;
        }
    }

    let elapsed = start.elapsed();
    let ms = elapsed.as_millis() as u64;
    println!();
    println!("=== Results ===");
    println!("Moves played: {total_moves}");
    println!("Total nodes: {total_nodes}");
    println!("Total time: {ms}ms");
    if ms > 0 {
        println!(
            "Search speed: {:.0} nps",
            total_nodes as f64 / (ms as f64 / 1000.0)
        );
    }
    println!(
        "Final: Black={} White={}",
        board.piece_count(taikyokushogi::Color::Black),
        board.piece_count(taikyokushogi::Color::White)
    );
    println!("Material score (Black): {}", board.material_score());
}
