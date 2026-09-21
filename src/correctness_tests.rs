//! Centralized correctness suite for the engine (runs with `cargo test`).
//!
//! Covers the invariants that used to live only in examples/ tools:
//! TSFEN round trips, perft golden values, apply/undo state restoration,
//! effect-dedup of generated moves, range-capture application, and search
//! returning legal moves without corrupting the board.

use crate::{Board, Color};

// ── Deterministic LCG for reproducible random walks ─────────────
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() as usize) % n }
    }
}

// ── TSFEN round trip ────────────────────────────────────────────
#[test]
fn tsfen_roundtrip_initial() {
    let b = Board::initial();
    let tsfen = b.to_tsfen();
    let restored = Board::from_tsfen(&tsfen).expect("initial TSFEN must parse");
    assert_eq!(restored.to_tsfen(), tsfen);
    assert_eq!(restored.piece_count(Color::Black), 402);
    assert_eq!(restored.piece_count(Color::White), 402);
    assert_eq!(restored.legal_moves().len(), 512);
}

#[test]
fn tsfen_roundtrip_after_random_walk() {
    let mut rng = Lcg(0xDEADBEEF);
    let mut board = Board::initial();
    for _ in 0..40 {
        let moves = board.legal_moves();
        if moves.is_empty() { break; }
        board.apply(&moves[rng.pick(moves.len())]);
    }
    let tsfen = board.to_tsfen();
    let restored = Board::from_tsfen(&tsfen).expect("walk TSFEN must parse");
    assert_eq!(restored.to_tsfen(), tsfen);
    assert_eq!(restored.material_score(), board.material_score());
    assert_eq!(restored.legal_moves().len(), board.legal_moves().len());
}

// ── Perft golden values (current tree, dedup ON) ────────────────
#[test]
fn perft_golden_initial() {
    let mut board = Board::initial();
    assert_eq!(perft(&mut board, 1), 512);
    assert_eq!(perft(&mut board, 2), 260_917);
}

fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = board.legal_moves();
    if depth == 1 { return moves.len() as u64; }
    let mut n = 0;
    for m in &moves {
        board.apply(m);
        n += perft(board, depth - 1);
        board.undo();
    }
    n
}

// ── apply/undo state restoration ────────────────────────────────
#[test]
fn apply_undo_restores_state() {
    let mut rng = Lcg(0xC0FFEE);
    let mut board = Board::initial();
    for ply in 0..80 {
        let before_tsfen = board.to_tsfen();
        let before_mat = board.material_score();
        let before_counts = (board.piece_count(Color::Black), board.piece_count(Color::White));
        let moves = board.legal_moves();
        if moves.is_empty() { break; }
        let m = moves[rng.pick(moves.len())].clone();
        board.apply(&m);
        board.undo();
        assert_eq!(board.to_tsfen(), before_tsfen, "cells diverged at ply {}", ply);
        assert_eq!(board.material_score(), before_mat, "material diverged at ply {}", ply);
        assert_eq!(
            (board.piece_count(Color::Black), board.piece_count(Color::White)),
            before_counts, "piece counts diverged at ply {}", ply
        );
        board.apply(&moves[(ply * 7919 + 13) % moves.len()]);
    }
}

// ══════════════════════════════════════════════════════════════════
// Crafted-position and rule tests (board.rs internals are crate-visible)
// ══════════════════════════════════════════════════════════════════
use crate::board;
use crate::types::{self, make_cell, cell_piece, cell_color, BLACK, WHITE, EMPTY_CELL, NUM_SQUARES, INVALID_SQ, DRAW_PLIES, GameResult, Move};
use crate::pieces;
use crate::movegen;

type IntBoard = board::Board;

/// Initial position as the internal board (crate tests need the raw type).
fn initial_int() -> IntBoard {
    let mut b = board::Board::new();
    b.setup_initial();
    b
}

fn empty_board() -> IntBoard { board::Board::new() }

fn put(b: &mut IntBoard, row: usize, col: usize, pt: u16, color: u8) {
    b.cells[row * 36 + col] = make_cell(pt, color);
}

fn find_piece(name: &str) -> u16 {
    (1..=301u16).find(|&pt| pieces::name(pt) == name)
        .unwrap_or_else(|| panic!("piece `{}` not found in PIECE_DEFS", name))
}

