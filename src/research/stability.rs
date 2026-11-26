#![allow(clippy::all)]
//! `nnue stability` — Tactical Stability Analyzer.
//!
//! Runs ONE search (single-threaded, deterministic) and records every
//! completed iterative-deepening iteration, then reports the depth ladder:
//! how the score and the best move evolve as the search gets deeper.
//!
//! Reading the output:
//! * `Δ` is the score change between consecutive depths, in engine units
//!   (a pawn is 500). Swings of ≥ 1 pawn usually mean a tactical change of
//!   horizon, not noise.
//! * `best` flips late = move-ordering luck; flips with pawn-sized score
//!   swings = the position is unstable or the evaluator disagrees with the
//!   deeper search.
//! * This is the cheap version of the audit's "Horizon Effect Detector":
//!   run it on positions the engine decides instantly and compare with
//!   `nnue multipv` at the same depth.
//!
//! ```text
//! nnue stability [<tsfen>] [--net FILE] [--depth D] [--time-ms T]
//! ```

use super::{load_position, num};

pub fn run(args: &[String]) -> Result<(), String> {
    let (mut board, net) = load_position(args)?;
    let depth = num(args, "--depth", 4u32)?;
    let time_ms = num(args, "--time-ms", 0u64)?;
    if depth == 0 {
        return Err("--depth must be at least 1".into());
    }
    if board.game_result().is_some() {
        return Err("position is already over — nothing to stabilize".into());
    }

    let mut searcher = crate::search::Searcher::new();
    let mut ladder: Vec<crate::search::IterInfo> = Vec::new();
    let result = board.search_with_info(
        &mut searcher,
        depth,
        time_ms,
        1,
        Some(&mut |it| ladder.push(it.clone())),
    );

    println!("=== Search stability ===");
    println!("position: {}", board.to_tsfen());
    println!(
        "eval:     {}",
        if net.is_some() {
            "NNUE network"
        } else {
            "hand-crafted evaluator"
        }
    );
    println!();
    println!(
        "{:>5}  {:>10}  {:>9}  {:<14}  {:>12}  {:>8}",
        "depth", "score", "Δ", "best", "nodes", "time"
    );
    let mut prev_score: Option<i32> = None;
    let mut prev_best: Option<String> = None;
    let mut max_delta: i64 = 0;
    let mut changes = 0usize;
    let mut pawn_swings = 0usize;
    for it in &ladder {
        let best = it
            .best_move
            .as_ref()
            .map_or_else(|| "-".to_string(), std::string::ToString::to_string);
        let delta = prev_score.map_or(0i64, |p| i64::from(it.score - p));
        if delta.abs() > max_delta {
            max_delta = delta.abs();
        }
        if delta.abs() >= 500 {
            pawn_swings += 1;
        }
        if let Some(pb) = &prev_best {
            if pb != &best {
                changes += 1;
            }
        }
        println!(
            "{:>5}  {:+10}  {:+9}  {:<14}  {:>12}  {:>7}ms",
            it.depth, it.score, delta, best, it.nodes, it.time_ms
        );
        prev_score = Some(it.score);
        prev_best = Some(best);
    }
    println!();
    if let Some(last) = ladder.last() {
        println!(
            "final:    depth {}  score {:+}  best {}",
            last.depth,
            result.score,
            result
                .best_move
                .as_ref()
                .map_or_else(|| "-".to_string(), std::string::ToString::to_string)
        );
    }
    println!("max |Δ| between iterations: {max_delta} engine units (a pawn is 500)");
    println!("best-move changes across the ladder: {changes} ({pawn_swings} swings of ≥ 1 pawn)");
    let verdict = if ladder.len() < 3 {
        "SHORT — fewer than 3 completed iterations; raise --depth (or --time-ms)"
    } else if changes == 0 && max_delta < 100 {
        "STABLE — same move throughout, the evaluation barely moves with depth"
    } else if pawn_swings == 0 {
        "MILD — small score movement; check whether the best-move flips are ordering luck"
    } else {
        "WOBBLING — pawn-sized score swings: the position is tactically unstable, or the evaluator disagrees with deeper search"
    };
    println!("verdict:  {verdict}");
    Ok(())
}

#[cfg(test)]
mod tests {
    /// One search must yield one ladder entry per completed depth, and the
    /// ladder must be depth-ordered and capped by the requested depth.
    /// (Depth 2 keeps the test fast; the structure is depth-independent.)
    #[test]
    fn ladder_covers_each_completed_iteration() {
        let mut board = crate::Board::initial();
        let mut searcher = crate::search::Searcher::new();
        let mut ladder: Vec<crate::search::IterInfo> = Vec::new();
        let result = board.search_with_info(
            &mut searcher,
            2,
            0,
            1,
            Some(&mut |it| ladder.push(it.clone())),
        );
        assert!(!ladder.is_empty(), "at least one iteration completes");
        assert!(ladder[0].depth >= 1);
        assert!(ladder.last().expect("non-empty").depth <= 2);
        assert!(
            ladder[0].nodes <= result.nodes,
            "an iteration's node count never exceeds the whole search's"
        );
        // Scores must be present and the ladder is depth-ordered.
        assert!(ladder.windows(2).all(|w| w[0].depth < w[1].depth));
    }
}
