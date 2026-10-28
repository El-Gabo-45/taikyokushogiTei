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
//!   across iterations.
//!
//! Iterative deepening with predictive time management lives in [`search`]
//! below.
//!
//! NOTE: this variant has NO check and NO checkmate (SPEC §7.3): the game
//! only ends when a side captures the opponent's LAST royal (SPEC §7.2).

mod buffers;
mod heuristics;
mod ordering;
pub mod params;
mod pvs;
mod qsearch;
mod root;
mod smp;
mod tt;

use self::buffers::SearchPool;
use self::heuristics::Heuristics;
use self::params::SearchConfig;
use self::root::RootState;
use self::smp::SharedBest;
use self::tt::Tt;
use crate::board::Board;
use crate::eval::{evaluate, MATE_SCORE};
use crate::types::Move;
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

/// Result of a completed search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u64,
}

/// A complete search engine in a box: transposition table, killer/history
/// tables, per-ply scratch pool, cached root move list and cooperative stop
/// flag.
///
/// Owning this is what makes the search reentrant. Two searchers — two threads,
/// two tests, a UI searching while a match runner works — never see each
/// other's state, and a caller that keeps its searcher alive between moves
/// keeps the tables warm. It replaces the process-global statics the search
/// used to hide behind.
pub struct Searcher {
    /// Technique on/off switches (see [`SearchConfig`]).
    cfg: SearchConfig,
    /// Shared with Lazy SMP workers through an `Arc` clone (see
    /// [`Searcher::share_tt`]): the table is the one piece of state workers are
    /// *supposed* to pool.
    tt: Arc<Tt>,
    heur: Heuristics,
    pool: SearchPool,
    root: RootState,
    /// Lazy SMP workers start the root move list at a different position so
    /// they do not all walk the same tree in lockstep. The master keeps 0
    /// (pure best-first ordering); worker `i` uses `i`.
    order_offset: usize,
}

impl Searcher {
    /// Creates a searcher with the production config (all techniques enabled).
    pub fn new() -> Self {
        Self::with_config(SearchConfig::production())
    }

    /// Creates a searcher with an explicit [`SearchConfig`]. Use this to
    /// enable/disable individual techniques for testing and A/B experiments:
    ///
    /// ```
    /// use taikyokushogi::search::Searcher;
    /// use taikyokushogi::search::params::SearchConfig;
    /// let cfg = SearchConfig::minimal();
    /// let s = Searcher::with_config(cfg);
    /// ```
    pub fn with_config(cfg: SearchConfig) -> Self {
        Searcher {
            cfg,
            tt: Arc::new(Tt::new()),
            heur: Heuristics::new(),
            pool: SearchPool::new(),
            root: RootState::new(),
            order_offset: 0,
        }
    }

    /// Returns a reference to this searcher's technique config.
    pub fn config(&self) -> SearchConfig {
        self.cfg
    }

    /// A second searcher that shares this one's transposition table but owns
    /// its heuristics, scratch pool and root cache, inheriting the same config.
    ///
    /// This is the shape Lazy SMP wants (workers pool what they learn into the
    /// shared table), and it is also how a self-play pool avoids allocating one
    /// table per worker.
    pub fn share_tt(&self) -> Self {
        Searcher {
            cfg: self.cfg,
            tt: Arc::clone(&self.tt),
            heur: Heuristics::new(),
            pool: SearchPool::new(),
            root: RootState::new(),
            order_offset: 0,
        }
    }

    /// Resize this searcher's transposition table to `mb` MiB; returns the size
    /// actually allocated (rounded down to a power-of-two bucket count). Safe
    /// to call at any time, including while a search is running.
    pub fn set_hash_mb(&self, mb: usize) -> usize {
        self.tt.resize_mb(mb)
    }

    /// This searcher's current transposition-table size in MiB.
    pub fn hash_mb(&self) -> usize {
        self.tt.size_mb()
    }

