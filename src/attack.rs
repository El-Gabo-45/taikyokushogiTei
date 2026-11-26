#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::type_complexity
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

//! Bitboard attack generation for Taikyoku Shogi.
//!
//! The core cost in `movegen::generate_pseudo_legal_moves` is walking every
//! piece and, for each, iterating its `Movement` Vecs (heap reads) plus
//! per-jump bounds checks. On a 36×36 board with ~400 pieces that's
//! O(pieces × `movement_vectors`) pointer chases and divisions.
//!
//! This module precomputes, per (`piece_type`, color), a **compact relative
//! attack template** in flat arrays (no heap Vecs):
//! - `jumps[(dr, dc); N]` — non-sliding deltas (jumps, steps, area, igui)
//! - `slides[(dir, max_range); N]` — sliding move specifiers
//!
//! The generation then avoids `pieces::movement()` heap reads for the ~90%
//! of piece types that are simple jumps/steps/slides. Special pieces
//! (hooks, range-capturers, lion mid-captures) still fall back to
//! `pieces::movement` for correctness.
//!
//! Reference: docx §5.3 "Bitboards y Estructuras de Datos Incrementales" —
//! bitboard-based generation scales with the board perimeter, not the area.

use crate::board::Board;
use crate::pieces;
use crate::types::{
    BLACK, BOARD_SIZE, Cell, E, EMPTY_CELL, HookType, INVALID_SQ, Movement, N, NE, NUM_DIRS, NW,
    RayTable, S, SE, SW, W, cell_color, cell_piece, get_deltas, in_promo_zone, ray_table, sq_col,
    sq_row, step_sq,
};
use std::sync::OnceLock;

const MAX_JUMP_DELTAS: usize = 32;
const MAX_SLIDES: usize = 10;

const HOOK_ORTHO_NS: [usize; 2] = [N, S];
const HOOK_ORTHO_EW: [usize; 2] = [E, W];
const HOOK_TURN_NE: [usize; 2] = [NW, SE];
const HOOK_TURN_SE: [usize; 2] = [NE, SW];
const HOOK_TURN_SW: [usize; 2] = [SE, NW];
const HOOK_TURN_NW: [usize; 2] = [NE, SW];

/// Flat movement template per (`piece_type`, color). Only valid for pieces
/// whose moves are pure jumps/steps/slides/area/igui (no hook, no
/// `range_capture`, no lion mid-capture). `valid=false` → must use fallback.
#[derive(Clone, Copy)]
pub struct Template {
    pub jumps: [(i8, i8); MAX_JUMP_DELTAS],
    pub n_jumps: u8,
    pub slides: [(u8, u8); MAX_SLIDES],
    pub n_slides: u8,
    pub has_igui: bool,
    pub area: u8,
    pub valid: bool,
}

const fn empty_template() -> Template {
    Template {
        jumps: [(0, 0); MAX_JUMP_DELTAS],
        n_jumps: 0,
        slides: [(0, 0); MAX_SLIDES],
        n_slides: 0,
        has_igui: false,
        area: 0,
        valid: false,
    }
}

static TEMPLATES: OnceLock<Box<[[Template; 2]]>> = OnceLock::new();

/// Pieces that need the generic `pieces::movement` fallback path (hooks,
/// range-capturers, lion mid-captures) have no precomputed template.
fn is_special_movement(mv: &Movement) -> bool {
    mv.hook.is_some() || !mv.range_capture.is_empty() || (mv.area >= 2 && !mv.jumps.is_empty())
}

/// Append the area-movement deltas (radius 1 or 2) to a template.
fn add_area_jumps(t: &mut Template, mv: &Movement, ti: &mut usize) {
    if mv.area == 0 {
        return;
    }
    for dr in -2i8..=2 {
        for dc in -2i8..=2 {
            if dr == 0 && dc == 0 {
                continue;
            }
            if dr.abs() > 1 && mv.area < 2 {
                continue;
            }
            if dc.abs() > 1 && mv.area < 2 {
                continue;
            }
            if *ti >= MAX_JUMP_DELTAS {
                break;
            }
            t.jumps[*ti] = (dr, dc);
            *ti += 1;
        }
    }
}