fn moves_from(b: &IntBoard, row: usize, col: usize) -> Vec<Move> {
    movegen::generate_pseudo_legal_moves(b).into_iter()
        .filter(|m| m.from_sq as usize == row * 36 + col)
        .collect()
}

#[test]
fn legal_equals_pseudo_legal_random_walk() {
    let mut rng = Lcg(0xAB_CDEF);
    let mut b = initial_int();
    for _ in 0..20 {
        let legal = movegen::generate_legal_moves(&b);
        let pseudo = movegen::generate_pseudo_legal_moves(&b);
        assert_eq!(legal.len(), pseudo.len());
        let key = |m: &Move| (m.from_sq, m.to_sq, m.promotion, m.mid_sq, m.is_igui,
                              m.captured_piece, m.range_cap, m.caps_value);
        let mut seen_l = std::collections::HashSet::new();
        let mut seen_p = std::collections::HashSet::new();
        for m in &legal { seen_l.insert(key(m)); }
        for m in &pseudo { seen_p.insert(key(m)); }
        assert_eq!(seen_l, seen_p, "legal != pseudo-legal move sets");
        let moves = legal;
        if moves.is_empty() { break; }
        b.apply_move(&moves[rng.pick(moves.len())]);
    }
}

// ── Zobrist + incremental scores restored by every undo ──────────
#[test]
fn undo_restores_hash_and_scores_every_ply() {
    let mut rng = Lcg(0x0DDBA11);
    let mut b = initial_int();
    for ply in 0..60 {
        let (h, mat, psqt) = (b.hash, b.material_score, b.psqt_score);
        let moves = movegen::generate_legal_moves(&b);
        if moves.is_empty() { break; }
        b.apply_move(&moves[rng.pick(moves.len())]);
        b.undo_move();
        assert_eq!(b.hash, h, "hash diverged at ply {}", ply);
        assert_eq!(b.material_score, mat, "material diverged at ply {}", ply);
        assert_eq!(b.psqt_score, psqt, "psqt diverged at ply {}", ply);
        b.apply_move(&moves[(ply * 104729) % moves.len()]);
    }
}

// ── Zobrist transposition: independent move orders hash identically ──
#[test]
fn zobrist_transposition_independent_moves() {
    let quiet = |m: &Move| {
        m.captured_piece == 0 && m.mid_sq == INVALID_SQ && !m.range_cap
            && m.from_sq != m.to_sq && !m.is_igui
    };
    let mut b = initial_int();
    let mvs = movegen::generate_legal_moves(&b);
    let quiet_black: Vec<Move> = mvs.iter().filter(|m| quiet(m)).cloned().collect();

    // find two black quiets with fully disjoint squares
    let mut found = None;
    for (i, m1) in quiet_black.iter().enumerate() {
        for m2 in &quiet_black[i + 1..] {
            let sqs = [m1.from_sq, m1.to_sq, m2.from_sq, m2.to_sq];
            if sqs.iter().collect::<std::collections::HashSet<_>>().len() == 4 {
                found = Some((m1.clone(), m2.clone()));
                break;
            }
        }
        if found.is_some() { break; }
    }
    let (m1, m2) = found.expect("initial position has many quiet moves");
    let used: std::collections::HashSet<u16> = [m1.from_sq, m1.to_sq, m2.from_sq, m2.to_sq].into();

    // a white quiet move w disjoint from all four squares
    b.apply_move(&m2);
    let wmvs = movegen::generate_legal_moves(&b);
    let w = wmvs.iter().find(|m| quiet(m)
        && !used.contains(&m.from_sq) && !used.contains(&m.to_sq))
        .expect("quiet white move exists").clone();
    b.undo_move();

    // path A: m1, w, m2  — path B: m2, w, m1
    b.apply_move(&m1);
    // w must be legal and identical after m1 too
    let w_after_m1 = movegen::generate_legal_moves(&b).into_iter()
        .find(|m| m.from_sq == w.from_sq && m.to_sq == w.to_sq && m.promotion == w.promotion)
        .expect("white quiet move legal after m1 as well");
    b.apply_move(&w_after_m1);
    let m2_after = movegen::generate_legal_moves(&b).into_iter()
        .find(|m| m.from_sq == m2.from_sq && m.to_sq == m2.to_sq)
        .expect("second black move still legal");
    b.apply_move(&m2_after);
    let hash_a = b.hash;
    b.undo_move(); b.undo_move(); b.undo_move();
    assert_eq!(b.hash, initial_int().hash, "triple undo restores start");

    b.apply_move(&m2);
    let w_after_m2 = movegen::generate_legal_moves(&b).into_iter()
        .find(|m| m.from_sq == w.from_sq && m.to_sq == w.to_sq && m.promotion == w.promotion)
        .expect("white quiet move legal after m2");
    b.apply_move(&w_after_m2);
    let m1_after = movegen::generate_legal_moves(&b).into_iter()
        .find(|m| m.from_sq == m1.from_sq && m.to_sq == m1.to_sq)
        .expect("first black move still legal");
    b.apply_move(&m1_after);
    let hash_b = b.hash;

    assert_eq!(hash_a, hash_b, "transposed move orders must hash identically");
}

