//! Reusable per-ply scratch buffers for the search.
//!
//! ## Why this module exists
//! The search used to allocate on the heap at *every* node: each `pvs()` /
//! `qsearch()` node built a fresh `Vec<Move>` (twice, for the capture and
//! quiet stages) plus a `Vec<(score, idx)>` scoring list. On a tree that
//! visits millions of nodes per move that is millions of `malloc`/`free`
//! pairs — pure allocator traffic in the hottest loop of the engine.
//!
//! Instead, every search owns a [`SearchPool`] with one [`PlyBuffers`] slot
//! per ply. A node *takes* the slot for its own ply (leaving an empty vector
//! behind), writes into the already-allocated capacity, and *puts* it back
//! before returning. Capacity is therefore allocated once per ply and then
//! reused for the whole search — after the first few plies the search issues
//! no allocations at all.
//!
//! The pool is per-search and per-thread (one pool per worker in Lazy SMP),
//! so no locking is involved.

use crate::types::Move;

/// Scratch space for one ply of the search tree.
///
/// Every field is reused across nodes at the same ply: callers `clear()` a
/// buffer before refilling it, which keeps the underlying allocation alive.
#[derive(Default)]
pub(crate) struct PlyBuffers {
    /// Full pseudo-legal move list for the current node (quiet stage).
    pub moves: Vec<Move>,
    /// Tactical (capture / promotion / igui) move list for this node.
    pub caps: Vec<Move>,
    /// Ordering score per entry of `caps` (parallel array — not a `Vec` of
    /// tuples, so filling it never re-allocates a nested container).
    pub cap_scores: Vec<i32>,
    /// Selection order over `cap_scores`: indices into `caps`, best first.
    pub cap_order: Vec<u32>,
    /// `(ordering score, index into `moves`, packed move)` for the quiet
    /// stage, scored once and then sorted / partially selected.
    pub scored: Vec<(i32, u32, u32)>,
}

impl PlyBuffers {
    fn new() -> Self {
        PlyBuffers {
            moves: Vec::with_capacity(900),
            caps: Vec::with_capacity(96),
            cap_scores: Vec::with_capacity(96),
            cap_order: Vec::with_capacity(96),
            scored: Vec::with_capacity(900),
        }
    }
}

/// Per-search pool of [`PlyBuffers`], one slot per ply.
pub(crate) struct SearchPool {
    plies: Vec<PlyBuffers>,
}

impl SearchPool {
    pub fn new() -> Self {
        SearchPool { plies: Vec::new() }
    }

    /// Defensive cap: a runaway ply counter must never grow the pool without
    /// bound. Real searches stay far below this (depth + the QS cap).
    const MAX_PLIES: usize = 512;

    fn ensure(&mut self, ply: usize) {
        let ply = ply.min(Self::MAX_PLIES - 1);
        while self.plies.len() <= ply {
            self.plies.push(PlyBuffers::new());
        }
    }

    /// Take the buffers for `ply`, leaving an empty slot behind.
    ///
    /// The returned value is *owned*, so the caller can keep using `&mut self`
    /// (recursive calls, board mutation) without fighting the borrow checker.
    /// The buffer MUST be handed back with [`SearchPool::put`].
    pub fn take(&mut self, ply: usize) -> PlyBuffers {
        self.ensure(ply);
        let ply = ply.min(Self::MAX_PLIES - 1);
        std::mem::take(&mut self.plies[ply])
    }

    /// Return a buffer taken with [`SearchPool::take`].
    pub fn put(&mut self, ply: usize, pb: PlyBuffers) {
        self.ensure(ply);
        let ply = ply.min(Self::MAX_PLIES - 1);
        self.plies[ply] = pb;
    }

    /// Total allocated capacity of the pool, in elements (diagnostics). This
    /// is constant after warmup, which is the whole point of the pool.
    #[allow(dead_code)] // used by tests / diagnostics
    pub fn capacity_items(&self) -> usize {
        self.plies.iter()
            .map(|p| p.moves.capacity() + p.caps.capacity() + p.scored.capacity()
                     + p.cap_scores.capacity() + p.cap_order.capacity())
            .sum()
    }

    /// Reset every buffer without releasing its allocation (new game).
    #[allow(dead_code)] // natural hook for a future game loop
    pub fn clear(&mut self) {
        for p in &mut self.plies {
            p.moves.clear();
            p.caps.clear();
            p.cap_scores.clear();
            p.cap_order.clear();
            p.scored.clear();
        }
    }
}

impl Default for SearchPool {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Move;

    /// A cheap placeholder move (the tests only exercise buffer capacity).
    fn dummy() -> Move {
        Move::simple(0, 0)
    }

    #[test]
    fn take_leaves_an_empty_slot_and_put_restores_capacity() {
        let mut pool = SearchPool::new();
        let mut pb = pool.take(3);
        pb.moves.extend([dummy(), dummy()]);
        let cap = pb.moves.capacity();
        assert!(cap >= 2);
        pool.put(3, pb);
        // The slot is back, with at least the capacity it had.
        let pb2 = pool.take(3);
        assert!(pb2.moves.capacity() >= cap);
        pool.put(3, pb2);
    }

    #[test]
    fn nested_plies_never_alias() {
        let mut pool = SearchPool::new();
        let parent = pool.take(0);
        let child = pool.take(1);
        assert_eq!(parent.moves.len(), 0);
        assert_eq!(child.moves.len(), 0);
        // Buffers for different plies are independent owned values, which is
        // what makes the take/put protocol safe across recursion.
        pool.put(1, child);
        pool.put(0, parent);
    }

    #[test]
    fn pool_capacity_is_stable_after_warmup() {
        let mut pool = SearchPool::new();
        for ply in 0..8 {
            let mut pb = pool.take(ply);
            pb.moves.extend(std::iter::repeat_with(dummy).take(700));
            pb.scored.extend([(0i32, 0u32, 0u32); 700]);
            pool.put(ply, pb);
        }
        let first = pool.capacity_items();
        for ply in 0..8 {
            let mut pb = pool.take(ply);
            pb.moves.clear();
            pb.scored.clear();
            pb.moves.extend(std::iter::repeat_with(dummy).take(700));
            pb.scored.extend([(0i32, 0u32, 0u32); 700]);
            pool.put(ply, pb);
        }
        assert_eq!(first, pool.capacity_items(), "refilling must not reallocate");
    }

    #[test]
    fn ply_index_is_clamped_to_the_pool_cap() {
        let mut pool = SearchPool::new();
        let pb = pool.take(usize::MAX);
        pool.put(usize::MAX, pb);
        assert!(pool.capacity_items() > 0);
    }
}