/// Build one colour's flat template from a movement descriptor.
fn build_template(mv: &Movement, color: u8) -> Template {
    let mut t = empty_template();
    let flip: i8 = if color == BLACK { 1 } else { -1 };
    let mut ti = 0usize;

    // Jumps.
    for &(jdr, jdc) in &mv.jumps {
        if ti >= MAX_JUMP_DELTAS {
            break;
        }
        t.jumps[ti] = (jdr * flip, jdc * flip);
        ti += 1;
    }

    // Steps (range == 1 slides).
    for &(dir, max_range) in &mv.slides {
        if max_range == 1 && ti < MAX_JUMP_DELTAS {
            let (dr, dc) = get_deltas(dir as usize, color);
            t.jumps[ti] = (dr as i8, dc as i8);
            ti += 1;
        }
    }

    // Area: radius-1 or radius-2 squares (no mid-capture since those are
    // excluded above; pure distance-1/2 jumps).
    add_area_jumps(&mut t, mv, &mut ti);
    t.n_jumps = ti as u8;

    // Slides.
    let mut si = 0usize;
    for &(dir, max_range) in &mv.slides {
        if max_range == 1 {
            continue;
        }
        if si >= MAX_SLIDES {
            break;
        }
        t.slides[si] = (dir, max_range);
        si += 1;
    }
    t.n_slides = si as u8;

    t.has_igui = mv.igui;
    t.area = mv.area;
    t.valid = true;
    t
}

fn build_templates() -> Box<[[Template; 2]]> {
    let mut table: Vec<[Template; 2]> = Vec::with_capacity(512);
    table.push([empty_template(); 2]); // index 0 unused
    for pt in 1..=511u16 {
        let mv = pieces::movement(pt);
        let mut entry = [empty_template(); 2];
        if !is_special_movement(mv) {
            for color in 0..2u8 {
                entry[color as usize] = build_template(mv, color);
            }
        }
        table.push(entry);
    }
    table.into_boxed_slice()
}

pub fn templates() -> &'static [[Template; 2]] {
    TEMPLATES.get_or_init(build_templates)
}

/// Generate tactical (capture / promotion / igui / mid-capture) moves into a
/// caller-owned buffer, in one pass over the moving side's pieces.
///
/// This is the fast bitboard generator used by the search (root and interior
/// nodes): for each piece it walks the flat attack template, checking
/// occupancy for slides and enemy occupancy for jumps. Hooks,
/// range-capturers **and** lion mid-captures are all handled natively here
/// (`gen_hook_captures` / `gen_range_capture_captures` / `gen_lion_captures`),
/// so no caller needs a slow fallback into the generic movegen path.
/// Shared context for generating a piece's tactical captures.
struct CaptureGen<'a> {
    board: &'a Board,
    templates: &'a [[Template; 2]],
    moves: &'a mut Vec<crate::types::Move>,
}

