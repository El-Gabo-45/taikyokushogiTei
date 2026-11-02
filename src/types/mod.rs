//! Core types for the Taikyoku Shogi engine.
//!
//! This module is split into submodules, each with a single responsibility:
//! - [`constants`] — board dimensions, colors, direction constants, ranks
//! - [`cell`] — cell encoding (`Cell`, `EMPTY_CELL`, `make_cell`, etc.)
//! - [`square`] — square indexing helpers (`sq_row`, `sq_col`, `step_sq`, etc.)
//! - [`mv`] — the internal `Move` struct and `UndoInfo`
//! - [`movement`] — piece movement rules (`Movement`, `HookType`)
//! - [`tables`] — precomputed lookup tables (ray table, Zobrist, promo zones)

// Public re-exports so `use crate::types::*` continues to work everywhere.
pub use self::cell::*;
pub use self::constants::*;
pub use self::movement::*;
pub use self::square::*;
pub use self::tables::*;

mod cell;
mod constants;
mod movement;
mod mv;
mod square;
mod tables;

// `mv` module contains `Move` and `UndoInfo` — re-export everything.
pub use self::mv::*;

// GameResult lives in lib.rs (public API) — this internal copy is used by
// board.rs and types-internal code. Kept here for cohesion with the type system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameResult {
    BlackWins,
    WhiteWins,
    Draw,
}