// ── Draw by the no-progress rule ────────────────────────────────
#[test]
fn draw_by_no_progress_rule() {
    let mut b = initial_int();
    assert!(b.game_result().is_none());
    b.no_progress_plies = DRAW_PLIES;
    assert!(matches!(b.game_result(), Some(GameResult::Draw)));
    b.no_progress_plies = DRAW_PLIES - 1;
    assert!(b.game_result().is_none());
}

// ── Losing the last royal loses ─────────────────────────────────
#[test]
fn losing_last_royal_loses() {
    let mut b = initial_int();
    for sq in 0..NUM_SQUARES {
        let cell = b.cells[sq];
        if cell != EMPTY_CELL && cell_color(cell) == WHITE && pieces::is_royal(cell_piece(cell)) {
            b.cells[sq] = EMPTY_CELL;
        }
    }
    b.rebuild_lists_pub();
    assert!(matches!(b.game_result(), Some(GameResult::BlackWins)),
        "removing all white royals must end the game with a black win");
    // black still has royals
    assert!(matches!(b.game_result(), Some(GameResult::BlackWins)));
}

// ── Igui: capture without moving ────────────────────────────────
#[test]
fn igui_captures_without_moving() {
    let mover = (1..=301u16).find(|&pt| pieces::movement(pt).igui)
        .expect("at least one piece has igui");
    let victim = find_piece("Pawn");
    let mut b = empty_board();
    put(&mut b, 17, 17, mover, BLACK);
    put(&mut b, 16, 17, victim, WHITE);
    b.rebuild_lists_pub();

    let moves = movegen::generate_pseudo_legal_moves(&b);
    let igui: Vec<Move> = moves.into_iter()
        .filter(|m| m.is_igui && m.captured_piece == victim && !m.promotion)
        .collect();
    assert!(!igui.is_empty(), "expected a non-promo igui move");

    let before = b.piece_count[0] + b.piece_count[1];
    b.apply_move(&igui[0]);
    assert_eq!(b.piece_count[0] + b.piece_count[1], before - 1, "victim removed");
    assert_eq!(cell_piece(b.cells[17 * 36 + 17]), mover, "mover stays on its square");
    assert_eq!(b.cells[16 * 36 + 17], EMPTY_CELL, "victim square empty");
    b.undo_move();
    assert_eq!(b.piece_count[0] + b.piece_count[1], before);
    assert_eq!(cell_piece(b.cells[16 * 36 + 17]), victim, "victim restored");
}

