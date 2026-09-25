//! Root search: one iteration at a fixed depth, with aspiration-window
//! refinement around the previous iteration's best score.
//!
//! ## Root move list caching
//! Full root breadth: ALL root moves are ranked and searched every iteration
//! (~500+). The root is a single node, so the list is a negligible fraction of
//! the tree, and a narrow root beam would discard most candidate moves.
//! Internal nodes keep their own beams, so depth is preserved by pruning
//! BELOW the root.
//!
//! What the root must NOT do is rebuild that list from scratch every
//! iteration. It used to: each iteration generated the captures, scored and
//! sorted them, then generated the ~700 quiets, scored and sorted those too —
//! four `Vec` allocations plus ~1_400 ordering-score computations per
//! iteration, repeated for every depth. [`RootState`] instead caches the move
//! list for the position and keeps a best-first permutation across
//! iterations: iteration *n+1* searches the moves in the order that iteration
//! *n* proved good, so the alpha-beta window is tightened as early as
//! possible and no move list is ever regenerated (unless the root position
//! itself changed).

use super::buffers::SearchPool;
use super::heuristics::Heuristics;
use super::ordering::{self, piece_vals};
use super::params;
use super::pvs;
use super::tt::Tt;
use crate::board::Board;
use crate::eval::{evaluate, MATE_SCORE};
use crate::movegen::generate_pseudo_legal_moves_into;
use crate::types::*;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

/// Cached root move list plus the ordering carried over from the previous
/// iteration. Owned by the searcher, so the buffers survive for the whole
/// iterative-deepening loop (a caller that keeps one between moves — e.g. a
/// future Lazy SMP master — gets the same benefit).
pub(crate) struct RootState {
    /// Zobrist hash of the position the list was built for.
    hash: u64,
    /// Every pseudo-legal root move, in generation order.
    moves: Vec<Move>,
    /// Last known score per move (`moves[i]`), used as the ordering key.
    scores: Vec<i32>,
    /// Permutation of `moves`, best first.
    order: Vec<u32>,
}

impl RootState {
    pub fn new() -> Self {
        RootState { hash: 0, moves: Vec::new(), scores: Vec::new(), order: Vec::new() }
    }

    /// Number of cached root moves (diagnostics / tests).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// Rebuild the list if the root position changed; otherwise keep the list
    /// and the ordering produced by the previous iteration.
    fn prepare(&mut self, board: &Board, tt_move: u32, depth: u32, root_hint: Option<u32>,
               heur: &Heuristics) {
        if self.hash != board.hash || self.moves.is_empty() {
            self.hash = board.hash;
            generate_pseudo_legal_moves_into(&mut self.moves, board);
            self.scores.clear();
            self.scores.resize(self.moves.len(), 0);
            let stm = board.side_to_move;
            for (i, m) in self.moves.iter().enumerate() {
                let packed = ordering::m_pack(m);
                let hist = heur.history_score(m.from_sq as usize, m.to_sq as usize, stm);
                let mut s = ordering::score_move(heur, m, tt_move, hist, 0, depth);
                if root_hint == Some(packed) { s += params::ROOT_HINT_SCORE; }
                self.scores[i] = s;
            }
        }
        self.order.clear();
        self.order.extend(0..self.moves.len() as u32);
        self.sort_best_first();
    }

    /// Stable descending sort by cached score: the best move of the last
    /// iteration is searched first, which is what makes the root alpha-beta
    /// window as tight as it can be from move one.
    fn sort_best_first(&mut self) {
        let scores = &self.scores;
        self.order.sort_by(|&a, &b| scores[b as usize].cmp(&scores[a as usize]));
    }
}

/// Root-search bookkeeping shared by every root move of one iteration.
pub(crate) struct RootCtx<'a> {
    pub nodes: &'a mut u64,
    pub deadline: Option<Instant>,
    pub stop: &'a AtomicBool,
    pub pool: &'a mut SearchPool,
    pub root: &'a mut RootState,
    /// Transposition table of the owning searcher (shared by SMP workers).
    pub tt: &'a Tt,
    /// Heuristic tables of the owning searcher (one set per worker).
    pub heur: &'a mut Heuristics,
}

