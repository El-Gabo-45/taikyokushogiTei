#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

//! Quiescence search: capture-only extension at the leaves.
//!
//! Reuses the TT for shallow results (captures transpose constantly on a
//! 36×36 board, so caching here saves whole quiescence subtrees) and is
//! depth-capped (`MAX_QDEPTH`) because without a cap a single call once
//! consumed 300,000 nodes on this game's open boards.
//!
//! Like `pvs`, this module allocates nothing: the capture list and the
//! scoring list come from the node's per-ply pooled buffer.

use super::ordering::{self, piece_vals};
use super::params;
use super::pvs::Ctx;
use super::tt;
use crate::eval::{MATE_SCORE, evaluate};
use crate::types::{BLACK, GameResult, WHITE};

/// Generate and order the captures for one quiescence node.
fn score_captures(ctx: &Ctx<'_>, pb: &mut super::buffers::PlyBuffers) {
    crate::movegen::generate_pseudo_legal_captures_into(&mut pb.caps, ctx.board);
    let values = piece_vals();
    pb.scored.clear();
    for (i, m) in pb.caps.iter().enumerate() {
        let s = ordering::capture_qs_score(ctx.board, m, values);
        if s < params::QS_PREFILTER {
            continue;
        }
        pb.scored.push((s, i as u32, 0));
    }
    pb.scored.sort_unstable_by_key(|e| std::cmp::Reverse(e.0));
}

/// Terminal score from the mover's perspective, with the quiescence ply.
fn terminal_qscore(result: GameResult, stm: u8, qd: u32) -> i32 {
    match result {
        GameResult::BlackWins => {
            if stm == BLACK {
                MATE_SCORE - qd as i32
            } else {
                -(MATE_SCORE - qd as i32)
            }
        }
        GameResult::WhiteWins => {
            if stm == WHITE {
                MATE_SCORE - qd as i32
            } else {
                -(MATE_SCORE - qd as i32)
            }
        }
        GameResult::Draw => 0,
    }
}

/// Cached quiescence score, if the TT entry is usable at this window.
fn probe_qtt(ctx: &Ctx<'_>, q_hash: u64, alpha: i32, beta: i32) -> Option<i32> {
    let entry = ctx.tt.probe(q_hash)?;
    match entry.flag {
        0 => Some(entry.score),
        1 if entry.score >= beta => Some(entry.score),
        2 if entry.score <= alpha => Some(entry.score),
        _ => None,
    }
}

/// Quiescence entry (qd = 0 from the main search).
pub(crate) fn run(ctx: &mut Ctx, alpha: i32, beta: i32) -> i32 {
    ctx.qsearch(alpha, beta, 0)
}

impl Ctx<'_> {
    fn qsearch(&mut self, mut alpha: i32, beta: i32, qd: u32) -> i32 {
        *self.nodes += 1;
        let init_q_alpha = alpha;
        // Same reasoning as pvs(): nodes here are expensive (movegen ~120-180us),
        // so check the clock much more often than a typical chess engine would.
        if *self.nodes & params::NODES_PER_CLOCK_CHECK_QS == 0 && self.past_deadline() {
            return alpha;
        }

        // Terminal check (same semantics as pvs, with the quiescence ply).
        if let Some(result) = self.board.game_result() {
            return terminal_qscore(result, self.board.side_to_move, qd);
        }

        // Reuse shallow leaf results: captures transpose constantly on 36×36.
        let q_hash = self.board.tt_key();
        if qd > 0 {
            if let Some(score) = probe_qtt(self, q_hash, alpha, beta) {
                return score;
            }
        }

        // Stand pat
        let stand_pat = evaluate(self.board);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }
        if qd >= params::MAX_QDEPTH {
            return alpha;
        }

        // Generate only captures and promotions (staged move generation —
        // Reference: docx §3.2 Futility Pruning & §4.4 Quiescence Search).
        // The capture-only generator skips the ~700 quiet moves, so QS now
        // scales with the number of pieces that can actually capture.
        //
        // QS gets its own slice of the pool above the PV plies: a PV node at
        // ply P never holds a QS slot (qsearch is entered at d == 0, before
        // the PV node takes its buffer), so `ply + qd` can never collide with
        // a live PV buffer.
        let slot = self.ply as usize + qd as usize;
        let mut pb = self.pool.take(slot);
        score_captures(self, &mut pb);

        // `early` reproduces the old straight-line `return`s: a stop or a beta
        // cutoff skips the TT store, while falling out of the loop stores.
        let mut early: Option<i32> = None;
        for (i_idx, &(_, i, _)) in pb.scored.iter().enumerate() {
            if i_idx > 0 && self.past_deadline() {
                early = Some(alpha);
                break;
            }
            let m = &pb.caps[i as usize];
            self.board.apply_move(m);
            let score = -self.qsearch(-beta, -alpha, qd + 1);
            self.board.undo_move();
            if score >= beta {
                early = Some(beta);
                break;
            }
            if score > alpha {
                alpha = score;
            }
        }
        // The immutable borrow of `pb.scored` above ends here, so the buffer
        // can go back to the pool before any return.
        self.pool.put(slot, pb);

        if let Some(score) = early {
            return score;
        }

        if qd > 0 {
            self.tt.store(
                q_hash,
                tt::TTEntry {
                    score: alpha,
                    depth: 0,
                    flag: if alpha <= init_q_alpha { 2 } else { 0 },
                    generation: self.tt.generation(),
                    best_move: 0,
                    // Taikyoku has no check (SPEC §7.3) — always false.
                    in_check: false,
                },
            );
        }
        alpha
    }
}