// ── Lion-type mid capture removes BOTH pieces ───────────────────
#[test]
fn area_mid_capture_removes_both_pieces() {
    let mover = (1..=301u16).find(|&pt| pieces::movement(pt).area >= 2)
        .expect("at least one piece has area>=2");
    let victim = find_piece("Pawn");
    let mut b = empty_board();
    put(&mut b, 17, 17, mover, BLACK);
    // enemies in both forward directions so the direction convention doesn't matter
    put(&mut b, 16, 17, victim, WHITE);
    put(&mut b, 15, 17, victim, WHITE);
    put(&mut b, 18, 17, victim, WHITE);
    put(&mut b, 19, 17, victim, WHITE);
    b.rebuild_lists_pub();

    let moves = movegen::generate_pseudo_legal_moves(&b);
    let mid = moves.iter().find(|m| {
        m.mid_piece != 0 && m.captured_piece != 0 && m.to_sq != m.from_sq
    }).expect("expected a mid-capture move").clone();

    let before = b.piece_count[0] + b.piece_count[1];
    b.apply_move(&mid);
    assert_eq!(b.piece_count[0] + b.piece_count[1], before - 2, "mid + landing both removed");
    assert_eq!(cell_piece(b.cells[mid.to_sq as usize]), mover, "mover landed at to");
    assert_eq!(b.cells[mid.mid_sq as usize], EMPTY_CELL, "mid square empty");
    b.undo_move();
    assert_eq!(b.piece_count[0] + b.piece_count[1], before);
}

// ── Promotion variants follow the promo-zone rules ──────────────
// Spec: the promotion zone is the 11 farthest rows; promotion is optional
// except for forward-only pieces reaching the farthest rank (must promote).
#[test]
fn pawn_promotion_variants_follow_zone_rules() {
    let pawn = find_piece("Pawn");
    // (start_row, dest_row, expected_variants, expected_promo_only)
    let cases: &[(usize, usize, usize, bool)] = &[
        (13, 12, 1, false),  // outside the zone: single non-promo move
        (11, 10, 2, false),  // entering the zone: promotion optional
        (6, 5, 1, false),    // inside zone, quiet move: no promo (only captures promote)
        (1, 0, 1, true),     // farthest rank: must promote
    ];
    for &(start, dest, variants, promo_only) in cases {
        let mut b = empty_board();
        put(&mut b, start, 17, pawn, BLACK);
        b.rebuild_lists_pub();
        let to = (dest * 36 + 17) as u16;
        let generated = moves_from(&b, start, 17);
        let matching: Vec<&Move> = generated.iter()
            .filter(|m| m.to_sq == to).collect();
        assert_eq!(matching.len(), variants,
            "black pawn {}->{}: expected {} variants", start, dest, variants);
        if promo_only {
            assert!(matching.iter().all(|m| m.promotion), "must promote at the farthest rank");
        } else if variants == 2 {
            assert!(matching.iter().any(|m| m.promotion) && matching.iter().any(|m| !m.promotion));
        }
    }
}

#[test]
fn white_pawn_promotion_mirrors_black() {
    let pawn = find_piece("Pawn");
    // white moves DOWN (increasing row); farthest rank = 35, zone = rows 25..=35
    let cases: &[(usize, usize, usize, bool)] = &[
        (22usize, 23usize, 1usize, false),
        (24, 25, 2, false),
        (34, 35, 1, true),
    ];
    for &(start, dest, variants, promo_only) in cases {
        let mut b = empty_board();
        put(&mut b, start, 17, pawn, WHITE);
        b.side_to_move = WHITE; // movegen generates for the side to move
        b.rebuild_lists_pub();
        let to = (dest * 36 + 17) as u16;
        let generated = moves_from(&b, start, 17);
        let matching: Vec<&Move> = generated.iter()
            .filter(|m| m.to_sq == to).collect();
        assert_eq!(matching.len(), variants,
            "white pawn {}->{}: expected {} variants", start, dest, variants);
        if promo_only {
            assert!(matching.iter().all(|m| m.promotion));
        }
    }
}

