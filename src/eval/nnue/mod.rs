//! NNUE evaluator "S" for Taikyoku Shogi (36x36 board, 804 pieces).
//!
//! # Architecture
//!
//! * **Features** (per perspective): one input per `(plane, square)`, where
//!   the plane is the piece *class* (see [`classes`]) and whether the piece
//!   belongs to the perspective's side: `plane = class` for own pieces,
//!   `C + class` for enemy pieces. There is no king anchor and no hashing:
//!   `index = plane * 1296 + square`. For White the board is rotated by 180
//!   degrees (`square -> 1295 - square`), so both sides see the same game.
//! * **Feature transformer**: `i16` weights, `i32` accumulators of width
//!   [`L1`] per perspective, plus [`OUT_BUCKETS`] PSQT values per feature.
//! * **Network**: `[stm | nstm]` clipped to `0..=127`, pairwise product
//!   (`a*b >> 7`) → 256 → 16 → 32 → 1, `i8` weights, one set of dense layers
//!   per output bucket (selected by total piece count).
//! * **Output**: `(psqt_diff/2 + out) / FV_SCALE`, in the engine's internal
//!   units (the same scale as the hand-crafted evaluator, pawn = 500).
//!
//! The architecture constants (class map, bucket thresholds, scales) live in
//! `rules/nnue_arch.json` and are shared with the Python training tools; the
//! file format is documented in [`format`]. See `docs/NNUE.md`.
//!
//! The network is a value ([`Network`], shared through `Arc`), never a
//! global: attach it to a board with `Board::set_network`.

pub mod arch;
pub mod classes;
pub mod dump;
pub mod format;

use std::sync::Arc;

use crate::board::Board;
use crate::types::*;

pub use arch::ArchSpec;
pub use format::NnueError;

/// Number of squares.
pub const NSQ: usize = 1296;
/// Accumulator width per perspective.
pub const L1: usize = 256;
/// Output buckets (selected by total piece count).
pub const OUT_BUCKETS: usize = 8;
/// First dense layer width.
pub const H1: usize = 16;
/// Second dense layer width.
pub const H2: usize = 32;
/// Accumulator → activation shift: `clip(x) = clamp(x >> FT_SHIFT, 0, 127)`.
pub const FT_SHIFT: u32 = 6;
/// Fixed-point scale of the output layer and the PSQT (`eval = raw / 16`).
pub const FV_SCALE: i32 = 16;
/// Upper bound on the pieces that can be on the board at once.
pub const MAX_BOARD_PIECES: usize = 2 * MAX_PIECES_PER_SIDE;
/// Snapshots kept for undo (a ring: older undos fall back to a refresh).
pub const SNAPSHOT_RING: usize = 256;

#[inline]
fn clip(x: i32) -> i32 {
    (x >> FT_SHIFT).clamp(0, 127)
}

/// Process-unique salt for a new network instance (never 0, which is the
/// salt of the hand-crafted evaluator).
pub(crate) fn next_tt_salt() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let mut z = COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) | 1
}

/// A loaded network. Immutable; share it with `Arc<Network>`.
pub struct Network {
    pub(crate) planes: usize,
    pub(crate) class_map: Vec<u8>,
    pub(crate) thresholds: [u16; arch::NUM_THRESHOLDS],
    pub(crate) arch_hash: [u8; 32],
    pub(crate) eval_limit: i32,
    /// Distinguishes this network instance in the transposition table.
    pub(crate) tt_salt: u64,
    /// `[feature][L1]`, flat and contiguous.
    pub(crate) ft: Box<[i16]>,
    pub(crate) ft_bias: Box<[i32]>,
    /// `[feature][OUT_BUCKETS]`.
    pub(crate) psqt: Box<[i32]>,
    /// `[bucket][input][output]`.
    pub(crate) fc0_w: Box<[i8]>,
    pub(crate) fc0_b: Box<[i32]>,
    pub(crate) fc1_w: Box<[i8]>,
    pub(crate) fc1_b: Box<[i32]>,
    pub(crate) fc2_w: Box<[i8]>,
    pub(crate) fc2_b: Box<[i32]>,
}

impl std::fmt::Debug for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Network")
            .field("classes", &(self.planes / 2))
            .field("dims", &self.dims())
            .field("thresholds", &self.thresholds)
            .finish()
    }
}

