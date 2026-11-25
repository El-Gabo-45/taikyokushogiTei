#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

//! Position dumps for cross-language verification (`nnue dump-positions`).
//!
//! The Python reference (`training/parity_gate.py`) recomputes features,
//! bucket and evaluation from the piece list alone and must agree with what
//! the engine computed incrementally.

use super::Network;
use crate::types::{EMPTY_CELL, cell_color, cell_piece};

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
#[must_use]
pub fn dump_position(board: &crate::Board, net: &Network) -> PositionDump {
    let b = &board.inner;
    let mut pieces = Vec::new();
    let mut features = [Vec::new(), Vec::new()];
    for (sq, &cell) in b.cells.iter().enumerate() {
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
        bucket: net.bucket(b.piece_list_len[0] + b.piece_list_len[1]),
        pieces,
        features,
        eval: board.evaluate(),
    }
}

/// A copy of `board` with a random subset of its non-royal pieces removed
/// (`keep_per_mille` of them are kept), used to exercise every output bucket
/// in the parity gate. The network of `board` is not carried over: attach one
/// with `set_network` afterwards.
#[must_use]
pub fn thinned(board: &crate::Board, keep_per_mille: u32, seed: u64) -> crate::Board {
    let mut inner = board.inner.clone();
    inner.set_network(None);
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    for slot in &mut inner.cells {
        let cell = *slot;
        if cell == EMPTY_CELL || crate::pieces::is_royal(cell_piece(cell)) {
            continue;
        }
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        if ((state >> 33) % 1000) as u32 >= keep_per_mille {
            *slot = EMPTY_CELL;
        }
    }
    inner.rebuild_lists_pub();
    crate::Board { inner }
}
