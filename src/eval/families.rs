//! Piece family classification.
//!
//! Each piece is assigned to one or more families based on its movement
//! pattern and game role.  The evaluation function then works in family
//! space rather than over individual piece IDs.
//!
//! Adding a new piece only requires updating its movement definition; the
//! classification is derived automatically from the movement primitives.
//!
//! # Performance: Precomputed Lookup Tables
//!
//! `classify()` and `family_value()` involve branching logic and string
//! checks (`abbrev.starts_with('+')`) that run for every piece on every
//! `evaluate()` call. With 804 pieces, that's 804× per eval. We precompute
//! the results for all 512 piece types once at startup into static arrays,
//! then use a simple array index — turning O(branches) into O(1) per piece.
//!
//! Reference: "Precomputed tables" is a standard technique in chess engines
//! (Chess Programming Wiki: "Evaluation Functions"). Stockfish uses similar
//! precomputed PSQT (Piece-Square Tables) arrays initialized at startup.

#![allow(dead_code)] // utility/debug API kept intentionally
use crate::pieces;
use crate::types::*;
use std::sync::OnceLock;

/// Symbolic family of a piece type.  Pieces can belong to multiple families
/// (e.g. a Lion is both a `Lion` and a `Promoted` family).  The classifier
/// returns the *primary* family — the most strategically relevant one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// K, CP — capture = win.
    Royal,
    /// GG, VG, BG, RO, VD, +FCR — fly over lower-rank pieces.
    RangeCap,
    /// LN, LI, +FFI, +BSP — multi-step area movers with igui.
    Lion,
    /// FE, EL, HF, +GEA, +GHK — Queen + long-range jump combinations.
    Eagle,
    /// DK, DH, RD, GD, RD, BL — half-rook + half-bishop style pieces.
    Dragon,
    /// FR, FT, FO, DM, FI, WA, WD, F — long-range attacker.
    Demon,
    /// Standard pieces: CN, SD, RN, GE, GD, WO.
    Standard,
    /// Long-range sliders: R, B, DH, DK, FH, etc.
    Slider,
    /// Stepping pieces: G, S, C, M, K, CP, etc.
    Stepper,
    /// Pawn-row pieces: P, GB, D, N, L, OC, TG.
    Pawn,
    /// Hook movers: HM, LO, CA, PC.
    Hook,
    /// Range-cap or Lion-iguib only.
    Special,
    /// Fallback when the classifier can't decide.
    Other,
}

impl Family {
    /// Human-readable label (used in logs and analysis).
    pub fn label(self) -> &'static str {
        match self {
            Family::Royal => "Royal",
            Family::RangeCap => "Range-capturer",
            Family::Lion => "Lion (area)",
            Family::Eagle => "Eagle (queen+jump)",
            Family::Dragon => "Dragon (W+B)",
            Family::Demon => "Demon (long-range)",
            Family::Standard => "Standard (B3R)",
            Family::Slider => "Slider",
            Family::Stepper => "Stepper",
            Family::Pawn => "Pawn-rank",
            Family::Hook => "Hook mover",
            Family::Special => "Special",
            Family::Other => "Other",
        }
    }
}

/// Per-family value weight.  The engine's raw `pieces::value` is a baseline;
/// the family weight acts as a multiplier / strategic bias.
pub const FAMILY_WEIGHT: &[i32] = &[
    0,   // unused index 0
    100, // Royal
    90,  // RangeCap
    85,  // Lion
    70,  // Eagle
    60,  // Dragon
    50,  // Demon
    45,  // Standard
    30,  // Slider
    25,  // Stepper
    10,  // Pawn
    20,  // Hook
    60,  // Special
    15,  // Other
];

/// Compute the family of a piece type.
///
/// This is the original classification logic. For hot paths (evaluate),
/// use [`classify_fast`] which does an O(1) table lookup instead.
pub fn classify(piece_type: u16) -> Family {
    if piece_type == 0 {
        return Family::Other;
    }
    let m = pieces::movement(piece_type);
    let rank = pieces::rank(piece_type);

    // Royal overrides everything.
    if rank == RANK_ROYAL {
        return Family::Royal;
    }

    // Range capturers.
    if !m.range_capture.is_empty() {
        return Family::RangeCap;
    }

    // Lions (area movers with igui).
    if m.area > 0 && m.igui {
        return Family::Lion;
    }

    // Hook movers.
    if m.hook.is_some() {
        return Family::Hook;
    }

    // Eagles: queen + significant jump capability.
    let queen = m.slides.iter().all(|&(_d, r)| r == 0) && m.slides.len() == 8;
    if queen && !m.jumps.is_empty() {
        return Family::Eagle;
    }

    // Dragons: half-queen (W + B or B + R).
    if m.slides.len() == 4
        && m.jumps.iter().all(|&(r, c)| r.abs() == c.abs())
        && !m.jumps.is_empty()
    {
        return Family::Dragon;
    }

    // Demons: queen + long-range attacks (more than 6 squares in a dir).
    let has_long_slide = m.slides.iter().any(|&(_, r)| r == 0 || r >= 5);
    let wide = m.slides.len() >= 4;
    if has_long_slide && wide {
        return Family::Demon;
    }

    // Standards: B + R combination (a rook + bishop base).
    let has_b = m.slides.iter().any(|&(d, _)| {
        matches!(
            d as usize,
            crate::types::NE | crate::types::SE | crate::types::SW | crate::types::NW
        )
    });
    let has_r = m.slides.iter().any(|&(d, _)| {
        matches!(
            d as usize,
            crate::types::N | crate::types::S | crate::types::E | crate::types::W
        )
    });
    if has_b && has_r {
        return Family::Standard;
    }

    // Sliders vs steppers.
    if m.slides.iter().any(|&(_, r)| r == 0) {
        return Family::Slider;
    }

    // Pawn row.
    // Hard-coded pawn-like pieces: tiny value and forward-only step.
    let max_range = m.slides.iter().map(|&(_, r)| r).max().unwrap_or(0);
    if max_range <= 2 {
        // Check piece name for "Pawn" by value — pawns are < 1500.
        if pieces::value(piece_type) <= 1000 {
            return Family::Pawn;
        }
        return Family::Stepper;
    }

    Family::Other
}

