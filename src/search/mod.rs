//! Alpha-beta search for Taikyoku Shogi (36x36 board, ~700 legal moves/node).
//!
//! Module layout (one responsibility per file):
//! * [`buffers`]    — per-ply pooled scratch buffers and ordering arrays, so
//!   the search issues no heap allocations after warmup.
//! * [`params`]     — every tunable constant and margin formula, in one place.
//! * [`tt`]         — lock-free bucketed transposition table (race-safe
//!   publish/reverify protocol for Lazy SMP).
//! * [`heuristics`] — killer moves, butterfly history, counter moves.
//! * [`ordering`]   — move scoring (TT move > MVV-LVA > killers > history).
//! * [`pvs`]        — the PVS core: null-window re-search, late move
//!   reductions (history-softened), null-move pruning, razoring / reverse
//!   futility / futility / ProbCut, internal iterative deepening, staged
//!   move generation with incremental pick-next ordering.
//! * [`qsearch`]    — capture-only quiescence with its own TT traffic.
//! * [`root`]       — one root iteration at a fixed depth (aspiration
//!   windows around the previous score), with the root move list cached
//!   across iterations; the opt-in material-delta root (disabled by default)
//!   lives here too.
//!
//! Iterative deepening with predictive time management lives in [`search`]
//! below.
//!
//! NOTE: this variant has NO check and NO checkmate (SPEC §7.3): the game
//! only ends when a side captures the opponent's LAST royal (SPEC §7.2).

mod buffers;
mod heuristics;
mod ordering;
mod params;
mod pvs;
mod qsearch;
mod root;
mod tt;

use crate::board::Board;
use crate::eval::{evaluate, MATE_SCORE};
use crate::types::Move;
use std::time::Instant;

/// Result of a completed search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u64,
}

/// Full search: iterative deepening with aspiration windows and predictive
/// time management.
pub fn search(board: &mut Board, depth: u32, time_limit_ms: u64) -> SearchResult {
    let start = Instant::now();
    // env::var takes a global lock — cache it once per search, not per iteration.
    let debug_log = std::env::var_os("RPS_DEBUG").is_some();
    let deadline = if time_limit_ms > 0 {
        Some(start + std::time::Duration::from_millis(time_limit_ms))
    } else { None };

    tt::tt_new_generation();
    ordering::piece_vals();
    let mut best_result = SearchResult { best_move: None, score: evaluate(board), nodes: 0, time_ms: 0 };
    // Search-lifetime scratch state, allocated once and reused by every
    // iteration: `nodes` is cumulative for the whole search, `pool` recycles
    // the per-ply move buffers and `root_state` caches the root move list.
    let mut nodes: u64 = 0;
    let mut pool = buffers::SearchPool::new();
    let mut root_state = root::RootState::new();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut root_hint: Option<u32> = None;
    let mut score_guess = best_result.score;
    let mut prev_iter_ms: u64 = 0;

    if depth == 0 {
        return SearchResult { best_move: None, score: evaluate(board), nodes: 1, time_ms: 0 };
    }

    for current_depth in 1..=depth {
        if let Some(dl) = deadline {
            if Instant::now() >= dl { break; }
        }

        // ── PREDICTIVE TIME MANAGEMENT ────────────────────────
        // The next iteration typically costs ~4x the previous one (branching
        // factor). If the last completed iteration's duration times 4 no
        // longer fits in the remaining budget, stop — starting it would burn
        // the rest of the time on an unusable partial result. (Standard
        // technique: predict the next iteration's cost from the previous
        // one, as in Stockfish.)
        if current_depth >= 2 && deadline.is_some() && prev_iter_ms > 0 {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let remaining = time_limit_ms.saturating_sub(elapsed_ms);
            if prev_iter_ms.saturating_mul(params::NEXT_ITER_COST_FACTOR) > remaining { break; }
        }

        // Search context shared by every root move of this iteration. The
        // node counter, scratch pool and cached root move list all outlive the
        // iteration, so nothing is reallocated or regenerated between depths.
        let mut rc = root::RootCtx {
            nodes: &mut nodes,
            deadline,
            stop: &stop,
            pool: &mut pool,
            root: &mut root_state,
        };
        let result = if current_depth <= 1 {
            root::search_root_window(board, current_depth, &mut rc, root_hint, -MATE_SCORE - 1, MATE_SCORE + 1)
        } else {
            // Aspiration windows at ALL depths >= 2. The previous version
            // disabled them for d >= 5 because a *narrow* (±64) window failed
            // constantly on this game's volatile scores (the eval can jump
            // thousands of centipawns between iterations when a large capture
            // is found), causing long cascades of re-searches. Policy:
            // initial window ±64 (±200 at d >= 5); on the FIRST fail grow ×8;
            // on the SECOND fail fall back to the FULL window. At most two
            // re-searches per iteration, and each failed re-search is much
            // cheaper than a full-window search thanks to TT hits.
            let mut window = params::aspiration_window(current_depth);
            let mut fails = 0u8;
            let mut alpha = score_guess.saturating_sub(window);
            let mut beta = score_guess.saturating_add(window);
            let mut local_result;
            loop {
                local_result = root::search_root_window(board, current_depth, &mut rc, root_hint, alpha, beta);
                if let Some(dl) = deadline {
                    if Instant::now() >= dl { break; }
                }
                if local_result.score <= alpha || local_result.score >= beta {
                    fails += 1;
                    if fails >= params::ASPIRATION_MAX_FAILS as u8 {
                        break; // keep the (bounded) result — full re-search not worth it
                    }
                    window = (window * params::ASPIRATION_WINDOW_GROW).min(MATE_SCORE);
                    if local_result.score <= alpha {
                        // Fail low: the true score is LOWER than guessed —
                        // recenter the window on the failed score so the next
                        // search is bounded around the right region.
                        score_guess = local_result.score;
                    }
                    alpha = score_guess.saturating_sub(window);
                    beta = score_guess.saturating_add(window);
                    continue;
                }
                break;
            }
            local_result
        };

        // A time limit (or a stop) can cut an iteration short. Its partial
        // result is only adopted when there is nothing better to return: the
        // FIRST iteration running out of budget mid-way still ranks the root
        // moves it reached, while for later iterations the last COMPLETED
        // iteration is the more reliable answer.
        if deadline.map(|dl| Instant::now() >= dl).unwrap_or(false) {
            if best_result.best_move.is_none() && result.best_move.is_some() {
                best_result = result;
            }
            break;
        }
        // `nodes` is cumulative across the whole search (the root shares one
        // counter with every iteration), so there is nothing to add up here.
        if debug_log {
            eprintln!("iter d={} nodes={} score={} t={}ms", current_depth, nodes, result.score, result.time_ms);
        }
        root_hint = result.best_move.as_ref().map(|m| ordering::m_pack(m));
        score_guess = result.score;
        prev_iter_ms = result.time_ms;
        best_result = result;
    }

    let elapsed = start.elapsed().as_millis() as u64;
    SearchResult {
        best_move: best_result.best_move,
        score: best_result.score,
        nodes: nodes.max(best_result.nodes),
        time_ms: elapsed,
    }
}