/// Splitmix64: tiny deterministic generator (same sequence on every platform).
struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[-r, r]`.
    fn range(&mut self, r: i64) -> i64 {
        (self.next() % (2 * r as u64 + 1)) as i64 - r
    }
}

impl Network {
    /// Number of feature planes (`2 * classes`).
    pub fn planes(&self) -> usize {
        self.planes
    }
    /// Input features per perspective.
    pub fn dims(&self) -> usize {
        self.planes * NSQ
    }
    pub fn num_classes(&self) -> usize {
        self.planes / 2
    }
    pub fn thresholds(&self) -> &[u16] {
        &self.thresholds
    }
    pub fn arch_hash(&self) -> &[u8; 32] {
        &self.arch_hash
    }
    pub fn class_of(&self, piece_type: u16) -> usize {
        self.class_map[piece_type as usize] as usize
    }

    /// A deterministic pseudo-random network for tests, benchmarks and the
    /// parity tool. It is *not* a playing network.
    pub fn random(seed: u64) -> Network {
        let arch = ArchSpec::embedded();
        let mut rng = SplitMix(seed);
        let dims = arch.dims();
        let ft: Box<[i16]> = (0..dims * L1).map(|_| rng.range(40) as i16).collect();
        let ft_bias: Box<[i32]> = (0..L1).map(|_| rng.range(1500) as i32).collect();
        let psqt: Box<[i32]> = (0..dims * OUT_BUCKETS)
            .map(|_| rng.range(300) as i32)
            .collect();
        let fc0_w: Box<[i8]> = (0..OUT_BUCKETS * L1 * H1)
            .map(|_| rng.range(12) as i8)
            .collect();
        let fc0_b: Box<[i32]> = (0..OUT_BUCKETS * H1)
            .map(|_| rng.range(2000) as i32)
            .collect();
        let fc1_w: Box<[i8]> = (0..OUT_BUCKETS * H1 * H2)
            .map(|_| rng.range(20) as i8)
            .collect();
        let fc1_b: Box<[i32]> = (0..OUT_BUCKETS * H2)
            .map(|_| rng.range(2000) as i32)
            .collect();
        let fc2_w: Box<[i8]> = (0..OUT_BUCKETS * H2).map(|_| rng.range(30) as i8).collect();
        let fc2_b: Box<[i32]> = (0..OUT_BUCKETS).map(|_| rng.range(4000) as i32).collect();
        let mut thresholds = [0u16; arch::NUM_THRESHOLDS];
        thresholds.copy_from_slice(&arch.thresholds_pieces);
        Network {
            planes: arch.planes(),
            class_map: arch.class_map.clone(),
            thresholds,
            arch_hash: arch.arch_hash(),
            eval_limit: arch.eval_limit,
            tt_salt: next_tt_salt(),
            ft,
            ft_bias,
            psqt,
            fc0_w,
            fc0_b,
            fc1_w,
            fc1_b,
            fc2_w,
            fc2_b,
        }
    }

    /// Feature index of a piece for one perspective (0 = Black, 1 = White).
    #[inline]
    pub fn feature_index(&self, perspective: u8, piece_type: u16, color: u8, sq: usize) -> usize {
        let cls = self.class_map[piece_type as usize] as usize;
        let plane = if color == perspective {
            cls
        } else {
            self.planes / 2 + cls
        };
        let vsq = if perspective == BLACK {
            sq
        } else {
            NSQ - 1 - sq
        };
        plane * NSQ + vsq
    }

    /// Output bucket for a total piece count.
    #[inline]
    pub fn bucket(&self, piece_count: usize) -> usize {
        for (k, &t) in self.thresholds.iter().enumerate() {
            if piece_count < t as usize {
                return k;
            }
        }
        OUT_BUCKETS - 1
    }

    /// Integer forward pass. Returns `(psqt_term, dense_out, total)` before
    /// the final clamp; `total` is the evaluation from the side to move.
    pub fn forward(&self, acc: &Accumulator, stm: u8, bucket: usize) -> (i32, i32, i32) {
        let us = &acc.acc[stm as usize];
        let them = &acc.acc[(stm ^ 1) as usize];

        let mut pw = [0i32; L1];
        for i in 0..L1 {
            pw[i] = (clip(us[i]) * clip(them[i])) >> 7;
        }

        let w0 = &self.fc0_w[bucket * L1 * H1..][..L1 * H1];
        let mut s0 = [0i32; H1];
        s0.copy_from_slice(&self.fc0_b[bucket * H1..][..H1]);
        for i in 0..L1 {
            let p = pw[i];
            if p != 0 {
                let row = &w0[i * H1..][..H1];
                for j in 0..H1 {
                    s0[j] += p * row[j] as i32;
                }
            }
        }
        let mut h0 = [0i32; H1];
        for j in 0..H1 {
            h0[j] = (s0[j] >> 6).clamp(0, 127);
        }

        let w1 = &self.fc1_w[bucket * H1 * H2..][..H1 * H2];
        let mut s1 = [0i32; H2];
        s1.copy_from_slice(&self.fc1_b[bucket * H2..][..H2]);
        for i in 0..H1 {
            let p = h0[i];
            let row = &w1[i * H2..][..H2];
            for j in 0..H2 {
                s1[j] += p * row[j] as i32;
            }
        }

        let w2 = &self.fc2_w[bucket * H2..][..H2];
        let mut out = self.fc2_b[bucket];
        for j in 0..H2 {
            out += (s1[j] >> 6).clamp(0, 127) * w2[j] as i32;
        }

        // `/` truncates toward zero (as the Python reference does explicitly).
        let ps = (acc.psqt[stm as usize][bucket] - acc.psqt[(stm ^ 1) as usize][bucket]) / 2;
        (ps, out, (ps + out) / FV_SCALE)
    }

    /// Evaluation from the side to move, clamped to `±eval_limit`.
    #[inline]
    pub fn evaluate(&self, acc: &Accumulator, stm: u8, piece_count: usize) -> i32 {
        let (_, _, total) = self.forward(acc, stm, self.bucket(piece_count));
        total.clamp(-self.eval_limit, self.eval_limit)
    }
}

/// Feature-transformer state of a position: one accumulator and one set of
/// PSQT sums per perspective (0 = Black, 1 = White).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Accumulator {
    pub(crate) acc: [[i32; L1]; 2],
    pub(crate) psqt: [[i32; OUT_BUCKETS]; 2],
}

impl Accumulator {
    pub fn zero() -> Self {
        Accumulator {
            acc: [[0; L1]; 2],
            psqt: [[0; OUT_BUCKETS]; 2],
        }
    }

    /// Rebuild from the pieces on the board.
    pub fn refresh(&mut self, net: &Network, board: &Board) {
        for p in 0..2 {
            self.acc[p].copy_from_slice(&net.ft_bias);
            self.psqt[p] = [0; OUT_BUCKETS];
        }
        for sq in 0..NUM_SQUARES {
            let cell = board.cells[sq];
            if cell != EMPTY_CELL {
                self.add(net, cell_piece(cell), cell_color(cell), sq);
            }
        }
    }

    #[inline]
    pub fn add(&mut self, net: &Network, piece_type: u16, color: u8, sq: usize) {
        for p in 0..2u8 {
            let idx = net.feature_index(p, piece_type, color, sq);
            let row = &net.ft[idx * L1..][..L1];
            for (a, &w) in self.acc[p as usize].iter_mut().zip(row) {
                *a += w as i32;
            }
            let prow = &net.psqt[idx * OUT_BUCKETS..][..OUT_BUCKETS];
            for (a, &w) in self.psqt[p as usize].iter_mut().zip(prow) {
                *a += w;
            }
        }
    }

    #[inline]
    pub fn sub(&mut self, net: &Network, piece_type: u16, color: u8, sq: usize) {
        for p in 0..2u8 {
            let idx = net.feature_index(p, piece_type, color, sq);
            let row = &net.ft[idx * L1..][..L1];
            for (a, &w) in self.acc[p as usize].iter_mut().zip(row) {
                *a -= w as i32;
            }
            let prow = &net.psqt[idx * OUT_BUCKETS..][..OUT_BUCKETS];
            for (a, &w) in self.psqt[p as usize].iter_mut().zip(prow) {
                *a -= w;
            }
        }
    }
}

/// Network plus incrementally maintained state attached to a [`Board`].
///
/// Undo uses a fixed ring of accumulator snapshots: memory does not depend
/// on the length of the game, apply/undo never allocate, and an undo that
/// goes further back than the ring simply refreshes from the board.
#[derive(Clone)]
pub struct NnueState {
    pub(crate) net: Arc<Network>,
    pub(crate) acc: Accumulator,
    ring: Vec<Accumulator>,
    top: usize,
    len: usize,
}

impl NnueState {
    pub(crate) fn new(net: Arc<Network>, board: &Board) -> Self {
        let mut s = NnueState {
            net,
            acc: Accumulator::zero(),
            ring: vec![Accumulator::zero(); SNAPSHOT_RING],
            top: 0,
            len: 0,
        };
        s.reset(board);
        s
    }

    /// Recompute from the board and forget all snapshots.
    pub(crate) fn reset(&mut self, board: &Board) {
        self.acc.refresh(&self.net, board);
        self.len = 0;
        self.top = 0;
    }

    #[inline]
    pub(crate) fn push_snapshot(&mut self) {
        self.top = (self.top + 1) % SNAPSHOT_RING;
        self.ring[self.top] = self.acc;
        self.len = (self.len + 1).min(SNAPSHOT_RING);
    }

    /// Restore the newest snapshot; `false` if the ring is exhausted.
    #[inline]
    pub(crate) fn pop_snapshot(&mut self) -> bool {
        if self.len == 0 {
            return false;
        }
        self.acc = self.ring[self.top];
        self.top = (self.top + SNAPSHOT_RING - 1) % SNAPSHOT_RING;
        self.len -= 1;
        true
    }

    /// Same network and accumulator, empty snapshot history.
    pub(crate) fn without_history(&self) -> Self {
        NnueState {
            net: self.net.clone(),
            acc: self.acc,
            ring: vec![Accumulator::zero(); SNAPSHOT_RING],
            top: 0,
            len: 0,
        }
    }

    #[inline]
    pub(crate) fn evaluate(&self, board: &Board) -> i32 {
        let pc = board.piece_count[0] + board.piece_count[1];
        self.net.evaluate(&self.acc, board.side_to_move, pc)
    }
}

// ============================================================
// Debugging / research: single-position forward-pass report
// ============================================================

/// Summary statistics of one perspective's accumulator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccStats {
    /// Minimum / maximum accumulator value (raw `i32`, before clipping).
    pub min: i32,
    pub max: i32,
    /// Arithmetic mean of the raw values.
    pub mean: f64,
    /// How many units clip to the lower bound (0) / upper bound (127).
    pub clipped_lo: usize,
    pub clipped_hi: usize,
}

fn acc_stats(acc: &[i32; L1]) -> AccStats {
    let mut min = i32::MAX;
    let mut max = i32::MIN;
    let mut sum = 0i64;
    let mut lo = 0usize;
    let mut hi = 0usize;
    for &x in acc {
        min = min.min(x);
        max = max.max(x);
        sum += x as i64;
        let c = clip(x);
        if c == 0 {
            lo += 1;
        } else if c == 127 {
            hi += 1;
        }
    }
    AccStats {
        min,
        max,
        mean: sum as f64 / L1 as f64,
        clipped_lo: lo,
        clipped_hi: hi,
    }
}

/// Stockfish-style "explain" report: every intermediate quantity of the
/// NNUE forward pass for ONE position, for debugging and research.
///
/// Built by [`explain`]; `eval` is the final (clamped) value the engine
/// returns, `refresh_eval` recomputes everything from scratch so the two
/// can be compared (the incremental accumulator must agree with a full
/// refresh — a mismatch is an apply/undo bookkeeping bug).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NnueExplain {
    /// Side to move (0 = Black, 1 = White).
    pub side_to_move: u8,
    /// Piece counts per color (Black, White).
    pub pieces: [usize; 2],
    /// Output bucket selected by the total piece count.
    pub bucket: usize,
    /// Active features per perspective (one per piece).
    pub features: usize,
    /// Accumulator statistics per perspective (Black, White).
    pub acc: [AccStats; 2],
    /// PSQT sum per perspective for the selected bucket.
    pub psqt: [i32; 2],
    /// `(psqt_stm - psqt_nstm) / 2`, the PSQT term of the output layer.
    pub psqt_term: i32,
    /// Output of the dense layers (before the final division).
    pub dense_out: i32,
    /// `(psqt_term + dense_out) / FV_SCALE`, before the final clamp.
    pub raw_total: i32,
    /// Final evaluation as `Board::evaluate()` returns it (clamped).
    pub eval: i32,
    /// Same evaluation recomputed from a full accumulator refresh.
    pub refresh_eval: i32,
}

/// Build an [`NnueExplain`] for `board` using the network attached to it.
///
/// Returns `None` when no network is attached to the board. Note that the
/// `eval`/`refresh_eval` fields are raw forward-pass outputs even for
/// terminal positions (where `Board::evaluate()` returns mate/draw scores).
pub fn explain(board: &crate::Board) -> Option<NnueExplain> {
    let b = &board.inner;
    let state = b.nnue.as_deref()?;
    let net = &state.net;
    let stm = b.side_to_move;
    let pieces = [b.piece_count[0] as usize, b.piece_count[1] as usize];
    let bucket = net.bucket(pieces[0] + pieces[1]);

    let acc = [acc_stats(&state.acc.acc[0]), acc_stats(&state.acc.acc[1])];
    let psqt = [state.acc.psqt[0][bucket], state.acc.psqt[1][bucket]];
    let (psqt_term, dense_out, raw_total) = net.forward(&state.acc, stm, bucket);
    let eval = raw_total.clamp(-net.eval_limit, net.eval_limit);

    // Parity check: rebuild the accumulator from scratch and re-run the
    // forward pass. Must equal the incremental result.
    let mut fresh = Accumulator::zero();
    fresh.refresh(net, b);
    let (_, _, refresh_raw) = net.forward(&fresh, stm, bucket);
    let refresh_eval = refresh_raw.clamp(-net.eval_limit, net.eval_limit);

    Some(NnueExplain {
        side_to_move: stm,
        pieces,
        bucket,
        features: pieces[0] + pieces[1],
        acc,
        psqt,
        psqt_term,
        dense_out,
        raw_total,
        eval,
        refresh_eval,
    })
}

#[cfg(test)]
mod tests;
