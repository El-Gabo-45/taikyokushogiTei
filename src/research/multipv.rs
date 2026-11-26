#![allow(clippy::all)]
//! `nnue multipv` — `MultiPV`: the best N candidate moves, compared.
//!
//! Method (documented because it shapes the output):
//!
//! 1. **Static-eval prefilter** — every legal move is applied once and the
//!    resulting static evaluation (mover's point of view) ranks it. On a
//!    36×36 board the root branches to ~800 moves, so searching every one is
//!    hopeless; the prefilter costs apply+eval+undo and only decides which
//!    lines get *analyzed*. Ranking inside the analyzed set is by real
//!    search, not by the prefilter.
//! 2. **Full-window search per line** — the top `--max-root` moves are
//!    searched at `--depth − 1`; the line's score is the negated search score
//!    (the search speaks for the opponent), so lines are comparable.
//! 3. A **reference search** at the full depth shows what the engine
//!    actually plays, so disagreements with the table are visible at a
//!    glance.
//!
//! ```text
//! nnue multipv [<tsfen>] [--net FILE] [--depth D] [--n N] [--max-root R] [--time-ms T]
//! ```

use super::{load_position, num, num_opt};
use crate::eval::MATE_SCORE;

/// One analyzed candidate line.
pub(crate) struct Line {
    /// The root move, in engine rendering (`from->to`, `+` on promotion).
    pub mv: crate::Move,
    /// Static evaluation after the move, mover's point of view (the rank).
    pub static_score: i32,
    /// Search-backed score, mover's point of view (the honest number).
    pub score: i32,
    /// The opponent's best reply, if the line was searched.
    pub reply: Option<String>,
    /// Nodes spent on this line.
    pub nodes: u64,
    /// Wall-clock time of this line's search.
    pub time_ms: u64,
    /// Set when the move ends the game (mate/draw score, no search needed).
    pub terminal: bool,
}

/// Result of one `MultiPV` analysis.
pub(crate) struct Analysis {
    /// Total legal root moves seen (before `--max-root` capping).
    pub root_moves: usize,
    /// Analyzed lines, sorted best-first by `score`.
    pub lines: Vec<Line>,
    /// The engine's own choice at the full depth: `(move, score)`.
    pub reference: Option<(String, i32)>,
}

/// Score of a move that ends the game, from the mover's point of view.
fn terminal_score(mover: crate::Color, result: &crate::GameResult) -> i32 {
    let won = matches!(
        (mover, result),
        (crate::Color::Black, crate::GameResult::BlackWins)
            | (crate::Color::White, crate::GameResult::WhiteWins)
    );
    if won {
        MATE_SCORE
    } else if matches!(result, crate::GameResult::Draw) {
        0
    } else {
        -MATE_SCORE
    }
}