/// Resize the transposition table to `mb` MiB; returns the size actually
/// allocated (rounded down to a power-of-two bucket count).
///
/// The table persists across [`search`] calls, so this is the knob behind a
/// UI's "hash" setting. Calling it is safe at any time — in-flight lookups
/// keep the table they started on alive until they finish.
pub fn set_hash_mb(mb: usize) -> usize {
    tt::resize_mb(mb)
}

/// Current transposition-table size in MiB.
pub fn hash_mb() -> usize {
    tt::size_mb()
}

/// Reset the state that the search shares between *searches*: the transposition
/// table and the killer/history tables.
///
/// These tables are global on purpose — carrying information from one search
/// into the next is what makes the engine fast across moves — so two
/// consecutive searches on the same position may legitimately return different
/// scores. Call this when a reproducible run is required (tests) or when the
/// caller knows the previous search is irrelevant (a new game).
#[allow(dead_code)] // used by tests; the natural hook for a UI "new game"
pub(crate) fn reset_shared_tables() {
    tt::clear();
    heuristics::clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Board;

    #[test]
    fn search_initial_reaches_depth_and_finds_a_move() {
        // Engine-global state is shared: run alone (see crate::test_lock).
        let _serial = crate::test_lock::lock();
        heuristics::clear();
        let mut board = Board::initial();
        let r = board.search(4, 0);
        assert!(r.best_move.is_some(), "depth-4 search must return a move");
        assert!(r.nodes > 0);
        assert!(r.score.abs() < MATE_SCORE, "eval must not look like mate");
    }

    #[test]
    fn history_clear_zeroes_counters() {
        // The history tables are engine-global: run alone (crate::test_lock).
        let _serial = crate::test_lock::lock();
        heuristics::history_store(3, 4, 5, 0);
        assert!(heuristics::history_score(3, 4, 0) > 0);
        heuristics::clear();
        assert_eq!(heuristics::history_score(3, 4, 1), 0);
        assert_eq!(heuristics::history_score(0, 0, 0), 0);
    }

    #[test]
    fn killer_store_is_read_back() {
        // The killer slots are engine-global: run alone (crate::test_lock).
        let _serial = crate::test_lock::lock();
        heuristics::killer_store(10, 0xABCD);
        assert_eq!(heuristics::killer_score(10, 0xABCD), params::KILLER1_SCORE);
    }
}