/// Aspiration-window search of the root at one depth.
pub(crate) fn search_root_window(
    board: &mut Board,
    depth: u32,
    rc: &mut RootCtx,
    root_hint: Option<u32>,
    root_alpha: i32,
    root_beta: i32,
) -> super::SearchResult {
    let start = Instant::now();
    piece_vals();
    let deadline = rc.deadline;
    let stop = rc.stop;
    let nodes = &mut *rc.nodes;
    let pool = &mut *rc.pool;
    let root = &mut *rc.root;
    let tt = rc.tt;
    let heur = &mut *rc.heur;

    if depth == 0 {
        return super::SearchResult { best_move: None, score: evaluate(board), nodes: 1, time_ms: 0 };
    }

    // Historical note: depths 1-3 used to take a "material-delta fast path"
    // that scored each root move on its material change alone, ignoring the
    // opponent's reply — which made "depth 2/3" behave like depth 1 and
    // silently invalidated every strength comparison across depths. The
    // shortcut was removed outright instead of being left behind as a switch:
    // every depth runs the real alpha-beta root below.
    let mut best_move = None;
    let mut best_score = -MATE_SCORE - 1;
    let root_tt_move = tt.probe(board.hash).map(|e| e.best_move).unwrap_or(0);

    // A depth-2 preliminary search warms the TT and gives the ordering a
    // cheap head start before the real iteration.
    if depth > 2 {
        let _ = pvs::pvs_with_pool(board, depth - 2, -MATE_SCORE - 1, MATE_SCORE + 1,
                                   nodes, deadline, 0, 0, stop, pool, tt, heur);
    }

    // ── ROOT MOVE LIST (cached across iterations) ──────────────
    // Rebuilt only when the root position changed; otherwise the moves keep
    // the order the previous iteration discovered, which is the best possible
    // move ordering for the root (the returned best move is tried first).
    root.prepare(board, root_tt_move, depth, root_hint, heur);
    if root.moves.is_empty() {
        // No legal move at all: the side to move loses (SPEC §7.3).
        let score = -(MATE_SCORE - depth as i32);
        return super::SearchResult {
            best_move: None, score, nodes: *nodes, time_ms: start.elapsed().as_millis() as u64,
        };
    }

    // ── SEARCH EVERY ROOT MOVE, BEST-FIRST ─────────────────────
    // The list is not regenerated and not re-scored here: it comes from the
    // cache, already ordered by the previous iteration's results. Captures
    // naturally sort ahead of quiets because they scored higher last time (and
    // MVV-LVA seeded them on the first iteration), so the historical
    // "captures first" staging is preserved without a second generation pass.
    // (The material-delta fast path above is disabled by default, so every
    // depth from 1 upwards runs this loop — no per-depth branch is needed.)
    let n_moves = root.moves.len();
    for rank in 0..n_moves {
        if deadline.map(|dl| Instant::now() >= dl).unwrap_or(false) { break; }
        if stop.load(std::sync::atomic::Ordering::Relaxed) { break; }
        let idx = root.order[rank] as usize;
        let m = &root.moves[idx];
        let packed = ordering::m_pack(m);
        board.apply_move(m);
        *nodes += 1;
        let (sa, sb) = if rank == 0 && best_score > root_alpha + params::ROOT_WINDOW_REFINE_GATE {
            (best_score - params::ROOT_WINDOW_REFINE, best_score + params::ROOT_WINDOW_REFINE)
        } else {
            (-MATE_SCORE - 1, -best_score.max(-MATE_SCORE - 1))
        };
        let score = if rank == 0 {
            -pvs::pvs_with_pool(board, depth - 1, sa, sb, nodes, deadline, 0, packed, stop, pool, tt, heur)
        } else {
            let nw = -pvs::pvs_with_pool(board, depth - 1, -sa - 1, -sa, nodes, deadline, 0,
                                         packed, stop, pool, tt, heur);
            if nw > sa && nw < sb {
                -pvs::pvs_with_pool(board, depth - 1, -sb, -sa, nodes, deadline, 0,
                                    packed, stop, pool, tt, heur)
            } else { nw }
        };
        let score = if score <= sa || score >= sb {
            let full = -pvs::pvs_with_pool(board, depth - 1, -MATE_SCORE - 1,
                                           -best_score.max(-MATE_SCORE - 1),
                                           nodes, deadline, 0, packed, stop, pool, tt, heur);
            if full > best_score { best_score = full; best_move = Some(m.clone()); }
            full
        } else {
            if score > best_score {
                best_score = score;
                best_move = Some(m.clone());
            }
            score
        };
        // Feed the result back into the root ordering: next iteration tries
        // the moves in decreasing order of THIS iteration's score.
        root.scores[idx] = score;
        board.undo_move();
        if best_score >= root_beta { break; }
    }
    root.sort_best_first();

    if best_move.is_none() {
        // The budget (or a stop) expired before a single root move was scored.
        // Returning None would tell the caller this side has no legal move at
        // all — a loss, SPEC §7.3 — so hand back the first move of the root
        // ordering together with the static score: the best guess available.
        if let Some(&i) = root.order.first() {
            best_move = Some(root.moves[i as usize].clone());
            best_score = evaluate(board);
        }
    }

    super::SearchResult {
        best_move,
        score: best_score,
        nodes: *nodes,
        time_ms: start.elapsed().as_millis() as u64,
    }
}

