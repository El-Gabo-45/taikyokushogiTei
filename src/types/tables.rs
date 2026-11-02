//! Precomputed lookup tables: ray table, Zobrist hash keys, promotion-zone tables.

use super::constants::*;
use super::square::*;
use std::sync::OnceLock;

// ============================================================================
// Precomputed ray table
// ============================================================================

/// For each (square, direction), the list of squares along that ray.
/// Computed once at init.
pub struct RayTable {
    // rays[sq * 8 + dir] = start index into `data`
    // lens[sq * 8 + dir] = length
    offsets: Vec<u32>,
    lens: Vec<u8>,
    data: Vec<u16>,
}

impl RayTable {
    pub fn new() -> Self {
        let total = NUM_SQUARES * NUM_DIRS;
        let mut offsets = vec![0u32; total];
        let mut lens = vec![0u8; total];
        let mut data = Vec::with_capacity(total * 18); // avg ~18 squares per ray

        for sq in 0..NUM_SQUARES {
            let r = sq_row(sq) as i32;
            let c = sq_col(sq) as i32;
            for dir in 0..NUM_DIRS {
                let idx = sq * NUM_DIRS + dir;
                offsets[idx] = data.len() as u32;
                let dr = DIR_DR[dir];
                let dc = DIR_DC[dir];
                let mut cr = r + dr;
                let mut cc = c + dc;
                let mut count = 0u8;
                while cr >= 0 && cr < BOARD_SIZE as i32 && cc >= 0 && cc < BOARD_SIZE as i32 {
                    data.push((cr as usize * BOARD_SIZE + cc as usize) as u16);
                    count += 1;
                    cr += dr;
                    cc += dc;
                }
                lens[idx] = count;
            }
        }
        RayTable {
            offsets,
            lens,
            data,
        }
    }

    /// Get the ray from `sq` in direction `dir`.
    #[inline]
    pub fn ray(&self, sq: usize, dir: usize) -> &[u16] {
        let idx = sq * NUM_DIRS + dir;
        let start = self.offsets[idx] as usize;
        let len = self.lens[idx] as usize;
        &self.data[start..start + len]
    }

    /// Get ray adjusted for color (Black uses dir directly, White uses opposite).
    #[inline]
    pub fn ray_for_color(&self, sq: usize, dir: usize, color: u8) -> &[u16] {
        if color == BLACK {
            self.ray(sq, dir)
        } else {
            self.ray(sq, (dir + 4) % 8)
        }
    }
}

static RAY_TABLE: OnceLock<RayTable> = OnceLock::new();

/// Get the global ray table (lazily initialized).
pub fn ray_table() -> &'static RayTable {
    RAY_TABLE.get_or_init(RayTable::new)
}

// ============================================================================
// Zobrist hash table
// ============================================================================

struct ZobristTable {
    piece_keys: Vec<[u64; NUM_SQUARES]>,
    side_key: u64,
}

static ZOBRIST_TABLE: OnceLock<ZobristTable> = OnceLock::new();

fn zobrist_table() -> &'static ZobristTable {
    ZOBRIST_TABLE.get_or_init(|| {
        let mut s: u64 = 0xDEAD_BEEF_CAFE_1234;
        let mut rng = || -> u64 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        };
        // 512 slots: pt*2 + color, covers all piece types + promotions for both sides
        let mut piece_keys = vec![[0u64; NUM_SQUARES]; 512];
        for pt in 0..512 {
            for sq in 0..NUM_SQUARES {
                piece_keys[pt][sq] = rng();
            }
        }
        ZobristTable {
            piece_keys,
            side_key: rng(),
        }
    })
}

#[inline]
pub fn zobrist_piece_key(pt: u16, sq: usize, color: u8) -> u64 {
    let idx = (pt as usize * 2 + color as usize).min(511);
    zobrist_table().piece_keys[idx][sq]
}

#[inline]
pub fn zobrist_side_key() -> u64 {
    zobrist_table().side_key
}

// ============================================================================
// Precomputed promotion zone and farthest rank tables
// ============================================================================
//
// Instead of computing `sq / BOARD_SIZE` on every `in_promo_zone()` and
// `is_farthest_rank()` call, we precompute the results as `[bool; 1296]`
// arrays (1.3KB each). This turns each check into a single array index.
//
// Reference: Chess Programming Wiki, "Precomputed Tables" → "Lookup Tables
// for boolean checks". Stockfish uses similar precomputed arrays.

static PROMO_ZONE_TABLES: OnceLock<[[bool; NUM_SQUARES]; 2]> = OnceLock::new();
static FARTHEST_RANK_TABLES: OnceLock<[[bool; NUM_SQUARES]; 2]> = OnceLock::new();

/// Check if a square is in the promotion zone for the given color.
/// Black's promotion zone: rows 0..=10 (opponent's side).
/// White's promotion zone: rows 25..=35 (opponent's side).
#[inline]
pub fn in_promo_zone(sq: usize, color: u8) -> bool {
    PROMO_ZONE_TABLES.get_or_init(|| {
        let mut black = [false; NUM_SQUARES];
        let mut white = [false; NUM_SQUARES];
        for row in 0..PROMO_ZONE_DEPTH {
            for col in 0..BOARD_SIZE {
                black[row * BOARD_SIZE + col] = true;
            }
        }
        for row in (BOARD_SIZE - PROMO_ZONE_DEPTH)..BOARD_SIZE {
            for col in 0..BOARD_SIZE {
                white[row * BOARD_SIZE + col] = true;
            }
        }
        [black, white]
    })[color as usize][sq]
}

/// Check if a square is the farthest rank for the given color.
/// Black: row 0. White: row 35.
#[inline]
pub fn is_farthest_rank(sq: usize, color: u8) -> bool {
    FARTHEST_RANK_TABLES.get_or_init(|| {
        let mut black = [false; NUM_SQUARES];
        let mut white = [false; NUM_SQUARES];
        for col in 0..BOARD_SIZE {
            black[col] = true; // row 0
            white[(BOARD_SIZE - 1) * BOARD_SIZE + col] = true; // row 35
        }
        [black, white]
    })[color as usize][sq]
}