/// Promotion multiplier — promoted forms are worth considerably more.
pub const PROMOTION_MULT: i32 = 2;

/// Compute the value contribution of a single piece.
///
/// This is the original computation. For hot paths (evaluate), use
/// [`family_value_fast`] which does an O(1) table lookup instead.
pub fn family_value(piece_type: u16) -> i32 {
    let fam = classify(piece_type);
    let weight = FAMILY_WEIGHT[fam as usize];
    let base = pieces::value(piece_type);

    // Promoted pieces get a strong bonus.
    let promo_mult = if pieces::is_royal(piece_type) {
        1
    } else {
        // Heuristic: pieces whose name starts with "+" are promoted. The
        // engine stores them with the same `value()` as their base, so we
        // add a flat bonus here.
        let abbrev = pieces::abbrev(piece_type);
        if abbrev.starts_with('+') {
            PROMOTION_MULT
        } else {
            1
        }
    };

    // Blend the per-piece value and the family weight (50/50).
    (base * promo_mult + weight * 50) / 2
}

// ── Precomputed lookup tables ──────────────────────────────────

static FAMILY_TABLE: OnceLock<[Family; 512]> = OnceLock::new();
static FAMILY_VALUE_TABLE: OnceLock<[i32; 512]> = OnceLock::new();

fn family_table() -> &'static [Family; 512] {
    FAMILY_TABLE.get_or_init(|| {
        let mut table = [Family::Other; 512];
        for pt in 0..512u16 {
            table[pt as usize] = classify(pt);
        }
        table
    })
}

fn family_value_table() -> &'static [i32; 512] {
    FAMILY_VALUE_TABLE.get_or_init(|| {
        let mut table = [0i32; 512];
        for pt in 0..512u16 {
            table[pt as usize] = family_value(pt);
        }
        table
    })
}

/// Maximum valid piece type ID (inclusive). Piece IDs are 1-based with
/// `0 = no piece`, so valid piece types are `1..=PIECE_TYPE_COUNT`.
const PIECE_TYPE_COUNT: usize = 301;

/// Fast classify via precomputed table — O(1) lookup.
///
/// Use this in hot paths like `evaluate()` instead of `classify()`.
/// The table is built once on first call (lazy init via `OnceLock`).
///
/// Returns `Family::Other` for out-of-range piece types instead of
/// silently clamping to the last table entry.
#[inline]
pub fn classify_fast(piece_type: u16) -> Family {
    let index = piece_type as usize;
    if index > PIECE_TYPE_COUNT {
        return Family::Other;
    }
    family_table()[index]
}

/// Fast family_value via precomputed table — O(1) lookup.
///
/// Use this in hot paths like `evaluate()` instead of `family_value()`.
/// The table is built once on first call (lazy init via `OnceLock`).
///
/// Returns `0` for out-of-range piece types instead of silently
/// clamping to the last table entry.
#[inline]
pub fn family_value_fast(piece_type: u16) -> i32 {
    let index = piece_type as usize;
    if index > PIECE_TYPE_COUNT {
        return 0;
    }
    family_value_table()[index]
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `classify_fast` matches `classify` for every valid piece type.
    #[test]
    fn test_fast_classification_matches_original() {
        for piece_type in 0..=PIECE_TYPE_COUNT as u16 {
            assert_eq!(
                classify(piece_type),
                classify_fast(piece_type),
                "Mismatch for piece_type {}",
                piece_type
            );
        }
    }

    /// Verify that `family_value_fast` matches `family_value` for every valid piece type.
    #[test]
    fn test_fast_value_matches_original() {
        for piece_type in 0..=PIECE_TYPE_COUNT as u16 {
            assert_eq!(
                family_value(piece_type),
                family_value_fast(piece_type),
                "Mismatch for piece_type {}",
                piece_type
            );
        }
    }

    /// Out-of-range piece types must not panic or return stale data.
    /// PIECE_TYPE_COUNT (301) is a valid piece type; 302 and above are not.
    #[test]
    fn test_fast_functions_reject_out_of_range() {
        assert_eq!(classify_fast((PIECE_TYPE_COUNT + 1) as u16), Family::Other);
        assert_eq!(classify_fast(u16::MAX), Family::Other);
        assert_eq!(family_value_fast((PIECE_TYPE_COUNT + 1) as u16), 0);
        assert_eq!(family_value_fast(u16::MAX), 0);
    }
}
