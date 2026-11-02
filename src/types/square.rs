//! Square indexing helpers: row/column lookup tables, direction deltas, step.

use super::constants::*;

// Per-square row lookup (compile-time precomputed), replacing `sq / BOARD_SIZE`.
// Division by a non-power-of-two in a hot per-piece movegen loop is measurably
// slower than a cached 1-byte table load on giant (36×36) boards.
static SQ_ROW: [u8; NUM_SQUARES] = const {
    let mut a = [0u8; NUM_SQUARES];
    let mut i = 0usize;
    while i < NUM_SQUARES {
        a[i] = (i / BOARD_SIZE) as u8;
        i += 1;
    }
    a
};

/// Per-square column lookup (compile-time precomputed), replacing `sq % BOARD_SIZE`.
static SQ_COL: [u8; NUM_SQUARES] = const {
    let mut a = [0u8; NUM_SQUARES];
    let mut i = 0usize;
    while i < NUM_SQUARES {
        a[i] = (i % BOARD_SIZE) as u8;
        i += 1;
    }
    a
};

/// Get the row of a square.
#[inline]
pub fn sq_row(sq: usize) -> usize {
    SQ_ROW[sq] as usize
}

/// Get the column of a square.
#[inline]
pub fn sq_col(sq: usize) -> usize {
    SQ_COL[sq] as usize
}

/// Get direction deltas adjusted for color.
#[inline]
pub fn get_deltas(dir: usize, color: u8) -> (i32, i32) {
    if color == BLACK {
        (DIR_DR[dir], DIR_DC[dir])
    } else {
        (-DIR_DR[dir], -DIR_DC[dir])
    }
}

/// Step one square in a direction from `sq`. Returns `None` if out of bounds.
#[inline]
pub fn step_sq(sq: usize, dir: usize, color: u8) -> Option<usize> {
    let (dr, dc) = get_deltas(dir, color);
    let r = sq_row(sq) as i32 + dr;
    let c = sq_col(sq) as i32 + dc;
    if r >= 0 && r < BOARD_SIZE as i32 && c >= 0 && c < BOARD_SIZE as i32 {
        Some((r as usize) * BOARD_SIZE + (c as usize))
    } else {
        None
    }
}
