//! Lazy SMP: several workers search the same position at the same time.
//!
//! ## Why "lazy"
//! Classic shared-memory search splits a tree and synchronises workers on the
//! critical path, which is complex and scales badly when a single node is
//! enormous. Lazy SMP does the opposite: every worker runs the *entire*
//! iterative-deepening search, sharing the transposition table. Redundant work
//! is accepted on purpose — what the workers really share is knowledge:
//!
//! * a worker that reaches depth *d* fills the table for everyone, so the
//!   master's next iteration is mostly TT hits;
//! * the master adopts a worker's result when that worker completed a *deeper*
//!   iteration than the master managed to finish itself.
//!
//! This engine is a particularly good fit: a node costs ~150 µs on the 36×36
//! board, so any worker synchronisation would be a rounding error, and the
//! per-worker state ([`Searcher`]) is already isolated — only the
//! transposition table is shared, through one `Arc<Tt>`.
//!
//! ## What makes workers different
//! Every worker rotates the root move list by its own index
//! ([`Searcher`]'s `order_offset`). Identical trees would make the workers pure
//! redundancy; a rotation means worker *i* dives into a different (still
//! playable) move first, which is where the TT-filling diversity comes from.

use super::root;
use super::{deadline_from, Searcher};
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

/// Best completed iteration published by any worker of a search group.
///
/// The publisher writes `score`/`packed` first and `depth` last with `Release`;
/// a reader that observes `depth` with `Acquire` therefore sees a consistent
/// triple. Only strictly deeper results are accepted, which keeps the slot
/// monotone without any lock (two workers finishing the same depth do not need
/// to agree on which one wins).
pub(crate) struct SharedBest {
    depth: AtomicU32,
    score: AtomicI32,
    /// `ordering::m_pack` of the best root move (0 = nothing published yet).
    packed: AtomicU64,
    /// Work reported by every worker (for the `nodes` field of the result).
    nodes: AtomicU64,
}

impl SharedBest {
    pub fn new() -> Self {
        SharedBest {
            depth: AtomicU32::new(0),
            score: AtomicI32::new(0),
            packed: AtomicU64::new(0),
            nodes: AtomicU64::new(0),
        }
    }

    #[inline]
    pub fn publish(&self, depth: u32, score: i32, packed: u32) {
        if depth > self.depth.load(Ordering::Relaxed) {
            self.score.store(score, Ordering::Relaxed);
            self.packed.store(packed as u64, Ordering::Relaxed);
            self.depth.store(depth, Ordering::Release);
        }
    }

    #[inline]
    pub fn add_nodes(&self, nodes: u64) {
        self.nodes.fetch_add(nodes, Ordering::Relaxed);
    }

    #[inline]
    pub fn total_nodes(&self) -> u64 {
        self.nodes.load(Ordering::Relaxed)
    }

    /// `(depth, score, packed move)`, or depth 0 when nothing was published.
    #[inline]
    pub fn snapshot(&self) -> (u32, i32, u32) {
        let depth = self.depth.load(Ordering::Acquire);
        (depth, self.score.load(Ordering::Relaxed), self.packed.load(Ordering::Relaxed) as u32)
    }
}

impl Default for SharedBest {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher {
    /// Lazy SMP search with `threads` workers (1 = plain [`Searcher::search`]).
    ///
    /// The calling searcher is the master: its result is authoritative, except
    /// when a worker completed a deeper iteration. Workers share this
    /// searcher's transposition table ([`Searcher::share_tt`]) and the master's
    /// stop flag, and they unwind as soon as the master finishes.
    pub fn search_threads(&mut self, board: &mut super::Board, depth: u32,
                          time_limit_ms: u64, threads: usize) -> super::SearchResult {
        let threads = threads.clamp(1, super::params::MAX_THREADS);
        if threads == 1 {
            return self.search(board, depth, time_limit_ms);
        }
        let deadline = deadline_from(time_limit_ms);
        let stop = std::sync::atomic::AtomicBool::new(false);
        let shared = SharedBest::new();

        let (master, master_depth) = std::thread::scope(|scope| {
            for i in 1..threads {
                let mut worker = self.share_tt();
                worker.order_offset = i;
                // Each worker needs its own cursor into the same position.
                let mut worker_board = board.clone();
                let stop = &stop;
                let shared = &shared;
                scope.spawn(move || {
                    // A worker runs the same deepening loop but never owns the
                    // stop flag: it keeps going until the master (or the
                    // deadline) stops it, publishing every iteration it
                    // completes.
                    worker.search_impl(&mut worker_board, depth, time_limit_ms, deadline,
                                       stop, Some(shared));
                });
            }
            let result = self.search_impl(board, depth, time_limit_ms, deadline, &stop,
                                          Some(&shared));
            // Workers poll this flag on every clock check and unwind promptly.
            stop.store(true, Ordering::Relaxed);
            result
        });

        adopt_deeper_worker_result(board, master, master_depth, &shared)
    }
}

/// Prefer a worker's result when it out-searched the master.
///
/// A worker publishes its move as a packed key, so the caller's own move
/// generation resolves it back into a full [`super::Move`]; if it cannot be
/// found (which cannot happen here — every worker searched this position) the
/// master's result stands.
fn adopt_deeper_worker_result(
    board: &super::Board,
    master: super::SearchResult,
    master_depth: u32,
    shared: &SharedBest,
) -> super::SearchResult {
    let (depth, score, packed) = shared.snapshot();
    let total_nodes = shared.total_nodes().max(master.nodes);
    if depth <= master_depth || packed == 0 {
        return super::SearchResult { nodes: total_nodes, ..master };
    }
    match crate::movegen::generate_pseudo_legal_moves(board)
        .into_iter()
        .find(|m| super::ordering::m_pack(m) == packed)
    {
        Some(m) => super::SearchResult {
            best_move: Some(m),
            score,
            nodes: total_nodes,
            time_ms: master.time_ms,
        },
        None => super::SearchResult { nodes: total_nodes, ..master },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_only_accepts_deeper_results() {
        let b = SharedBest::new();
        assert_eq!(b.snapshot().0, 0, "nothing published yet");
        b.publish(2, 100, 0xAA);
        assert_eq!(b.snapshot(), (2, 100, 0xAA));
        // A shallow result cannot overwrite a deeper one.
        b.publish(1, 999, 0xBB);
        assert_eq!(b.snapshot(), (2, 100, 0xAA));
        // A deeper one can.
        b.publish(3, -50, 0xCC);
        assert_eq!(b.snapshot(), (3, -50, 0xCC));
    }

    #[test]
    fn nodes_accumulate_across_workers() {
        let b = SharedBest::new();
        assert_eq!(b.total_nodes(), 0);
        b.add_nodes(10);
        b.add_nodes(32);
        assert_eq!(b.total_nodes(), 42);
    }
}