impl CaptureGen<'_> {
    /// Hooks, range-capturers and lion mid-captures use the generic paths.
    fn special(&mut self, sq: usize, pt: u16, color: u8, start: usize) {
        let mv = pieces::movement(pt);
        let rt = ray_table();
        if mv.hook.is_some() {
            gen_hook_captures(self.board, sq, pt, color, mv, rt, self.moves);
        }
        if !mv.range_capture.is_empty() {
            gen_range_capture_captures(self.board, sq, pt, color, mv, rt, self.moves);
        }
        if mv.area >= 2 {
            gen_lion_captures(self.board, sq, pt, color, mv, self.moves);
        }
        crate::movegen::dedup_piece_range(self.moves, start);
    }

    /// Flat template: jumps/steps/area, sliders and igui.
    fn flat(&mut self, sq: usize, pt: u16, color: u8, tmpl: &Template) {
        let sq_r = sq_row(sq) as i32;
        let sq_c = sq_col(sq) as i32;

        // Jumps/steps/area: only capture if target is an enemy piece.
        for &(dr, dc) in tmpl.jumps.iter().take(tmpl.n_jumps as usize) {
            let nr = sq_r + i32::from(dr);
            let nc = sq_c + i32::from(dc);
            if nr < 0 || nr >= BOARD_SIZE as i32 || nc < 0 || nc >= BOARD_SIZE as i32 {
                continue;
            }
            let nsq = (nr as usize) * BOARD_SIZE + (nc as usize);
            let target = self.board.cells[nsq];
            if target != EMPTY_CELL && cell_color(target) != color {
                push_move(self.moves, sq as u16, nsq as u16, pt, color, target);
            }
        }

        // Sliders: walk rays (occupancy-dependent blocking).
        for &(dir, max_range) in tmpl.slides.iter().take(tmpl.n_slides as usize) {
            walk_ray_captures(
                self.board,
                ray_table(),
                sq,
                pt,
                color,
                dir as usize,
                max_range,
                self.moves,
            );
        }

        // Igui (capture in place).
        if tmpl.has_igui {
            for d in 0..NUM_DIRS {
                if let Some(nsq) = step_sq(sq, d, color) {
                    let target = self.board.cells[nsq];
                    if target != EMPTY_CELL && cell_color(target) != color {
                        push_move_igui(self.moves, sq as u16, nsq, pt, color, target);
                    }
                }
            }
        }
    }
}

pub fn generate_captures_bb_into(moves: &mut Vec<crate::types::Move>, board: &Board) {
    moves.clear();
    let color = board.side_to_move;
    let c = color as usize;
    let mut cg = CaptureGen {
        board,
        templates: templates(),
        moves,
    };

    for i in 0..board.piece_list_len[c] {
        let sq = board.piece_list[c][i] as usize;
        if sq == INVALID_SQ as usize {
            continue;
        }
        let cell = board.cells[sq];
        if cell == EMPTY_CELL {
            continue;
        }
        let pt = cell_piece(cell);
        let start = cg.moves.len();
        let tmpl = &cg.templates[(pt as usize).min(511)][color as usize];

        if !tmpl.valid {
            cg.special(sq, pt, color, start);
            continue;
        }
        cg.flat(sq, pt, color, tmpl);
        crate::movegen::dedup_piece_range(cg.moves, start);
    }
}

/// Generate hook-move captures (orthogonal or diagonal hook movers).
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
fn gen_hook_captures(
    board: &Board,
    sq: usize,
    pt: u16,
    color: u8,
    mv: &Movement,
    rt: &RayTable,
    moves: &mut Vec<crate::types::Move>,
) {
    let dirs: &[usize] = match mv.hook {
        Some(HookType::Orthogonal) => &[N, E, S, W],
        Some(HookType::Diagonal) => &[NE, SE, SW, NW],
        None => return,
    };
    for &d in dirs {
        let ray = rt.ray_for_color(sq, d, color);
        for &mid_sq in ray {
            let mid = mid_sq as usize;
            let target = board.cells[mid];
            if target != EMPTY_CELL {
                if cell_color(target) != color {
                    push_move(moves, sq as u16, mid_sq, pt, color, target);
                }
                break;
            }
            let turn_dirs: &[usize] = match mv.hook {
                Some(HookType::Orthogonal) => {
                    if d == N || d == S {
                        &HOOK_ORTHO_NS
                    } else {
                        &HOOK_ORTHO_EW
                    }
                }
                Some(HookType::Diagonal) => match d {
                    NE => &HOOK_TURN_NE,
                    SE => &HOOK_TURN_SE,
                    SW => &HOOK_TURN_SW,
                    NW => &HOOK_TURN_NW,
                    _ => &[],
                },
                None => &[],
            };
            for &td in turn_dirs {
                let turn_ray = rt.ray_for_color(mid, td, color);
                for &tsq in turn_ray {
                    let t = board.cells[tsq as usize];
                    if t == EMPTY_CELL {
                        continue;
                    }
                    if cell_color(t) != color {
                        push_move(moves, sq as u16, tsq, pt, color, t);
                        break;
                    }
                    break;
                }
            }
        }
    }
}