    /// Drop everything this searcher learned — the "new game" hook: the
    /// transposition table, the killer/history tables and the cached root move
    /// list. Deterministic tests call it between runs.
    pub fn clear(&mut self) {
        self.tt.clear();
        self.heur.clear();
        self.root = RootState::new();
    }

    /// Full search: iterative deepening with aspiration windows and predictive
    /// time management. Single-threaded; see [`Searcher::search_threads`] for
    /// the Lazy SMP variant.
    pub fn search(&mut self, board: &mut Board, depth: u32, time_limit_ms: u64) -> SearchResult {
        let deadline = deadline_from(time_limit_ms);
        // A searcher used on its own owns its stop flag; the workers of a Lazy
        // SMP group share the master's instead (see `search_threads`).
        let stop = AtomicBool::new(false);
        self.search_impl(board, depth, time_limit_ms, deadline, &stop, None).0
    }

    /// The iterative-deepening loop behind [`Searcher::search`] and
    /// [`Searcher::search_threads`].
    ///
    /// `stop` is supplied by the caller so Lazy SMP workers can share one flag,
    /// and `publish` receives every completed iteration so the SMP master can
    /// adopt a worker's deeper result. Returns the result plus the depth of the
    /// last completed iteration (0 when none finished).
    fn search_impl(&mut self, board: &mut Board, depth: u32, time_limit_ms: u64,
                   deadline: Option<Instant>, stop: &AtomicBool,
                   publish: Option<&SharedBest>) -> (SearchResult, u32) {
        let start = Instant::now();
        // env::var takes a global lock — cache it once per search, not per iteration.
        let debug_log = std::env::var_os("RPS_DEBUG").is_some();

        self.tt.new_generation();
        ordering::piece_vals();
        let mut best_result =
            SearchResult { best_move: None, score: evaluate(board), nodes: 0, time_ms: 0 };
        // `nodes` is cumulative for the whole search; the scratch pool, the
        // cached root move list and the worker's own heuristics tables live on
        // `self`, so they survive between iterations — and between searches
        // when the caller keeps the searcher.
        let mut nodes: u64 = 0;
        let mut root_hint: Option<u32> = None;
        let mut score_guess = best_result.score;
        let mut prev_iter_ms: u64 = 0;
        let mut last_depth: u32 = 0;

        if depth == 0 {
            let r = SearchResult { best_move: None, score: evaluate(board), nodes: 1, time_ms: 0 };
            return (r, 0);
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
            cfg: self.cfg,
            nodes: &mut nodes,
            deadline,
            stop,
            pool: &mut self.pool,
            root: &mut self.root,
            tt: &self.tt,
            heur: &mut self.heur,
            order_offset: self.order_offset,
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
        root_hint = result.best_move.as_ref().map(ordering::m_pack);
        score_guess = result.score;
        prev_iter_ms = result.time_ms;
        best_result = result;
        last_depth = current_depth;
        // Tell the Lazy SMP master that this iteration is done, so it can adopt
        // our result if it is deeper than what the master has reached itself.
        if let Some(shared) = publish {
            if let Some(m) = best_result.best_move.as_ref() {
                shared.publish(current_depth, best_result.score, ordering::m_pack(m));
            }
        }
    }

        let elapsed = start.elapsed().as_millis() as u64;
        if let Some(shared) = publish {
            shared.add_nodes(nodes);
        }
        let result = SearchResult {
            best_move: best_result.best_move,
            score: best_result.score,
            nodes: nodes.max(best_result.nodes),
            time_ms: elapsed,
        };
        (result, last_depth)
    }
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

/// A `time_limit_ms` budget turned into an absolute deadline (`None` = no
/// limit). Lazy SMP workers share the master's deadline so they all stop on
/// the same wall-clock moment.
fn deadline_from(time_limit_ms: u64) -> Option<Instant> {
    if time_limit_ms > 0 {
        Some(Instant::now() + std::time::Duration::from_millis(time_limit_ms))
    } else {
        None
    }
}

thread_local! {
    /// The searcher behind the free [`search`] entry point: one per thread, so
    /// the tables persist across calls on the same thread (which is what keeps
    /// the engine fast across moves in a game loop) while two threads never
    /// touch each other's state.
    static DEFAULT_SEARCHER: RefCell<Searcher> = RefCell::new(Searcher::new());
}

/// Search `board` with the calling thread's default [`Searcher`].
///
/// Use a [`Searcher`] you own when you want to control the transposition-table
/// size, share the table with other workers, or hold the learned state
/// explicitly (see [`Searcher::clear`] for the "new game" case).
pub fn search(board: &mut Board, depth: u32, time_limit_ms: u64) -> SearchResult {
    DEFAULT_SEARCHER.with(|s| s.borrow_mut().search(board, depth, time_limit_ms))
}

/// Resize the default (per-thread) searcher's transposition table to `mb` MiB.
/// See [`Searcher::set_hash_mb`] for a searcher you own.
pub fn set_hash_mb(mb: usize) -> usize {
    DEFAULT_SEARCHER.with(|s| s.borrow().set_hash_mb(mb))
}

/// The default (per-thread) searcher's transposition-table size in MiB.
pub fn hash_mb() -> usize {
    DEFAULT_SEARCHER.with(|s| s.borrow().hash_mb())
}

/// Search `board` with the calling thread's default [`Searcher`] using Lazy
/// SMP: `threads` workers (1 = plain [`search`], clamped to 1..=[`MAX_THREADS`])
/// share the default searcher's transposition table.
///
/// Like [`search`], the tables persist across calls on this thread. Results
/// with `threads > 1` are not bit-reproducible (see
/// [`Searcher::search_threads`]).
pub fn search_threads(board: &mut Board, depth: u32, time_limit_ms: u64,
                      threads: usize) -> SearchResult {
    DEFAULT_SEARCHER.with(|s| s.borrow_mut().search_threads(board, depth, time_limit_ms, threads))
}

// ── Lazy SMP tuning surface (the numbers callers may rely on) ────
/// Default worker count: 1 keeps the plain single-threaded search (and stays
/// deterministic).
pub use self::params::DEFAULT_THREADS;
/// Hard cap on Lazy SMP worker threads; `search_threads` clamps to it.
pub use self::params::MAX_THREADS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Board;
    use crate::types;

