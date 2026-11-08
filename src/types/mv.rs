//! The internal `Move` struct and `UndoInfo`.
//!
//! All fields are `Copy` primitives, so `Move` is `Copy` as well. This
//! eliminates needless `.clone()` calls in the hot movegen path
//! (`dedup_piece_range`, move list compaction, etc.).

use super::cell::*;

/// A move in the game (internal representation).
///
/// The public API wraps this in [`crate::Move`] which adds convenience
/// accessors and hides internal fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Move {
    pub from_sq: u16,
    pub to_sq: u16,
    pub promotion: bool,
    pub captured_piece: u16, // 0 if no capture
    pub captured_color: u8,
    pub is_igui: bool,
    // For lion mid-capture
    pub mid_sq: u16, // INVALID_SQ if none
    pub mid_piece: u16,
    pub mid_color: u8,
    /// True for moves emitted by the range-capture generator. `apply_move`
    /// recomputes ALL occupied squares strictly between from and to (any
    /// color — rank legality was verified at generation) and captures them,
    /// plain POD: no `Rc<Vec<..>>`, zero heap allocations in the search.
    pub range_cap: bool,
    /// Sum of the values of pieces captured on INTERMEDIATE squares of a
    /// range-capture move (the landing capture is in `captured_piece`).
    /// Used for move ordering and the material-delta fast path; always 0
    /// for non-range-capture moves.
    pub caps_value: i32,
}

impl Move {
    /// Create a simple non-capture, non-promotion move.
    pub fn simple(from: u16, to: u16) -> Self {
        Move {
            from_sq: from,
            to_sq: to,
            promotion: false,
            captured_piece: 0,
            captured_color: 0,
            is_igui: false,
            mid_sq: INVALID_SQ,
            mid_piece: 0,
            mid_color: 0,
            range_cap: false,
            caps_value: 0,
        }
    }
}

/// Undo information saved per move.
#[derive(Clone)]
pub struct UndoInfo {
    pub from_sq: u16,
    pub to_sq: u16,
    pub from_cell: Cell,
    pub to_cell: Cell,
    pub side: u8,
    pub move_number: u32,
    pub mid_sq: u16,
    pub mid_cell: Cell,
    /// Index into `Board::cap_stack` marking where this move's captured
    /// pieces are popped back to. No heap allocation.
    pub cap_base: usize,
    pub no_progress_plies: u32,
    pub hash: u64,
    /// Incremental material score snapshot for fast undo.
    /// Reference: Chess Programming Wiki, "Incremental Updates".
    pub material_score: i32,
    /// Incremental PSQT score snapshot for fast undo.
    pub psqt_score: i32,
}