/// Generate range-capture moves (captures pieces of lower rank along rays).
/// `range_cap = true` makes `apply_move` recompute and capture ALL occupied
/// squares between from and to — previously this fast path left intermediate
/// captures unapplied (the old Move had `range_caps`: None here), silently
/// diverging from the generator's intent.
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
fn gen_range_capture_captures(
    board: &Board,
    sq: usize,
    pt: u16,
    color: u8,
    mv: &Movement,
    rt: &RayTable,
    moves: &mut Vec<crate::types::Move>,
) {
    let piece_rank = pieces::rank(pt);
    for &dir in &mv.range_capture {
        let ray = rt.ray_for_color(sq, dir as usize, color);
        let mut caps_value: i32 = 0;
        for &rsq in ray {
            let target = board.cells[rsq as usize];
            if target == EMPTY_CELL {
                continue;
            }
            let t_pt = cell_piece(target);
            let t_rank = pieces::rank(t_pt);
            if t_rank > piece_rank {
                let mut m = crate::types::Move::simple(sq as u16, rsq);
                m.captured_piece = t_pt;
                m.captured_color = cell_color(target);
                m.range_cap = true;
                m.caps_value = caps_value;
                crate::movegen::push_move_raw(moves, m);
                caps_value += pieces::value(t_pt);
            } else {
                break;
            }
        }
    }
}

/// Generate lion area captures — the radius-1 and radius-2 moves, including
/// mid-captures (capture a piece on the way to a second target). This
/// eliminates the slow fallback to movegen for lions.
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
fn gen_lion_captures(
    board: &Board,
    sq: usize,
    pt: u16,
    color: u8,
    mv: &Movement,
    moves: &mut Vec<crate::types::Move>,
) {
    let r = sq_row(sq) as i32;
    let c = sq_col(sq) as i32;

    for d1 in 0..NUM_DIRS {
        let (dr1, dc1) = get_deltas(d1, color);
        let r1 = r + dr1;
        let c1 = c + dc1;
        if r1 < 0 || r1 >= BOARD_SIZE as i32 || c1 < 0 || c1 >= BOARD_SIZE as i32 {
            continue;
        }
        let sq1 = r1 as usize * BOARD_SIZE + c1 as usize;
        let t1 = board.cells[sq1];
        if t1 != EMPTY_CELL && cell_color(t1) == color {
            continue;
        }

        // Direct capture at first step (if enemy).
        if t1 != EMPTY_CELL && cell_color(t1) != color {
            push_move(moves, sq as u16, sq1 as u16, pt, color, t1);
        }

        if mv.area >= 2 {
            for d2 in 0..NUM_DIRS {
                let (dr2, dc2) = get_deltas(d2, color);
                let r2 = r1 + dr2;
                let c2 = c1 + dc2;
                if r2 < 0 || r2 >= BOARD_SIZE as i32 || c2 < 0 || c2 >= BOARD_SIZE as i32 {
                    continue;
                }
                let sq2 = r2 as usize * BOARD_SIZE + c2 as usize;
                if sq2 == sq {
                    continue;
                }
                let t2 = board.cells[sq2];
                if t2 != EMPTY_CELL && cell_color(t2) == color {
                    continue;
                }

                // Mid-capture: capture t1 on the way to sq2.
                if t1 != EMPTY_CELL && cell_color(t1) != color {
                    let mut m = crate::types::Move::simple(sq as u16, sq2 as u16);
                    m.mid_sq = sq1 as u16;
                    m.mid_piece = cell_piece(t1);
                    m.mid_color = cell_color(t1);
                    if t2 != EMPTY_CELL {
                        m.captured_piece = cell_piece(t2);
                        m.captured_color = cell_color(t2);
                    }
                    crate::movegen::push_move_raw(moves, m);
                } else if t2 != EMPTY_CELL && cell_color(t2) != color {
                    // Direct capture at second step (path empty).
                    push_move(moves, sq as u16, sq2 as u16, pt, color, t2);
                }
            }
        }
    }
}