// ── Range-capture rank hierarchy ────────────────────────────────
// Spec §5.3: a range-capable piece flies over and captures pieces of LOWER
// status only; royals and equal-or-higher rank pieces stop the ray entirely.
#[test]
fn range_capture_respects_rank_hierarchy() {
    let ro = find_piece("Flying General"); // range-capable general
    let pawn = find_piece("Pawn");
    let king = find_piece("King");

    // lower-rank enemy on the ray is captured; the enemy King stops the ray
    let mut b = empty_board();
    put(&mut b, 17, 17, ro, BLACK);
    put(&mut b, 17, 19, pawn, WHITE);
    put(&mut b, 17, 22, king, WHITE);
    b.rebuild_lists_pub();
    let from_ro = moves_from(&b, 17, 17);
    assert!(from_ro.iter().any(|m| m.to_sq == (17 * 36 + 19) as u16 && m.captured_piece == pawn),
        "lower-rank enemy must be capturable");
    assert!(!from_ro.iter().any(|m| m.to_sq == (17 * 36 + 22) as u16),
        "royals cannot be range-captured or overflown");
    assert!(!from_ro.iter().any(|m| (m.to_sq as usize % 36) > 22 && (m.to_sq as usize / 36) == 17),
        "nothing beyond the blocking royal is reachable");

    // an equal-rank piece also stops the ray (not capturable)
    let mut b2 = empty_board();
    put(&mut b2, 17, 17, ro, BLACK);
    put(&mut b2, 17, 19, ro, WHITE);
    b2.rebuild_lists_pub();
    let from_ro2 = moves_from(&b2, 17, 17);
    assert!(!from_ro2.iter().any(|m| m.to_sq == (17 * 36 + 19) as u16),
        "equal-rank pieces must not be capturable");
}

// ── Exhaustive apply/undo over ALL initial moves ────────────────
#[test]
fn exhaustive_apply_undo_initial() {
    let mut b = initial_int();
    let moves = movegen::generate_legal_moves(&b);
    assert_eq!(moves.len(), 512);
    for m in &moves {
        let before_hash = b.hash;
        let before_cells = b.cells;
        b.apply_move(m);
        b.undo_move();
        assert_eq!(b.hash, before_hash, "hash after undo of {:?}->{:?}", m.from_sq, m.to_sq);
        assert_eq!(b.cells, before_cells, "cells after undo");
    }
}

// ── Incremental material equals the piece scan ──────────────────
#[test]
fn material_score_matches_piece_sum() {
    let mut b = initial_int();
    let mut rng = Lcg(0xFACE);
    let mut check = 0;
    for _ in 0..26 {
        let mut sum = 0i32;
        for sq in 0..NUM_SQUARES {
            let cell = b.cells[sq];
            if cell != EMPTY_CELL {
                let v = pieces::value(cell_piece(cell));
                if cell_color(cell) == BLACK { sum += v; } else { sum -= v; }
            }
        }
        assert_eq!(b.material_score, sum, "incremental material diverged (check {})", check);
        check += 1;
        let moves = movegen::generate_legal_moves(&b);
        if moves.is_empty() { break; }
        b.apply_move(&moves[rng.pick(moves.len())]);
    }
}

// ── Search respects its time budget ─────────────────────────────
#[test]
fn search_terminates_with_time_limit() {
    let start = std::time::Instant::now();
    let mut b = initial_int();
    let r = crate::search::search(&mut b, 6, 300);
    assert!(r.best_move.is_some(), "time-limited search must return a move");
    assert!(start.elapsed() < std::time::Duration::from_secs(10),
        "search(6, 300ms) took {:?}", start.elapsed());
}

// ── Selfplay smoke: a short game always stays legal ─────────────
#[test]
fn short_selfplay_game_smoke() {
    let mut b = initial_int();
    let mut plies = 0;
    while b.game_result().is_none() && plies < 30 {
        let r = crate::search::search(&mut b, 1, 0);
        match r.best_move {
            Some(m) => b.apply_move(&m),
            None => break,
        }
        plies += 1;
    }
    assert!(plies > 0, "selfplay must make progress");
    let tsfen = crate::tsfen::to_tsfen(&b);
    let restored = crate::tsfen::from_tsfen(&tsfen).expect("selfplay position must round-trip");
    assert_eq!(crate::tsfen::to_tsfen(&restored), tsfen);
}#[test]
fn bulk_unwind_restores_state() {
    let mut rng = Lcg(0x5EED);
    let mut board = Board::initial();
    for _ in 0..5 {
        let base = board.to_tsfen();
        let base_mat = board.material_score();
        let mut stack = Vec::new();
        for _ in 0..30 {
            let moves = board.legal_moves();
            if moves.is_empty() { break; }
            let m = moves[rng.pick(moves.len())].clone();
            board.apply(&m);
            stack.push(m);
        }
        while let Some(_m) = stack.pop() {
            board.undo();
        }
        assert_eq!(board.to_tsfen(), base);
        assert_eq!(board.material_score(), base_mat);
    }
}

