//! Legal move generation for Taikyoku Shogi.
//!
//! There is **no check filtering**: the variant has no check rule, so
//! `generate_legal_moves` never rejects a move for leaving a royal piece en
//! prise.
//!
//! Module layout:
//! - [`core`] — the main move generation entry points, dedup, and all
//!   generation logic (sliding, jumping, range capture, igui, promotion).

pub use self::core::*;

mod core;