    #[test]
    fn search_initial_reaches_depth_and_finds_a_move() {
        let mut board = Board::initial();
        let r = board.search(4, 0);
        assert!(r.best_move.is_some(), "depth-4 search must return a move");
        assert!(r.nodes > 0);
        assert!(r.score.abs() < MATE_SCORE, "eval must not look like mate");
    }

    #[test]
    fn hash_size_is_configurable_and_reported() {
        let s = Searcher::new();
        let before = s.hash_mb();
        let actual = s.set_hash_mb(8);
        assert_eq!(actual, s.hash_mb(), "the reported size must be the allocated one");
        assert!(actual >= params::TT_MIN_MB);
        assert_eq!(s.set_hash_mb(before), s.hash_mb());
        assert_eq!(s.hash_mb(), before, "restoring the previous size must round-trip");
    }

    #[test]
    fn shared_tt_searchers_see_the_same_table() {
        let master = Searcher::new();
        let worker = master.share_tt();
        let mb = master.set_hash_mb(16);
        assert_eq!(worker.hash_mb(), mb, "workers must see the shared table");
    }

    #[test]
    fn history_clear_zeroes_counters() {
        let mut h = Heuristics::new();
        h.history_store(3, 4, 5, 0);
        assert!(h.history_score(3, 4, 0) > 0);
        h.clear();
        assert_eq!(h.history_score(3, 4, 1), 0);
        assert_eq!(h.history_score(0, 0, 0), 0);
    }

    #[test]
    fn killer_store_is_read_back() {
        let mut h = Heuristics::new();
        h.killer_store(10, 0xABCD);
        assert_eq!(h.killer_score(10, 0xABCD), params::KILLER1_SCORE);
        assert_eq!(h.killer_score(10, 0x1234), 0);
    }

    // ── Reproduction tests for the audit-fix items ──────────────────