#[inline]
fn can_promote(pt: u16) -> bool {
    pieces::promotes_to(pt).is_some()
}

#[inline]
// Hot movegen path: the eight scalars are passed by value on purpose; a
// parameters struct would add indirection without helping readability.
#[allow(clippy::too_many_arguments)]
fn walk_ray_captures(
    board: &Board,
    rt: &RayTable,
    sq: usize,
    pt: u16,
    color: u8,
    dir: usize,
    max_range: u8,
    moves: &mut Vec<crate::types::Move>,
) {
    let ray = rt.ray_for_color(sq, dir, color);
    let limit = if max_range == 0 {
        ray.len()
    } else {
        (max_range as usize).min(ray.len())
    };
    for &rsq_raw in ray.iter().take(limit) {
        let rsq = rsq_raw as usize;
        let target = board.cells[rsq];
        if target == EMPTY_CELL {
            // Non-capturing promotion: only include if entering promo zone.
            if in_promo_zone(rsq, color) && can_promote(pt) {
                push_move(moves, sq as u16, rsq as u16, pt, color, EMPTY_CELL);
            }
        } else if cell_color(target) != color {
            push_move(moves, sq as u16, rsq as u16, pt, color, target);
            break;
        } else {
            break;
        }
    }
}

/// Fast path for a single non-special piece. Generates jump/step/slide/area/
/// igui moves into `moves` using the flat template `tmpl`.
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
pub fn fast_piece(
    board: &Board,
    sq: usize,
    pt: u16,
    color: u8,
    tmpl: &Template,
    rt: &RayTable,
    moves: &mut Vec<crate::types::Move>,
) {
    let sq_r = sq_row(sq) as i32;
    let sq_c = sq_col(sq) as i32;

    for &(dr, dc) in tmpl.jumps.iter().take(tmpl.n_jumps as usize) {
        let nr = sq_r + i32::from(dr);
        let nc = sq_c + i32::from(dc);
        if nr < 0 || nr >= BOARD_SIZE as i32 || nc < 0 || nc >= BOARD_SIZE as i32 {
            continue;
        }
        let nsq = (nr as usize) * BOARD_SIZE + (nc as usize);
        let target = board.cells[nsq];
        if target == EMPTY_CELL {
            push_move(moves, sq as u16, nsq as u16, pt, color, EMPTY_CELL);
        } else if cell_color(target) != color {
            push_move(moves, sq as u16, nsq as u16, pt, color, target);
        }
    }

    for &(dir, max_range) in tmpl.slides.iter().take(tmpl.n_slides as usize) {
        walk_ray(board, rt, sq, pt, color, dir as usize, max_range, moves);
    }

    if tmpl.has_igui {
        for d in 0..NUM_DIRS {
            if let Some(nsq) = step_sq(sq, d, color) {
                let target = board.cells[nsq];
                if target != EMPTY_CELL && cell_color(target) != color {
                    push_move_igui(moves, sq as u16, nsq, pt, color, target);
                }
            }
        }
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
fn push_move(
    moves: &mut Vec<crate::types::Move>,
    from: u16,
    to: u16,
    pt: u16,
    color: u8,
    target: Cell,
) {
    // Delegate to the shared move builder so promotion variants are
    // generated identically in the fast and slow paths.
    crate::movegen::add_move(moves, from, to, pt, color, target);
}

#[inline]
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
fn push_move_igui(
    moves: &mut Vec<crate::types::Move>,
    from: u16,
    victim_sq: usize,
    pt: u16,
    color: u8,
    target: Cell,
) {
    crate::movegen::add_igui_move(moves, from as usize, victim_sq, pt, color, target);
}

#[inline]
// Hot movegen path: see walk_ray_captures for why the signature stays flat.
#[allow(clippy::too_many_arguments)]
fn walk_ray(
    board: &Board,
    rt: &RayTable,
    sq: usize,
    pt: u16,
    color: u8,
    dir: usize,
    max_range: u8,
    moves: &mut Vec<crate::types::Move>,
) {
    let ray = rt.ray_for_color(sq, dir, color);
    let limit = if max_range == 0 {
        ray.len()
    } else {
        (max_range as usize).min(ray.len())
    };
    for &rsq_raw in ray.iter().take(limit) {
        let rsq = rsq_raw as usize;
        let target = board.cells[rsq];
        if target == EMPTY_CELL {
            push_move(moves, sq as u16, rsq as u16, pt, color, EMPTY_CELL);
        } else if cell_color(target) != color {
            push_move(moves, sq as u16, rsq as u16, pt, color, target);
            break;
        } else {
            break;
        }
    }
}

/// Like [`walk_ray`] but only emits non-capturing (quiet) moves: stops at the
/// first occupied square of either color, never emits captures.
#[inline]
// Hot movegen path: see walk_ray_captures for why the signature stays flat.
#[allow(clippy::too_many_arguments)]
fn walk_ray_quiets(
    board: &Board,
    rt: &RayTable,
    sq: usize,
    pt: u16,
    color: u8,
    dir: usize,
    max_range: u8,
    moves: &mut Vec<crate::types::Move>,
) {
    let ray = rt.ray_for_color(sq, dir, color);
    let limit = if max_range == 0 {
        ray.len()
    } else {
        (max_range as usize).min(ray.len())
    };
    for &rsq_raw in ray.iter().take(limit) {
        let rsq = rsq_raw as usize;
        let target = board.cells[rsq];
        if target == EMPTY_CELL {
            push_move(moves, sq as u16, rsq as u16, pt, color, EMPTY_CELL);
        } else {
            // Occupied square (friendly or enemy) blocks a quiet slide.
            break;
        }
    }
}

/// Fast path for a single non-special piece: quiets only.
///
/// Like [`fast_piece`] but never emits captures — used by the search's quiet
/// stage (Stage 2) so it does not regenerate captures already searched in
/// Stage 1.
#[allow(clippy::too_many_arguments)]
// Hot movegen / search path: a parameters struct would add indirection without
// improving readability, so the flat signature is kept on purpose.
pub fn fast_piece_quiets(
    board: &Board,
    sq: usize,
    pt: u16,
    color: u8,
    tmpl: &Template,
    rt: &RayTable,
    moves: &mut Vec<crate::types::Move>,
) {
    let sq_r = sq_row(sq) as i32;
    let sq_c = sq_col(sq) as i32;

    // Jumps/steps/area: only quiet when target is empty.
    for &(dr, dc) in tmpl.jumps.iter().take(tmpl.n_jumps as usize) {
        let nr = sq_r + i32::from(dr);
        let nc = sq_c + i32::from(dc);
        if nr < 0 || nr >= BOARD_SIZE as i32 || nc < 0 || nc >= BOARD_SIZE as i32 {
            continue;
        }
        let nsq = (nr as usize) * BOARD_SIZE + (nc as usize);
        let target = board.cells[nsq];
        if target == EMPTY_CELL {
            push_move(moves, sq as u16, nsq as u16, pt, color, EMPTY_CELL);
        }
    }

    // Sliders: walk rays, stop at first occupancy.
    for &(dir, max_range) in tmpl.slides.iter().take(tmpl.n_slides as usize) {
        walk_ray_quiets(board, rt, sq, pt, color, dir as usize, max_range, moves);
    }

    // Igui is capture-only: nothing to emit for quiets.
}
