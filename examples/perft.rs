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

fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut n = 0u64;
    for m in moves {
        board.apply(&m);
        n += perft(board, depth - 1);
        board.undo();
    }
    n
}

fn main() {
    let depth = std::env::args()
        .nth(1)
        .map_or(2, |a| a.parse::<u32>().unwrap());
    let mut board = Board::initial();
    let t = std::time::Instant::now();
    let n = perft(&mut board, depth);
    let ms = t.elapsed().as_millis() as u64;
    println!("perft({depth}) = {n}   (reference perft(1)=512 perft(2)=260917)");
    println!("elapsed: {ms}ms");
}