// ── Effect dedup: no two legal moves may produce the same position ──
#[test]
fn legal_moves_have_no_duplicate_effects() {
    let mut rng = Lcg(0x1234_5678);
    let mut board = Board::initial();
    for _ in 0..15 {
        let moves = board.legal_moves();
        let mut seen = std::collections::HashSet::new();
        for m in &moves {
            let r = m.raw();
            let key = (r.from_sq, r.to_sq, r.promotion, r.mid_sq, r.mid_piece != 0,
                       r.captured_piece, r.range_cap, r.caps_value);
            assert!(seen.insert(key), "duplicate-effect move {:?}->{:?}", r.from_sq, r.to_sq);
        }
        if moves.is_empty() { break; }
        board.apply(&moves[rng.pick(moves.len())]);
    }
}

// ── Range captures must empty every occupied intermediate square ────
#[test]
fn range_capture_empties_intermediates() {
    let mut board = Board::initial();
    let moves = board.legal_moves();
    let rc = moves.iter().find(|m| {
        let r = m.raw();
        r.range_cap && r.caps_value != 0
    }).expect("initial position has range-capture moves").clone();
    let (from, to) = (rc.raw().from_sq as usize, rc.raw().to_sq as usize);
    let (fr, fc) = (from / 36, from % 36);
    let (tr, tc) = (to / 36, to % 36);
    let dr = (tr as i32 - fr as i32).signum();
    let dc = (tc as i32 - fc as i32).signum();
    // Count occupied squares strictly between from and to.
    let mut occupied_between: usize = 0;
    let (mut r, mut c) = (fr as i32 + dr, fc as i32 + dc);
    while (r, c) != (tr as i32, tc as i32) {
        if board.get(r as usize, c as usize).is_some() { occupied_between += 1; }
        r += dr; c += dc;
    }
    let landing_occupied = board.get(tr, tc).is_some() as usize;
    let before_total = board.piece_count(Color::Black) + board.piece_count(Color::White);
    board.apply(&rc);
    let after_total = board.piece_count(Color::Black) + board.piece_count(Color::White);
    assert_eq!(before_total - after_total, occupied_between + landing_occupied,
        "range capture must remove ALL occupied path squares (any color)");
    board.undo();
    assert_eq!(board.piece_count(Color::Black) + board.piece_count(Color::White), before_total);
}

// ── Search: returns a legal move and leaves the board untouched ─────
#[test]
fn search_returns_legal_move_and_preserves_state() {
    let mut board = Board::initial();
    let before = board.to_tsfen();
    let r = board.search(2, 0);
    let best = r.best_move.expect("depth-2 search must return a move");
    let legal = board.legal_moves();
    assert!(
        legal.iter().any(|m| m.raw().from_sq == best.raw().from_sq
            && m.raw().to_sq == best.raw().to_sq
            && m.raw().promotion == best.raw().promotion),
        "best move must be legal"
    );
    assert_eq!(board.to_tsfen(), before, "search must not mutate the board");
}

#[test]
fn search_expands_a_real_tree() {
    let mut board = Board::initial();
    let r = board.search(3, 0);
    assert!(r.score.abs() < 1_000_000, "non-mate score must be below MATE_SCORE");
    // With the material fast path DISABLED, depth-3 must genuinely expand
    // the tree (the old fake path produced ~1536 nodes, one per root move).
    assert!(r.nodes > 1_000, "depth-3 must expand a real tree (got {} nodes)", r.nodes);
}

// ============================================================
// New test batch: board API, rules edge cases, search, and Elo
// ============================================================

#[test]
fn side_to_move_alternates_and_move_number_increments() {
    let mut board = Board::initial();
    assert_eq!(board.side_to_move(), Color::Black);
    assert_eq!(board.move_number(), 1);
    let m = board.legal_moves().into_iter().next().unwrap();
    board.apply(&m);
    assert_eq!(board.side_to_move(), Color::White);
    assert_eq!(board.move_number(), 1);
    let m = board.legal_moves().into_iter().next().unwrap();
    board.apply(&m);
    assert_eq!(board.side_to_move(), Color::Black);
    assert_eq!(board.move_number(), 2);
}

