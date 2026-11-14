#![allow(dead_code)] // utility/debug API kept intentionally
use crate::eval::families::family_value;
use crate::eval::zones::zone_score;
use crate::pieces;
use crate::types::{cell_piece, BLACK, EMPTY_CELL, INVALID_SQ, WHITE};

/// Resumen detallado del heuristic para inspección rápida.
#[derive(Debug, Clone)]
pub struct HeuristicDebugReport {
    pub total: i32,
    pub material: i32,
    pub family_weight: i32,
    pub zones: i32,
    /// Royal-center penalty (`psqt_value` subtracts `2 * Manhattan distance
    /// to (17,17)` for every royal piece; this is that term from Black's
    /// perspective). Tracked separately so the decomposition reproduces the
    /// engine's hand-crafted score exactly.
    pub royal_center: i32,
    pub king_safety: i32,
    pub by_piece: Vec<(String, i32)>,
}

/// Devuelve un reporte legible del valor heurístico actual.
pub fn debug_heuristic(board: &crate::Board) -> HeuristicDebugReport {
    let b = &board.inner;
    let mut material = 0i32;
    let mut family_weight = 0i32;
    let mut royal_center = 0i32;
    let mut by_piece: Vec<(String, i32)> = Vec::new();

    for color in [BLACK, WHITE] {
        let sign = if color == BLACK { 1 } else { -1 };
        for i in 0..b.piece_list_len[color as usize] {
            let sq = b.piece_list[color as usize][i] as usize;
            if sq == INVALID_SQ as usize {
                continue;
            }
            let cell = b.cells[sq];
            if cell == EMPTY_CELL {
                continue;
            }
            let pt = cell_piece(cell);
            let piece_name = pieces::abbrev(pt).to_string();
            let value = pieces::value(pt);
            let family = family_value(pt);
            material += sign * value;
            family_weight += sign * family;

            // Same royal-center penalty as eval/psqt.rs::psqt_value.
            if pieces::is_royal(pt) {
                let r = (sq / crate::types::BOARD_SIZE) as i32;
                let c = (sq % crate::types::BOARD_SIZE) as i32;
                let dist = (r - 17).abs() + (c - 17).abs();
                royal_center -= sign * dist * 2;
            }

            if let Some((_, existing)) = by_piece.iter_mut().find(|(name, _)| name == &piece_name) {
                *existing += sign * value;
            } else {
                by_piece.push((piece_name, sign * value));
            }
        }
    }

    by_piece.sort_by(|a, b| b.1.cmp(&a.1));

    let zones = zone_score(b, BLACK) - zone_score(b, WHITE);
    // Same king-safety term the engine's hand-crafted evaluator uses
    // (threat-zone bitboard ∩ friendly occupancy) — reusing it keeps this
    // decomposition exactly equal to the engine's score.
    let king_safety = crate::eval::king_safety(b, BLACK) - crate::eval::king_safety(b, WHITE);
    let total = family_weight + zones + royal_center + king_safety;

    HeuristicDebugReport {
        total,
        material,
        family_weight,
        zones,
        royal_center,
        king_safety,
        by_piece,
    }
}

/// Convierte el reporte en una cadena formateada para impresión o logs.
pub fn format_heuristic_debug(board: &crate::Board) -> String {
    let report = debug_heuristic(board);
    let mut lines = vec![
        format!("heuristic_total={}", report.total),
        format!("material={}", report.material),
        format!("family_weight={}", report.family_weight),
        format!("zones={}", report.zones),
        format!("royal_center={}", report.royal_center),
        format!("king_safety={}", report.king_safety),
    ];

    if !report.by_piece.is_empty() {
        lines.push("by_piece=".to_string());
        for (name, value) in report.by_piece {
            lines.push(format!("  {name}={value}"));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hand-crafted decomposition must reproduce the engine's
    /// hand-crafted score exactly — this is what makes `debug eval`
    /// trustworthy. Walks a few positions from the start.
    #[test]
    fn decomposition_matches_handcrafted_eval() {
        let _serial = crate::test_lock::lock();
        let mut b = crate::Board::initial();
        check(&b, 0);
        let mut rng = 0x1234_5678_9ABC_DEF0u64;
        for ply in 1..=20u32 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let moves = b.legal_moves();
            if moves.is_empty() || b.game_result().is_some() {
                break;
            }
            b.apply(&moves[((rng >> 33) as usize) % moves.len()]);
            check(&b, ply);
        }
    }

    fn check(b: &crate::Board, ply: u32) {
        if b.game_result().is_some() {
            return;
        }
        let r = debug_heuristic(b);
        let expected = if b.side_to_move() == crate::Color::Black {
            r.total
        } else {
            -r.total
        };
        let hand = b.evaluate();
        assert_eq!(
            hand,
            expected,
            "ply {ply}: decomposition {} != engine {} (tsfen {})",
            expected,
            hand,
            b.to_tsfen()
        );
    }

    /// The incrementally-maintained `psqt_score` must equal a fresh sum of
    /// `psqt(pt, sq, color)` over the piece list.
    #[test]
    fn incremental_psqt_matches_recompute() {
        let _serial = crate::test_lock::lock();
        let mut b = crate::Board::initial();
        assert!(psqt_matches(&b), "initial position");
        let mut rng = 0x1234_5678_9ABC_DEF0u64;
        for ply in 1..=20u32 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let moves = b.legal_moves();
            if moves.is_empty() || b.game_result().is_some() {
                break;
            }
            b.apply(&moves[((rng >> 33) as usize) % moves.len()]);
            assert!(
                psqt_matches(&b),
                "ply {ply}: incremental psqt_score drifted (tsfen {})",
                b.to_tsfen()
            );
        }
    }

    fn psqt_matches(b: &crate::Board) -> bool {
        let bb = &b.inner;
        let mut sum = 0i32;
        for color in [BLACK, WHITE] {
            let sign: i32 = if color == BLACK { 1 } else { -1 };
            for i in 0..bb.piece_list_len[color as usize] {
                let sq = bb.piece_list[color as usize][i] as usize;
                if sq >= crate::types::NUM_SQUARES {
                    continue;
                }
                let cell = bb.cells[sq];
                if cell == EMPTY_CELL {
                    continue;
                }
                sum += sign * crate::eval::psqt::psqt(cell_piece(cell), sq, color);
            }
        }
        sum == bb.psqt_score
    }
}