/// Analyze a position: prefilter by static eval, search the top `max_root`.
pub(crate) fn analyze(
    board: &mut crate::Board,
    depth: u32,
    max_root: usize,
    time_ms: u64,
) -> Result<Analysis, String> {
    let scored = super::static_move_scores(board);
    let root_moves = scored.len();
    let search_depth = depth.saturating_sub(1);
    let mover = board.side_to_move();

    let mut lines: Vec<Line> = Vec::new();
    for (static_score, mv) in scored.into_iter().take(max_root) {
        board.apply(&mv);
        let (score, reply, nodes, time_ms_line, terminal) = match board.game_result() {
            Some(result) => (terminal_score(mover, &result), None, 0, 0, true),
            None if search_depth == 0 => (-board.evaluate(), None, 0, 0, false),
            None => {
                // A fresh searcher per line: the default searcher caches its
                // root move list and TT across calls, and sharing them
                // between *different child positions* would make line N's
                // result depend on line N−1's search — poison for a
                // comparison tool.
                let mut s = crate::search::Searcher::new();
                let r = board.search_with(&mut s, search_depth, time_ms);
                (
                    -r.score,
                    r.best_move.map(|m| m.to_string()),
                    r.nodes,
                    r.time_ms,
                    false,
                )
            }
        };
        board.undo();
        lines.push(Line {
            mv,
            static_score,
            score,
            reply,
            nodes,
            time_ms: time_ms_line,
            terminal,
        });
    }
    // Honest ranking: by search score, ties broken deterministically.
    lines.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.mv.to_string().cmp(&b.mv.to_string()))
    });

    let reference = {
        // Fresh searcher for the same reason: independence from every line.
        let mut s = crate::search::Searcher::new();
        let r = board.search_with(&mut s, depth, time_ms);
        Some((
            r.best_move.map(|m| m.to_string()).unwrap_or_default(),
            r.score,
        ))
    };
    Ok(Analysis {
        root_moves,
        lines,
        reference,
    })
}

    /// # Errors
    ///
    /// Returns an error describing the failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let (mut board, net) = load_position(args)?;
    let depth = num(args, "--depth", 2u32)?;
    let n = num(args, "--n", 5usize)?;
    let max_root = num(args, "--max-root", 20usize)?;
    let time_ms = num_opt(args, "--time-ms")?.unwrap_or(0);
    if depth == 0 {
        return Err("--depth must be at least 1".into());
    }
    if board.game_result().is_some() {
        return Err("position is already over — no moves to analyze".into());
    }

    let analysis = analyze(&mut board, depth, max_root, time_ms)?;

    println!("=== MultiPV ===");
    println!("position: {}", board.to_tsfen());
    println!(
        "eval:     {}",
        if net.is_some() {
            "NNUE network"
        } else {
            "hand-crafted evaluator"
        }
    );
    println!(
        "method:   static prefilter over {} legal moves, depth-{} search on the top {} (raise --max-root to widen)",
        analysis.root_moves,
        depth.saturating_sub(1),
        analysis.lines.len()
    );
    println!();
    println!(
        "  #  {:<12} {:>10} {:>10} {:>9}  {:<12} {:>10} {:>7}",
        "move", "static", "search", "Δ", "reply", "nodes", "ms"
    );
    let best = analysis.lines.first().map_or(0, |l| l.score);
    for (i, l) in analysis.lines.iter().take(n).enumerate() {
        let delta = l.score - best;
        println!(
            "{:>3}  {:<12} {:+10} {:+10} {:+9}  {:<12} {:>10} {:>7}",
            i + 1,
            l.mv.to_string(),
            l.static_score,
            l.score,
            delta,
            l.reply
                .as_deref()
                .unwrap_or(if l.terminal { "game over" } else { "-" }),
            l.nodes,
            l.time_ms
        );
    }
    if let Some((mv, score)) = &analysis.reference {
        println!();
        let among = analysis
            .lines
            .iter()
            .take(n)
            .any(|l| &l.mv.to_string() == mv);
        println!("engine's own choice (full depth {depth}): {mv} ({score:+})");
        if among {
            println!("it is among the printed lines ✓");
        } else {
            println!(
                "⚠ NOT among the printed lines — the static prefilter dropped it; raise --max-root"
            );
        }
    }
    println!();
    println!(
        "note: line scores are full-window (exact at this depth) search values. The engine's own"
    );
    println!(
        "root search uses aspiration windows, so its reported score can differ from the line score"
    );
    println!("of the same move — compare the CHOICE, not the raw numbers.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Depth 1: no per-line search happens (`search_depth == 0`), so the
    /// "search" column equals the static score and every line is sorted by
    /// it. Structural guarantees only — exact scores depend on the evaluator.
    /// (Root scores use aspiration windows and fail-high bounds — an engine
    /// property learned while probing; per-line scores here are full-window
    /// exact values and need not match them.)
    #[test]
    fn depth1_analysis_is_static_only_and_sorted() {
        let mut board = crate::Board::initial();
        let a = analyze(&mut board, 1, 5, 0).expect("initial position analyzes");
        assert!(a.root_moves > 400, "the initial position branches widely");
        assert_eq!(a.lines.len(), 5);
        assert!(a.lines.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(a.lines.iter().all(|l| !l.terminal));
        assert!(a.reference.is_some());
    }

    /// Depth 2: lines are searched (`nodes > 0`) and replies are present.
    #[test]
    fn depth2_analysis_searches_each_line() {
        let mut board = crate::Board::initial();
        let a = analyze(&mut board, 2, 2, 0).expect("initial position analyzes");
        assert_eq!(a.lines.len(), 2);
        assert!(a.lines.iter().all(|l| l.nodes > 0));
        assert!(a.lines.iter().all(|l| l.reply.is_some()));
        assert!(a.lines.windows(2).all(|w| w[0].score >= w[1].score));
    }

    /// `run()` must not panic and must produce output. Cheap parameters:
    /// depth 1 (static-only lines) and only 3 analyzed lines, so the smoke
    /// stays fast in debug builds.
    #[test]
    fn run_on_initial_position_works() {
        let args: Vec<String> = vec![
            "--depth".to_string(),
            "1".to_string(),
            "--max-root".to_string(),
            "3".to_string(),
        ];
        run(&args).expect("initial position is analyzable");
    }
}
