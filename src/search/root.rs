#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

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
//! four `Vec` allocations plus ~`1_400` ordering-score computations per
//! iteration, repeated for every depth. [`RootState`] instead caches the move
//! list for the position and keeps a best-first permutation across
//! iterations: iteration *n+1* searches the moves in the order that iteration
//! *n* proved good, so the alpha-beta window is tightened as early as
//! possible and no move list is ever regenerated (unless the root position
//! itself changed).

use super::buffers::SearchPool;
use super::heuristics::Heuristics;
use super::ordering::{self, piece_vals};
use super::params::{self, SearchConfig};
use super::pvs;
use super::tt::Tt;
use crate::board::Board;
use crate::eval::{MATE_SCORE, evaluate};
use crate::movegen::generate_pseudo_legal_moves_into;
use crate::types::Move;
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
    /// Optional packed-move whitelist (`go searchmoves`). A change forces a
    /// rebuild of the cached list.
    filter: Option<Vec<u32>>,
}

impl RootState {
    pub fn new() -> Self {
        RootState {
            hash: 0,
            moves: Vec::new(),
            scores: Vec::new(),
            order: Vec::new(),
            filter: None,
        }
    }

    /// Restrict the root moves to `filter` (packed keys). `None` clears it.
    pub fn set_filter(&mut self, filter: Option<Vec<u32>>) {
        if self.filter != filter {
            self.hash = 0; // force regeneration on the next prepare()
        }
        self.filter = filter;
    }

    /// Number of cached root moves (diagnostics / tests).
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.moves.len()
    }

    /// Rebuild the list if the root position changed; otherwise keep the list
    /// and the ordering produced by the previous iteration.
    #[allow(clippy::too_many_arguments)]
    // Hot movegen / search path: a parameters struct would add indirection without
    // improving readability, so the flat signature is kept on purpose.
    fn prepare(
        &mut self,
        board: &Board,
        tt_move: u32,
        depth: u32,
        root_hint: Option<u32>,
        heur: &Heuristics,
        order_offset: usize,
    ) {
        if self.hash != board.hash || self.moves.is_empty() {
            self.hash = board.hash;
            generate_pseudo_legal_moves_into(&mut self.moves, board);
            if let Some(filter) = &self.filter {
                self.moves.retain(|m| filter.contains(&ordering::m_pack(m)));
                if self.moves.is_empty() {
                    // No requested move is legal here: search everything
                    // rather than reporting a bogus "no legal moves" loss.
                    generate_pseudo_legal_moves_into(&mut self.moves, board);
                }
            }
            self.scores.clear();
            self.scores.resize(self.moves.len(), 0);
            let stm = board.side_to_move;
            let values = ordering::piece_vals();
            for (i, m) in self.moves.iter().enumerate() {
                let packed = ordering::m_pack(m);
                let hist = heur.history_score(m.from_sq as usize, m.to_sq as usize, stm);
                let mut s = ordering::score_move(heur, m, tt_move, hist, 0, depth, values);
                if root_hint == Some(packed) {
                    s += params::ROOT_HINT_SCORE;
                }
                self.scores[i] = s;
            }
        }
        self.order.clear();
        self.order.extend(0..self.moves.len() as u32);
        self.sort_best_first();
        // Lazy SMP diversity: worker `i` starts from a different point in the
        // (still best-first sorted) list. The master keeps offset 0, so its own
        // ordering — and therefore its result quality — is unaffected.
        if order_offset > 0 && !self.order.is_empty() {
            let by = order_offset % self.order.len();
            self.order.rotate_left(by);
        }
    }

    /// Stable descending sort by cached score: the best move of the last
    /// iteration is searched first, which is what makes the root alpha-beta
    /// window as tight as it can be from move one.
    fn sort_best_first(&mut self) {
        let scores = &self.scores;
        self.order
            .sort_by(|&a, &b| scores[b as usize].cmp(&scores[a as usize]));
    }
}