    /// A move applied through `apply_by_coord` with out-of-range coordinates
    /// must return `false` without panicking (the audit fix validates
    /// coordinates before any indexing).
    #[test]
    fn apply_by_coord_out_of_bounds_returns_false_no_panic() {
        let mut board = Board::initial();
        for &(fr, fc, tr, tc) in &[
            (36, 0, 35, 0), (0, 36, 35, 0), (35, 0, 36, 0), (35, 0, 35, 36),
            (100, 100, 101, 101),
        ] {
            // If this panics, the test fails — which is exactly what we want
            // to catch: out-of-range coordinates must be rejected before any
            // indexing occurs.
            assert!(!board.apply_by_coord(fr, fc, tr, tc, false, None),
                    "apply_by_coord({fr},{fc}->{tr},{tc}) must return false");
        }
        assert_eq!(board.side_to_move(), crate::Color::Black);
    }

    /// A valid move from the initial position is accepted.
    #[test]
    fn apply_by_coord_edge_coordinates_accepted() {
        let mut board = Board::initial();
        let moves = board.legal_moves();
        assert!(!moves.is_empty(), "initial position must have legal moves");
        let m = &moves[0];
        let fr = m.inner.from_sq as usize / types::BOARD_SIZE;
        let fc = m.inner.from_sq as usize % types::BOARD_SIZE;
        let tr = m.inner.to_sq as usize / types::BOARD_SIZE;
        let tc = m.inner.to_sq as usize % types::BOARD_SIZE;
        assert!(board.apply_by_coord(fr, fc, tr, tc, false, None),
                "first legal move from initial position must be accepted");
        assert_eq!(board.side_to_move(), crate::Color::White);
    }

    /// `apply_by_coord` with `mid_sq` matches only moves whose `mid_sq` equals
    /// the given value, and rejects moves whose `mid_sq` differs.
    #[test]
    fn apply_by_coord_mid_sq_matches_and_rejects() {
        let mut board = Board::initial();
        let moves = board.legal_moves();
        assert!(!moves.is_empty(), "initial position must have legal moves");
        let m = &moves[0];
        let fr = m.inner.from_sq as usize / types::BOARD_SIZE;
        let fc = m.inner.from_sq as usize % types::BOARD_SIZE;
        let tr = m.inner.to_sq as usize / types::BOARD_SIZE;
        let tc = m.inner.to_sq as usize % types::BOARD_SIZE;
        assert!(board.apply_by_coord(fr, fc, tr, tc, false, None),
                "mid_sq=None must match a normal move");
        board.undo();
        assert!(!board.apply_by_coord(fr, fc, tr, tc, false, Some(500)),
                "mid_sq=Some(500) must not match a normal move (mid_sq=NONE)");
        assert_eq!(board.side_to_move(), crate::Color::Black);
    }

    /// `Move::mid_sq()` maps `INVALID_SQ` to `None` and a real square to
    /// `Some(sq)`. All initial moves are quiet/pawn pushes: mid_sq=None.
    #[test]
    fn move_mid_sq_maps_invalid_to_none() {
        let board = Board::initial();
        let moves = board.legal_moves();
        assert!(!moves.is_empty(), "initial position must have legal moves");
        for m in &moves {
            assert_eq!(m.mid_sq(), None,
                       "initial move {:?} must have mid_sq=None", m);
        }
    }

    // ── SearchConfig on/off switching tests ─────────────────────────

