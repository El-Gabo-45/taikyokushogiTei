//! Cell encoding: 0 = empty, otherwise `(piece_type << 1) | color`.

use super::constants::*;

/// Cell encoding: 0 = empty, otherwise `(piece_type << 1) | color`.
pub type Cell = u16;

/// Represents an empty square.
pub const EMPTY_CELL: Cell = 0;

/// Sentinel value for an invalid square index.
pub const INVALID_SQ: u16 = 0xFFFF;

/// Create a cell from a piece type and color.
#[inline]
pub fn make_cell(piece_type: u16, color: u8) -> Cell {
    (piece_type << 1) | (color as u16)
}

/// Extract the piece type ID from a cell.
#[inline]
pub fn cell_piece(cell: Cell) -> u16 {
    cell >> 1
}

/// Extract the color from a cell.
#[inline]
pub fn cell_color(cell: Cell) -> u8 {
    (cell & 1) as u8
}

/// Convert a (row, col) pair to a flat square index.
#[inline]
pub fn sq_index(row: usize, col: usize) -> usize {
    row * BOARD_SIZE + col
}
