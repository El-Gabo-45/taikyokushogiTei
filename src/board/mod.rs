//! Board representation for Taikyoku Shogi.
//!
//! The `Board` struct and its core methods live in [`core`].  Additional
//! submodules (`apply`, `undo`, `pieces`, `royal`, `result`, `display`)
//! can be split out of `core.rs` in a follow-up as the codebase grows.
//!
//! For now, all board logic is in `core.rs`; this module simply makes
//! `Board` available as `crate::board::Board` for internal use.

// Re-export the public Board type so `crate::board::Board` works.
pub use self::core::Board;

mod core;