#[test]
fn random_move_is_always_legal() {
    let mut board = Board::initial();
    for _ in 0..30 {
        let legal = board.legal_moves();
        let m = board.random_move().expect("startpos walk always has moves");
        assert!(legal.iter().any(|lm| lm.raw().from_sq == m.raw().from_sq
            && lm.raw().to_sq == m.raw().to_sq
            && lm.raw().promotion == m.raw().promotion));
        board.apply(&m);
    }
}

#[test]
fn from_tsfen_rejects_garbage() {
    assert!(Board::from_tsfen("").is_err());
    assert!(Board::from_tsfen("not a tsfen").is_err());
    assert!(Board::from_tsfen("a/b/c b 1").is_err());
    // valid TSFEN parses
    assert!(Board::from_tsfen(&Board::initial().to_tsfen()).is_ok());
}

#[test]
fn clone_is_independent() {
    let mut board = Board::initial();
    let mut clone = board.clone();
    let m = board.legal_moves().into_iter().next().unwrap();
    board.apply(&m);
    assert_eq!(clone.move_number(), 1);
    assert_ne!(board.to_tsfen(), clone.to_tsfen());
    // the clone can continue on its own
    let m2 = clone.legal_moves().into_iter().next().unwrap();
    clone.apply(&m2);
    // one black move played → white to move, move_number unchanged
    assert_eq!(clone.move_number(), 1);
    assert_eq!(clone.side_to_move(), Color::White);
    // clone state is self-consistent: material equals the recomputed sum
    let sum: i32 = clone.pieces(Color::Black).iter().map(|(_, p)| p.value()).sum::<i32>()
        - clone.pieces(Color::White).iter().map(|(_, p)| p.value()).sum::<i32>();
    assert_eq!(clone.material_score(), sum);
}

#[test]
fn material_score_matches_recomputed_sum() {
    let mut rng = Lcg(0xBEEF);
    let mut board = Board::initial();
    for _ in 0..40 {
        let moves = board.legal_moves();
        if moves.is_empty() { break; }
        board.apply(&moves[rng.pick(moves.len())]);
        let sum: i32 = board.pieces(Color::Black).iter().map(|(_, p)| p.value()).sum::<i32>()
            - board.pieces(Color::White).iter().map(|(_, p)| p.value()).sum::<i32>();
        assert_eq!(board.material_score(), sum,
            "incremental material diverged at move {} (tsfen {})",
            board.move_number(), board.to_tsfen());
    }
}

#[test]
fn no_progress_counter_resets_on_capture() {
    let mut board = Board::initial();
    let mut rng = Lcg(0xCAFE);
    for _ in 0..80 {
        let quiet = board.no_progress_plies();
        let b_before = board.piece_count(Color::Black);
        let w_before = board.piece_count(Color::White);
        let moves = board.legal_moves();
        board.apply(&moves[rng.pick(moves.len())]);
        let captured = board.piece_count(Color::Black) != b_before
            || board.piece_count(Color::White) != w_before;
        if captured {
            assert_eq!(board.no_progress_plies(), 0, "capture must reset the counter");
        } else {
            assert_eq!(board.no_progress_plies(), quiet + 1, "quiet move must increment");
        }
    }
}

#[test]
fn piece_type_table_is_complete() {
    // 209 initial types + 92 promoted forms
    assert!(crate::num_piece_types() >= 301);
    // every type has a non-empty abbreviation and a positive material value
    for i in 1..crate::num_piece_types() as u16 {
        assert!(!crate::pieces::abbrev(i).is_empty(), "type {} abbreviation empty", i);
        assert!(crate::pieces::value(i) > 0, "type {} value must be positive", i);
        assert!(!crate::pieces::name(i).is_empty(), "type {} name empty", i);
    }
}

#[test]
fn royal_pieces_exist_in_startpos() {
    let board = Board::initial();
    let mut kings = 0;
    let mut princes = 0;
    for (_, p) in board.pieces(Color::Black).into_iter().chain(board.pieces(Color::White)) {
        if p.is_royal() {
            if p.abbrev() == "K" { kings += 1; } else { princes += 1; }
        }
    }
    assert_eq!(kings, 2, "one king per side");
    assert_eq!(princes, 2, "one crown prince per side");
}

