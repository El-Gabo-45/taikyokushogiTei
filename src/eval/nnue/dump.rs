//! Position dumps for cross-language verification (`nnue dump-positions`).
//!
//! The Python reference (`training/parity_gate.py`) recomputes features,
//! bucket and evaluation from the piece list alone and must agree with what
//! the engine computed incrementally.

use super::Network;
use crate::types::*;

/// Everything the Python reference needs to recompute one evaluation.
pub struct PositionDump {
    /// Side to move (0 = Black, 1 = White).
    pub stm: u8,
    /// `(square, piece_type, color)` for every piece on the board.
    pub pieces: Vec<(u16, u16, u8)>,
    /// Output bucket the engine selected.
    pub bucket: usize,
    /// Feature indices per perspective (0 = Black, 1 = White), ascending.
    pub features: [Vec<u32>; 2],
    /// `Board::evaluate()` as the engine computed it (incrementally).
    pub eval: i32,
}

/// Describe `board`. The board must have `net` attached (so `eval` is the
/// engine's incremental result); positions that are over are not dumped
/// by the caller because `evaluate()` returns mate/draw scores for them.
pub fn dump_position(board: &crate::Board, net: &Network) -> PositionDump {
    let b = &board.inner;
    let mut pieces = Vec::new();
    let mut features = [Vec::new(), Vec::new()];
    for sq in 0..NUM_SQUARES {
        let cell = b.cells[sq];
        if cell == EMPTY_CELL {
            continue;
        }
        let (pt, color) = (cell_piece(cell), cell_color(cell));
        pieces.push((sq as u16, pt, color));
        for p in 0..2u8 {
            features[p as usize].push(net.feature_index(p, pt, color, sq) as u32);
        }
    }
    features[0].sort_unstable();
    features[1].sort_unstable();
    PositionDump {
        stm: b.side_to_move,
        bucket: net.bucket(b.piece_count[0] + b.piece_count[1]),
        pieces,
        features,
        eval: board.evaluate(),
    }
}

/// A copy of `board` with a random subset of its non-royal pieces removed
/// (`keep_per_mille` of them are kept), used to exercise every output bucket
/// in the parity gate. The network of `board` is not carried over: attach one
/// with `set_network` afterwards.
pub fn thinned(board: &crate::Board, keep_per_mille: u32, seed: u64) -> crate::Board {
    let mut inner = board.inner.clone();
    inner.set_network(None);
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for sq in 0..NUM_SQUARES {
        let cell = inner.cells[sq];
        if cell == EMPTY_CELL || crate::pieces::is_royal(cell_piece(cell)) {
            continue;
        }
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        if ((state >> 33) % 1000) as u32 >= keep_per_mille {
            inner.cells[sq] = EMPTY_CELL;
        }
    }
    inner.rebuild_lists_pub();
    crate::Board { inner }
}
