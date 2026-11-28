#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
//! Positional bonuses inspired by Tomatene's `evalPiece`.
//!
//! Two ideas are borrowed (reimplemented, not copied):
//! - non-royal pieces prefer the center and advancing ranks;
//! - pieces near a royal family get a defensive/attacking bonus.
//!
//! The values are folded into the existing PSQT table, so they cost nothing
//! at search time: `psqt_score` is already updated incrementally by
//! `apply_move`/`undo_move`. Move generation and the search core are untouched.

use crate::pieces;
use crate::types::{BLACK, NUM_SQUARES};
use std::sync::OnceLock;

const BOARD: i32 = 36;

pub struct PositionalTables {
    /// `[color][square]` combined bonus from that color's perspective.
    bonus: [[i32; NUM_SQUARES]; 2],
}

/// Royal start squares derived straight from the setup ranks.
///
/// This deliberately avoids `Board::initial()`: building a board computes the
/// PSQT, which calls back into this module's `bonus()` and would deadlock the
/// `OnceLock` that owns these tables.
fn royal_start_squares() -> [[usize; 2]; 2] {
    let mut out = [[0usize; 2]; 2];
    for (rank_idx, rank_str) in crate::pieces::SETUP_RANKS.iter().enumerate() {
        let mut n = 0;
        for (col, pt) in crate::pieces::parse_setup_rank(rank_str).iter().enumerate() {
            let Some(pt) = pt else { continue };
            if !pieces::is_royal(*pt) || n >= 2 {
                continue;
            }
            let black_row = 35 - rank_idx;
            out[0][n] = black_row * 36 + col;
            out[1][n] = (35 - black_row) * 36 + (35 - col);
            n += 1;
        }
    }
    out
}

fn build_tables() -> PositionalTables {
    let royals = royal_start_squares();
    let mut bonus = [[0i32; NUM_SQUARES]; 2];
    for color in 0..2 {
        let own = royals[color][0];
        let enemy = royals[1 - color][0];
        for (sq, value) in bonus[color].iter_mut().enumerate() {
            let row = (sq / 36) as i32;
            let col = (sq % 36) as i32;
            // Perspective: row 0 is that color's back rank.
            let rel_row = if color == BLACK as usize {
                row
            } else {
                BOARD - 1 - row
            };
            let rel_col = if color == BLACK as usize {
                col
            } else {
                BOARD - 1 - col
            };

            let horizontal = 18 - (rel_col - 17).abs();
            let vertical = rel_row.min(25);
            let centrality = horizontal + vertical + horizontal * vertical / 2;

            // Both royal squares are already in raw board coordinates, so no
            // mirroring here: the color symmetry comes from `royals[1]` being
            // the 180-degree rotation of `royals[0]`.
            let dist_own = (row - (own / 36) as i32).abs() + (col - (own % 36) as i32).abs();
            let dist_enemy = (row - (enemy / 36) as i32).abs() + (col - (enemy % 36) as i32).abs();
            let near_own = if dist_own <= 4 { 40 } else { 0 };
            let near_enemy = if dist_enemy < 8 {
                20 + (8 - dist_enemy) * 4
            } else {
                0
            };

            // Keep the contribution small relative to material so existing
            // play and seeded tests are not swamped by the new terms.
            *value = (centrality + near_own + near_enemy) / 10;
        }
    }
    PositionalTables { bonus }
}

fn tables() -> &'static PositionalTables {
    static TABLES: OnceLock<PositionalTables> = OnceLock::new();
    TABLES.get_or_init(build_tables)
}

/// Whether the new bonuses are folded into the PSQT.
///
/// Off by default so existing search results (and seeded strength tests) are
/// bit-identical. Enable with `TAIKYOKU_POSITIONAL=1` to opt in.
pub fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("TAIKYOKU_POSITIONAL").ok().as_deref() == Some("1"))
}

/// Bonus for a piece of type `pt` on `sq` from `color`'s perspective.
///
/// Royals return `0` here: their placement is handled by the existing PSQT
/// center penalty and the dedicated king-safety term.
pub fn bonus(pt: u16, sq: usize, color: u8) -> i32 {
    if sq >= NUM_SQUARES || pieces::is_royal(pt) {
        return 0;
    }
    tables().bonus[color as usize][sq]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn royals_get_no_centrality_bonus() {
        let royal = pieces::find_by_abbrev("K").unwrap_or(0);
        assert_eq!(bonus(royal, 17 * 36 + 17, BLACK), 0);
    }

    #[test]
    fn center_is_better_than_corner() {
        let pawn = pieces::find_by_abbrev("P").unwrap_or(1);
        let center = bonus(pawn, 17 * 36 + 17, BLACK);
        let corner = bonus(pawn, 0, BLACK);
        assert!(
            center > corner,
            "center {center} should beat corner {corner}"
        );
    }

    #[test]
    fn tables_are_mirrored_between_colors() {
        let pawn = pieces::find_by_abbrev("P").unwrap_or(1);
        for sq in [0, 35, 17 * 36 + 17, 9 * 36 + 4, 1295] {
            let row = sq / 36;
            let col = sq % 36;
            let mirrored = (35 - row) * 36 + (35 - col);
            assert_eq!(
                bonus(pawn, sq, BLACK),
                bonus(pawn, mirrored, 1),
                "square {sq} should mirror to {mirrored}"
            );
        }
    }
}