#[test]
fn perft_two_via_manual_expansion() {
    let mut board = Board::initial();
    assert_eq!(perft(&mut board, 1), 512);
    let moves = board.legal_moves();
    let mut sum = 0u64;
    for m in &moves {
        board.apply(m);
        sum += board.legal_moves().len() as u64;
        board.undo();
    }
    assert_eq!(sum, 260_917, "perft(2) via manual expansion");
}

#[test]
fn repeated_search_is_deterministic() {
    let mut b1 = Board::initial();
    let mut b2 = Board::initial();
    let r1 = b1.search(2, 0);
    let r2 = b2.search(2, 0);
    assert_eq!(r1.score, r2.score);
    match (&r1.best_move, &r2.best_move) {
        (Some(m1), Some(m2)) => {
            assert_eq!(m1.raw().from_sq, m2.raw().from_sq);
            assert_eq!(m1.raw().to_sq, m2.raw().to_sq);
        }
        (None, None) => {}
        _ => panic!("search results diverged"),
    }
}

#[test]
fn search_survives_back_to_back_runs_without_tt_corruption() {
    let mut board = Board::initial();
    for _ in 0..3 {
        let r = board.search(3, 0);
        assert!(r.nodes > 0);
        assert!(r.best_move.is_some());
    }
    // state intact after all searches
    assert_eq!(board.piece_count(Color::Black), 402);
    assert_eq!(board.piece_count(Color::White), 402);
}

#[test]
fn undo_to_start_restores_initial_tsfen() {
    let mut board = Board::initial();
    let start = board.to_tsfen();
    let mut rng = Lcg(0x1234);
    let mut applied = 0;
    for _ in 0..25 {
        let ms = board.legal_moves();
        if ms.is_empty() { break; }
        board.apply(&ms[rng.pick(ms.len())]);
        applied += 1;
    }
    for _ in 0..applied {
        assert!(board.undo());
    }
    assert_eq!(board.to_tsfen(), start);
    assert!(!board.undo(), "cannot undo past the start");
}

#[test]
fn elo_expected_and_elo_roundtrip() {
    let e = crate::elo::expected_score(200.0);
    assert!(e > 0.7 && e < 0.8, "+200 Elo ≈ 0.76, got {}", e);
    let back = crate::elo::elo_from_score(e);
    assert!((back - 200.0).abs() < 1e-6);
    // WDL of all draws → exactly 0 Elo
    let w = crate::elo::Wdl { wins: 0, draws: 10, losses: 0 };
    assert!(w.elo().abs() < 1e-9);
    assert!(w.elo_margin95().is_finite());
}

#[test]
fn elo_match_report_math_is_consistent() {
    let w = crate::elo::Wdl { wins: 55, draws: 30, losses: 15 };
    let expected = crate::elo::elo_from_score(w.score());
    assert!((w.elo() - expected).abs() < 1e-9);
    assert!(w.elo() > 0.0);
    let mirror = crate::elo::Wdl { wins: 15, draws: 30, losses: 55 };
    assert!((w.elo() + mirror.elo()).abs() < 1e-9, "mirrored WDLs must negate");
}

#[test]
fn sprt_accepts_a_dominant_engine() {
    let s = crate::elo::Sprt { elo0: 0.0, elo1: 50.0, alpha: 0.05, beta: 0.05 };
    let dominant = crate::elo::Wdl { wins: 80, draws: 15, losses: 5 };
    assert_eq!(s.decision(&dominant), crate::elo::SprtDecision::AcceptH1);
    let terrible = crate::elo::Wdl { wins: 5, draws: 15, losses: 80 };
    assert_eq!(s.decision(&terrible), crate::elo::SprtDecision::AcceptH0);
    let marginal = crate::elo::Wdl { wins: 11, draws: 0, losses: 11 };
    assert_eq!(s.decision(&marginal), crate::elo::SprtDecision::Continue);
}

#[test]
fn selfplay_one_game_terminates_and_counts_plies() {
    let a = crate::elo::EngineConfig::depth(1);
    let o = crate::elo::play_game(&a, &a, 20);
    assert!(o.plies <= 20);
    assert!(o.plies >= 1, "at least one ply must be played before any terminal check");
}