    /// Every technique flag can be toggled independently; a search with all
    /// techniques off (minimal config) still returns a best move.
    #[test]
    fn search_minimal_config_returns_a_move() {
        let mut board = Board::initial();
        let mut searcher = Searcher::with_config(SearchConfig::minimal());
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some(),
                "minimal-config search must return a best move");
        assert!(r.nodes > 0);
    }

    /// A search with all techniques on (production config) returns a best move.
    #[test]
    fn search_production_config_returns_a_move() {
        let mut board = Board::initial();
        let mut searcher = Searcher::with_config(SearchConfig::production());
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
        assert!(r.nodes > 0);
        assert!(r.score.abs() < crate::eval::MATE_SCORE);
    }

    /// Disabling TT must not crash and must still produce a move.
    #[test]
    fn search_without_tt_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.transposition_table = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling null-move pruning must not crash and must still produce a move.
    #[test]
    fn search_without_null_move_pruning_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.null_move_pruning = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling LMR must not crash and must still produce a move.
    #[test]
    fn search_without_lmr_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.late_move_reductions = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling all shallow-eval pruning (razoring, RFP, futility) must not crash.
    #[test]
    fn search_without_shallow_pruning_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.razoring = false;
        cfg.reverse_futility = false;
        cfg.futility = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling quiescence must not crash at depth 0 (leaf eval used instead).
    #[test]
    fn search_without_quiescence_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.quiescence = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 1, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling capture beam searches all captures (more nodes, no crash).
    #[test]
    fn search_without_capture_beam_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.capture_beam = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 2, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling hash-move extension must not crash.
    #[test]
    fn search_without_hash_move_extension_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.hash_move_extension = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling IID must not crash.
    #[test]
    fn search_without_iid_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.internal_iterative_deepening = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling ProbCut must not crash.
    #[test]
    fn search_without_probcut_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.probcut = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// Disabling LMP must not crash.
    #[test]
    fn search_without_lmp_still_works() {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        cfg.late_move_pruning = false;
        let mut searcher = Searcher::with_config(cfg);
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
    }

    /// `SearchConfig::classic()` (TT + ordering + reductions, no pruning) works.
    #[test]
    fn search_classic_config_returns_a_move() {
        let mut board = Board::initial();
        let mut searcher = Searcher::with_config(SearchConfig::classic());
        let r = board.search_with(&mut searcher, 3, 0);
        assert!(r.best_move.is_some());
        assert!(r.nodes > 0);
    }
}


    /// `Searcher::with_config` reports the same config via `config()`, and
    /// `share_tt()` inherits it.
    #[test]
    fn searcher_config_is_round_trip_and_shared() {
        let cfg = SearchConfig::minimal();
        let master = Searcher::with_config(cfg);
        assert_eq!(master.config(), cfg);
        let worker = master.share_tt();
        assert_eq!(worker.config(), cfg,
                   "share_tt must inherit the master's config");
    }

    /// `Searcher::new()` uses the production config by default.
    #[test]
    fn searcher_new_uses_production_config() {
        let s = Searcher::new();
        assert_eq!(s.config(), SearchConfig::production());
    }

    // ── Determinism / stability tests ───────────────────────────────

    /// Two fixed-depth searches with the same minimal config produce the same
    /// best move (deterministic, no TT / SMP randomness).
    #[test]
    fn minimal_config_search_is_deterministic() {
        let mut board1 = Board::initial();
        let mut board2 = Board::initial();
        let mut s1 = Searcher::with_config(SearchConfig::minimal());
        let mut s2 = Searcher::with_config(SearchConfig::minimal());
        let r1 = board1.search_with(&mut s1, 3, 0);
        let r2 = board2.search_with(&mut s2, 3, 0);
        assert_eq!(r1.best_move, r2.best_move,
                   "two identical minimal-config searches must agree");
    }

    /// The board is not corrupted by a search: side to move is unchanged.
    #[test]
    fn board_state_unchanged_after_search() {
        let mut board = Board::initial();
        let before = board.side_to_move();
        let mut searcher = Searcher::with_config(SearchConfig::production());
        let _ = board.search_with(&mut searcher, 3, 0);
        assert_eq!(board.side_to_move(), before);
    }

    /// Depth-0 search returns no move but the static eval score.
    #[test]
    fn depth_zero_search_returns_eval_only() {
        let mut board = Board::initial();
        let mut searcher = Searcher::with_config(SearchConfig::production());
        let r = board.search_with(&mut searcher, 0, 0);
        assert!(r.best_move.is_none());
        assert_eq!(r.score, crate::eval::evaluate(&board.inner));
    }
}