/// Root-search bookkeeping shared by every root move of one iteration.
pub(crate) struct RootCtx<'a> {
    /// Technique on/off switches (see [`super::params::SearchConfig`]).
    pub cfg: SearchConfig,
    pub nodes: &'a mut u64,
    pub deadline: Option<Instant>,
    pub stop: &'a AtomicBool,
    pub pool: &'a mut SearchPool,
    pub root: &'a mut RootState,
    /// Transposition table of the owning searcher (shared by SMP workers).
    pub tt: &'a Tt,
    /// Heuristic tables of the owning searcher (one set per worker).
    pub heur: &'a mut Heuristics,
    /// Root move list rotation: 0 for the master, `i` for Lazy SMP worker `i`.
    pub order_offset: usize,
}

/// Aspiration-window search of the root at one depth.
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
/// Running best-move state during a root iteration.
struct RootSearchState<'a> {
    best_score: i32,
    best_move: &'a mut Option<Move>,
    root_beta: i32,
}

/// Per-move context for one root search.
struct RootMoveCtx {
    rank: usize,
    idx: usize,
    best_score: i32,
    root_alpha: i32,
}

/// Context shared while searching the root moves of one iteration.
struct RootSearch<'a> {
    board: &'a mut Board,
    deadline: Option<Instant>,
    stop: &'a AtomicBool,
    nodes: &'a mut u64,
    pool: &'a mut SearchPool,
    tt: &'a Tt,
    heur: &'a mut Heuristics,
    cfg: SearchConfig,
}

impl RootSearch<'_> {
    /// Warm the TT with a shallower full-window search before the real iteration.
    fn warm_up(&mut self, depth: u32) {
        let _ = pvs::pvs_with_pool(
            self.board,
            depth - 2,
            -MATE_SCORE - 1,
            MATE_SCORE + 1,
            self.nodes,
            self.deadline,
            0,
            0,
            self.stop,
            self.pool,
            self.tt,
            self.heur,
            self.cfg,
        );
    }

    /// Search one ranked root move, update the running best, and report
    /// whether the beta cutoff ended the iteration.
    fn search_rank(
        &mut self,
        root: &mut RootState,
        depth: u32,
        ctx: &mut RootMoveCtx,
        state: &mut RootSearchState<'_>,
    ) -> bool {
        let idx = ctx.idx;
        let m = root.moves[idx];
        let packed = ordering::m_pack(&m);
        self.board.apply_move(&m);
        *self.nodes += 1;
        ctx.best_score = state.best_score;
        let score = self.search_move(depth, packed, ctx);
        if score > state.best_score {
            state.best_score = score;
            *state.best_move = Some(m);
        }
        root.scores[idx] = score;
        self.board.undo_move();
        state.best_score >= state.root_beta
    }

    /// Search one root move with PVS and window refinement. Returns its score.
    fn search_move(&mut self, depth: u32, packed: u32, ctx: &RootMoveCtx) -> i32 {
        let (rank, best_score, root_alpha) = (ctx.rank, ctx.best_score, ctx.root_alpha);
        let (sa, sb) = if rank == 0 && best_score > root_alpha + params::ROOT_WINDOW_REFINE_GATE {
            (
                best_score - params::ROOT_WINDOW_REFINE,
                best_score + params::ROOT_WINDOW_REFINE,
            )
        } else {
            (-MATE_SCORE - 1, -best_score.max(-MATE_SCORE - 1))
        };
        let mut search = |a: i32, b: i32| {
            pvs::pvs_with_pool(
                self.board,
                depth - 1,
                a,
                b,
                self.nodes,
                self.deadline,
                0,
                packed,
                self.stop,
                self.pool,
                self.tt,
                self.heur,
                self.cfg,
            )
        };
        let score = if rank == 0 {
            -search(sa, sb)
        } else {
            let nw = -search(-sa - 1, -sa);
            if nw > sa && nw < sb {
                -search(-sb, -sa)
            } else {
                nw
            }
        };
        if score <= sa || score >= sb {
            -search(-MATE_SCORE - 1, -best_score.max(-MATE_SCORE - 1))
        } else {
            score
        }
    }
}

