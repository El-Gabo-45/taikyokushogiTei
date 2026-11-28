#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! Movement-derived features for every piece type.
//!
//! Tomatene generates its piece table from movement descriptors. Your engine
//! keeps an explicit (hard-coded) piece table and movegen — this module does
//! **not** change either. It only reads the existing descriptors through
//! [`pieces::movement`] and exports a small numeric feature vector per piece
//! type, useful as NNUE metadata, analysis and cross-engine comparisons.

use crate::pieces;
use std::fmt::Write;

// These booleans are independent metadata flags (not a state machine), so the
// "too many bools" lint does not apply to this read-only struct.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MovementFeatures {
    /// Number of sliding directions.
    pub slides: u8,
    /// Longest sliding range (0 = unlimited).
    pub max_slide_range: u8,
    /// Number of jumping/step directions.
    pub jumps: u8,
    /// Number of range-capture directions.
    pub range_capture_dirs: u8,
    /// Lion area: 0 none, 2 lion, 3 lion-dog.
    pub area: u8,
    pub hook: bool,
    pub igui: bool,
    pub promotes: bool,
    pub royal: bool,
    pub value: i32,
}

/// Feature vector for piece type `pt`.
#[must_use]
pub fn features(pt: u16) -> MovementFeatures {
    let m = pieces::movement(pt);
    MovementFeatures {
        slides: m.slides.len() as u8,
        max_slide_range: m.slides.iter().map(|&(_, range)| range).max().unwrap_or(0),
        jumps: m.jumps.len() as u8,
        range_capture_dirs: m.range_capture.len() as u8,
        area: m.area,
        hook: m.hook.is_some(),
        igui: m.igui,
        promotes: pieces::promotes_to(pt).is_some(),
        royal: pieces::is_royal(pt),
        value: pieces::value(pt),
    }
}

/// JSON array with one object per piece type, keyed by numeric type id.
/// Mirrors the shape of `training/piece_metadata.json` but adds the movement
/// features; it is a data export, not a code generator.
#[must_use]
pub fn json_metadata() -> String {
    let mut out = String::from("[\n");
    for pt in 0..pieces::num_piece_types() as u16 {
        let f = features(pt);
        if pt > 0 {
            out.push_str(",\n");
        }
        let _ = write!(
            out,
            "  {{\"id\":{pt},\"name\":\"{}\",\"slides\":{},\"max_range\":{},\"jumps\":{},\"range_dirs\":{},\"area\":{},\"hook\":{},\"igui\":{},\"promotes\":{},\"royal\":{},\"value\":{}}}",
            pieces::abbrev(pt),
            f.slides,
            f.max_slide_range,
            f.jumps,
            f.range_capture_dirs,
            f.area,
            f.hook,
            f.igui,
            f.promotes,
            f.royal,
            f.value
        );
    }
    out.push_str("\n]\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_piece_has_features() {
        for pt in 0..pieces::num_piece_types() as u16 {
            let _ = features(pt);
        }
    }

    #[test]
    fn json_is_indexed_and_complete() {
        let json = json_metadata();
        assert!(json.starts_with("[\n"));
        assert!(json.contains("\"id\":0"));
        let last = pieces::num_piece_types() - 1;
        assert!(json.contains(&format!("\"id\":{last}")));
    }

    #[test]
    fn royals_are_marked() {
        let royal = pieces::find_by_abbrev("K").unwrap_or(0);
        assert!(pieces::is_royal(royal), "king must be a royal piece");
        assert!(features(royal).royal);
    }
}
