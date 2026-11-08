//! NNUE piece classes: the 301 piece types are folded into a small number
//! of classes (about 27) by movement pattern and value tier.
//!
//! The assignment is a **pure, deterministic function** of the piece table
//! (`pieces.rs`). It is frozen in `rules/nnue_arch.json`; the unit test
//! `class_map_matches_frozen_arch` fails whenever `pieces.rs` changes in a
//! way that alters it, which forces a regeneration (`nnue gen-classes`) and,
//! because the class map is part of the architecture hash, a retraining.
//!
//! Piece ids are 1-based (0 = no piece), so the class map has
//! `num_piece_types() + 1` entries and entry 0 is unused (always 0).

use crate::pieces;

/// Bit mask of the orthogonal directions (N, E, S, W) in a direction set.
const ORTH: u32 = 0b0101_0101;
/// Bit mask of the diagonal directions (NE, SE, SW, NW).
const DIAG: u32 = 0b1010_1010;

fn tier(value: i32, thresholds: &[i32]) -> u8 {
    thresholds
        .iter()
        .position(|&t| value <= t)
        .unwrap_or(thresholds.len()) as u8
}

/// Sort key of a piece type. Equal keys share a class; class ids are the
/// ranks of the distinct keys in ascending order.
fn class_key(pt: u16) -> (u8, u8, u8) {
    let m = pieces::movement(pt);
    let value = pieces::value(pt);

    if pieces::is_royal(pt) {
        return (0, 0, 0);
    }
    if !m.range_capture.is_empty() {
        return (1, 0, 0);
    }
    if m.area > 0 || m.igui {
        return (2, 0, 0);
    }
    if m.hook.is_some() {
        return (3, 0, 0);
    }

    // Directions with an unlimited slide (bit d = direction d, see
    // `types::N..NW`; bit 0 = N = "forward" for the owner).
    let mut unlimited = 0u32;
    for &(d, range) in &m.slides {
        if range == 0 {
            unlimited |= 1 << d;
        }
    }
    if unlimited == 0 {
        // Steppers and pure jumpers, by value tier.
        return (4, tier(value, &[1000, 1500, 2000, 2800, 3500]), 0);
    }

    let orth = (unlimited & ORTH).count_ones();
    let diag = (unlimited & DIAG).count_ones();
    let n = orth + diag;
    let forward = (unlimited & 1) as u8;

    if diag == 0 {
        // Orthogonal-only sliders.
        return match n {
            1 => (5, tier(value, &[3000]), 0),
            2 if forward == 0 => (6, 0, 0),
            2 => (6, 1, tier(value, &[3500])),
            _ => (7, tier(value, &[4500]), 0),
        };
    }
    if orth == 0 {
        // Diagonal-only sliders.
        return (8, if n <= 2 { 0 } else { 1 }, 0);
    }
    // Mixed (orthogonal + diagonal) sliders, by number of directions.
    match n {
        0..=3 => (9, tier(value, &[4000]), 0),
        4 => (10, tier(value, &[4500]), 0),
        5..=6 => (11, tier(value, &[5000]), 0),
        _ => (12, tier(value, &[6000]), 0),
    }
}

/// Generate the class map from the piece table: `map[piece_type]` is the
/// class id, for `piece_type` in `1..=num_piece_types()`; `map[0] == 0`.
pub fn generate_class_map() -> Vec<u8> {
    let n = pieces::num_piece_types();
    let keys: Vec<(u8, u8, u8)> = (1..=n as u16).map(class_key).collect();
    let mut distinct = keys.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert!(distinct.len() <= 255, "too many NNUE classes");
    let mut map = vec![0u8; n + 1];
    for (i, k) in keys.iter().enumerate() {
        map[i + 1] = distinct.binary_search(k).unwrap() as u8;
    }
    map
}

/// Number of classes of a class map (max id + 1).
pub fn num_classes(map: &[u8]) -> usize {
    map.iter()
        .skip(1)
        .copied()
        .max()
        .map_or(0, |m| m as usize + 1)
}

/// Mean material value (engine units, pawn = 500) of the pieces of each
/// class. Not part of the architecture: the trainer uses it to start the
/// PSQT as a plain material count.
pub fn class_values(map: &[u8]) -> Vec<i32> {
    (0..num_classes(map))
        .map(|cls| {
            let vals: Vec<i64> = (1..map.len())
                .filter(|&pt| map[pt] as usize == cls)
                .map(|pt| pieces::value(pt as u16) as i64)
                .collect();
            ((vals.iter().sum::<i64>() as f64) / vals.len() as f64).round() as i32
        })
        .collect()
}

/// Human-readable table: class id, member count and member abbreviations.
pub fn describe(map: &[u8]) -> String {
    let c = num_classes(map);
    let mut out = String::new();
    for cls in 0..c {
        let members: Vec<&str> = (1..map.len())
            .filter(|&pt| map[pt] as usize == cls)
            .map(|pt| pieces::abbrev(pt as u16))
            .collect();
        let shown: Vec<&str> = members.iter().take(8).copied().collect();
        out.push_str(&format!(
            "class {:2}  n={:3}  {}{}\n",
            cls,
            members.len(),
            shown.join(","),
            if members.len() > shown.len() {
                ",..."
            } else {
                ""
            }
        ));
    }
    out
}