/// Static-eval-only result for a zero-depth root search.
fn zero_depth_result(board: &Board) -> super::SearchResult {
    super::SearchResult {
        best_move: None,
        score: evaluate(board),
        nodes: 1,
        time_ms: 0,
    }
}

/// Result when the position has no legal root move (SPEC §7.3: a loss).
fn no_moves_result(depth: u32, nodes: u64, start: Instant) -> super::SearchResult {
    super::SearchResult {
        best_move: None,
        score: -(MATE_SCORE - depth as i32),
        nodes,
        time_ms: start.elapsed().as_millis() as u64,
    }
}

/// Aspiration window for a root iteration.
pub(crate) type RootWindow = (i32, i32);

pub(crate) fn search_root_window(
    board: &mut Board,
    depth: u32,
    rc: &mut RootCtx,
    root_hint: Option<u32>,
    window: RootWindow,
) -> super::SearchResult {
    let (root_alpha, root_beta) = window;
    let start = Instant::now();
    piece_vals();
    let deadline = rc.deadline;
    let stop = rc.stop;
    let nodes = &mut *rc.nodes;
    let pool = &mut *rc.pool;
    let root = &mut *rc.root;
    let tt = rc.tt;
    let heur = &mut *rc.heur;
    let order_offset = rc.order_offset;
    let cfg = rc.cfg;

    if depth == 0 {
        return zero_depth_result(board);
    }

    // Every depth runs the real alpha-beta root: the old depth 1-3
    // material-delta shortcut was removed (it made shallow depths lie).
    let mut best_move = None;
    let mut best_score = -MATE_SCORE - 1;
    let root_tt_move = tt.probe(board.tt_key()).map_or(0, |e| e.best_move);

    // A depth-2 preliminary search warms the TT and the move ordering.
    if depth > 2 {
        RootSearch {
            board,
            deadline,
            stop,
            nodes,
            pool,
            tt,
            heur,
            cfg,
        }
        .warm_up(depth);
    }

    // ── ROOT MOVE LIST (cached across iterations) ──────────────
    // Rebuilt only when the root position changed; otherwise the moves keep
    // the order the previous iteration discovered, which is the best possible
    // move ordering for the root (the returned best move is tried first).
    root.prepare(board, root_tt_move, depth, root_hint, heur, order_offset);
    if root.moves.is_empty() {
        return no_moves_result(depth, *nodes, start);
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
        if deadline.is_some_and(|dl| Instant::now() >= dl) {
            break;
        }
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        let idx = root.order[rank] as usize;
        let mut rs = RootSearch {
            board,
            deadline,
            stop,
            nodes,
            pool,
            tt,
            heur,
            cfg,
        };
        let mut state = RootSearchState {
            best_score,
            best_move: &mut best_move,
            root_beta,
        };
        let mut move_ctx = RootMoveCtx {
            rank,
            idx,
            best_score,
            root_alpha,
        };
        let cutoff = rs.search_rank(root, depth, &mut move_ctx, &mut state);
        best_score = state.best_score;
        if cutoff {
            break;
        }
    }
    root.sort_best_first();

    if best_move.is_none() {
        // Budget/stop expired before scoring any move: return the first
        // ordered move with the static score (None would mean "no moves").
        if let Some((m, s)) = fallback_move(root, board) {
            best_move = Some(m);
            best_score = s;
        }
    }
    make_result(best_move, best_score, *nodes, start)
}

/// A best-guess move and its static score.
type Fallback = (Move, i32);

fn fallback_move(root: &RootState, board: &Board) -> Option<Fallback> {
    let i = *root.order.first()?;
    Some((root.moves[i as usize], evaluate(board)))
}

fn make_result(
    best_move: Option<Move>,
    score: i32,
    nodes: u64,
    start: Instant,
) -> super::SearchResult {
    super::SearchResult {
        best_move,
        score,
        nodes,
        time_ms: start.elapsed().as_millis() as u64,
    }
}
