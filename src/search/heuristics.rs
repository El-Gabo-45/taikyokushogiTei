//! Search heuristics: killer moves, butterfly history and counter moves.
//!
//! The tables live in a [`Heuristics`] value **owned by the searcher**, not in
//! process-wide statics. Two searches running at the same time (a match
//! runner, a UI searching on a worker thread, the test suite) can therefore
//! never disturb each other's move ordering, and a caller that keeps its
//! searcher alive between moves keeps the statistics it learned. Lazy SMP
//! starts one of these per worker; only the transposition table is shared.

use super::params;
use crate::types::NUM_SQUARES;

/// One history plane per side.
const HIST_PLANES: usize = 2;
/// Counter-move slots, indexed by the low bits of the previous move.
const COUNTER_SLOTS: usize = 1 << 16;

pub(crate) struct Heuristics {
    /// Two killers per ply, packed as `first | second << 32`.
    killers: Box<[u64]>,
    /// Butterfly history, indexed by the exact (side, from, to) triple:
    /// `2 × 1296 × 1296` i32 ≈ 13 MB, zero-initialized (the OS faults the
    /// pages in lazily, so an unused searcher costs almost nothing).
    history: Box<[i32]>,
    /// Counter moves: previous move → reply that refuted it.
    counter: Box<[u32]>,
}

impl Heuristics {
    pub fn new() -> Self {
        Heuristics {
            killers: vec![0u64; params::KILLER_MAX_PLY as usize + 1].into_boxed_slice(),
            history: vec![0i32; HIST_PLANES * NUM_SQUARES * NUM_SQUARES].into_boxed_slice(),
            counter: vec![0u32; COUNTER_SLOTS].into_boxed_slice(),
        }
    }

    /// Zero every table (new game). The transposition table belongs to the
    /// searcher and is cleared separately.
    pub fn clear(&mut self) {
        self.killers.fill(0);
        self.history.fill(0);
        self.counter.fill(0);
    }

    /// Record `mv` as the new first killer at `depth` (old first becomes second).
    #[inline]
    pub fn killer_store(&mut self, depth: u32, mv: u32) {
        let d = depth.min(params::KILLER_MAX_PLY) as usize;
        let mv0 = self.killers[d] as u32;
        if mv != mv0 {
            self.killers[d] = mv as u64 | ((mv0 as u64) << 32);
        }
    }

    #[inline]
    pub fn killer_score(&self, depth: u32, mv: u32) -> i32 {
        let d = depth.min(params::KILLER_MAX_PLY) as usize;
        let p = self.killers[d];
        let mv0 = p as u32;
        let mv1 = (p >> 32) as u32;
        if mv == mv0 {
            params::KILLER1_SCORE
        } else if mv == mv1 {
            params::KILLER2_SCORE
        } else {
            0
        }
    }

    #[inline]
    fn history_idx(from: usize, to: usize, side: u8) -> usize {
        side as usize * NUM_SQUARES * NUM_SQUARES + from * NUM_SQUARES + to
    }

    pub fn history_store(&mut self, from: usize, to: usize, depth: u32, side: u8) {
        let idx = Self::history_idx(from, to, side);
        let bonus = (depth * depth).min(params::HIST_BONUS_CAP as u32) as i32;
        self.history[idx] = self.history[idx].saturating_add(bonus).min(params::HIST_MAX);
    }

    /// History gravity: penalize quiets that failed to cause a cutoff, so next
    /// time the cutting move (and similar ones) are tried first.
    pub fn history_malus(&mut self, from: usize, to: usize, depth: u32, side: u8) {
        let idx = Self::history_idx(from, to, side);
        let malus = (depth * depth).min(params::HIST_BONUS_CAP as u32) as i32;
        self.history[idx] = self.history[idx].saturating_sub(malus).max(0);
    }

    #[inline]
    pub fn history_score(&self, from: usize, to: usize, side: u8) -> i32 {
        self.history[Self::history_idx(from, to, side)]
    }

    pub fn counter_store(&mut self, prev: u32, mv: u32) {
        self.counter[(prev as usize) & 0xFFFF] = mv;
    }

    #[inline]
    pub fn counter_score(&self, prev: u32, mv: u32) -> i32 {
        if prev == 0 {
            return 0;
        }
        if self.counter[(prev as usize) & 0xFFFF] == mv {
            params::COUNTER_SCORE
        } else {
            0
        }
    }
}

impl Default for Heuristics {
    fn default() -> Self {
        Self::new()
    }
}
