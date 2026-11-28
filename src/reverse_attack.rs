#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! Reverse attack index ("who attacks this square?").
//!
//! Tomatene keeps a bidirectional attack map so changing a square only
//! invalidates the pieces that attack it. This module adds the reverse side
//! as an **auxiliary index**: it is built on demand from the existing move
//! generator and never changes how moves are generated or searched.
//!
//! Intended uses: cheap attacked/defended queries for evaluation features,
//! diagnostics and tooling. It is deliberately not wired into the hot search
//! path, so existing strength and performance stay untouched.

use crate::Board;
use crate::movegen::generate_pseudo_legal_moves;
use crate::types::{BLACK, NUM_SQUARES, WHITE};

#[derive(Clone, Default)]
pub struct ReverseAttackMap {
    attackers: Vec<Vec<u16>>,
}

impl ReverseAttackMap {
    #[must_use]
    pub fn new() -> Self {
        Self {
            attackers: vec![Vec::new(); NUM_SQUARES],
        }
    }

    /// Rebuild the index from both sides' pseudo-legal moves.
    pub fn rebuild(&mut self, board: &Board) {
        for list in &mut self.attackers {
            list.clear();
        }
        for color in [BLACK, WHITE] {
            let mut probe = board.inner.clone();
            probe.side_to_move = color;
            for mv in generate_pseudo_legal_moves(&probe) {
                let attacker = mv.from_sq;
                let destinations = [
                    Some(mv.to_sq),
                    (mv.mid_sq != crate::types::INVALID_SQ).then_some(mv.mid_sq),
                ];
                for target in destinations.into_iter().flatten() {
                    let list = &mut self.attackers[target as usize];
                    if !list.contains(&attacker) {
                        list.push(attacker);
                    }
                }
            }
        }
    }

    /// Squares occupied by pieces that can reach `sq`.
    #[must_use]
    pub fn attackers_of(&self, sq: usize) -> &[u16] {
        self.attackers.get(sq).map_or(&[], Vec::as_slice)
    }

    /// Whether any piece can reach `sq`.
    #[must_use]
    pub fn is_attacked(&self, sq: usize) -> bool {
        !self.attackers_of(sq).is_empty()
    }

    /// Total number of indexed (attacker, target) relations.
    #[must_use]
    pub fn relation_count(&self) -> usize {
        self.attackers.iter().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_position_has_reverse_attacks() {
        let board = Board::initial();
        let mut map = ReverseAttackMap::new();
        map.rebuild(&board);
        assert!(map.relation_count() > 0);
        // The pawn rows attack squares directly in front of them.
        assert!(map.is_attacked(18 * 36 + 17) || map.is_attacked(17 * 36 + 17));
    }

    #[test]
    fn index_is_consistent_with_movegen() {
        let board = Board::initial();
        let mut map = ReverseAttackMap::new();
        map.rebuild(&board);
        for mv in board.legal_moves() {
            let to = mv.raw().to_sq as usize;
            let from = mv.raw().from_sq;
            assert!(
                map.attackers_of(to).contains(&from),
                "move {from}->{to} missing from reverse index"
            );
        }
    }
}
