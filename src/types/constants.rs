//! Board dimensions, colors, direction constants, piece ranks.

/// Board size (36x36).
pub const BOARD_SIZE: usize = 36;

/// Total number of squares (1,296).
pub const NUM_SQUARES: usize = BOARD_SIZE * BOARD_SIZE;

/// Side / player color constants.
pub const BLACK: u8 = 0;
pub const WHITE: u8 = 1;

/// Promotion zone depth (11 rows on the opponent's side of the board).
pub const PROMO_ZONE_DEPTH: usize = 11;

// Directions (from Black's perspective: forward = decreasing row)
pub const N: usize = 0;
pub const NE: usize = 1;
pub const E: usize = 2;
pub const SE: usize = 3;
pub const S: usize = 4;
pub const SW: usize = 5;
pub const W: usize = 6;
pub const NW: usize = 7;
pub const NUM_DIRS: usize = 8;

// Direction deltas for Black (forward = decreasing row)
pub const DIR_DR: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];
pub const DIR_DC: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];

/// Piece rank for range-capture restrictions.
///
/// The engine models five tiers; a piece can range-capture everything in the
/// tiers *below* its own:
/// 1. King, Crown Prince (`RANK_ROYAL`) — can never be range-captured
/// 2. Great General (`RANK_GREAT`)
/// 3. Vice General (`RANK_VICE`)
/// 4. Flying General, Angle General, Fierce Dragon, Flying Crocodile (`RANK_RANGE_CAP`)
/// 5. all remaining pieces (`RANK_NORMAL`)
pub const RANK_ROYAL: u8 = 0;
pub const RANK_GREAT: u8 = 1;
pub const RANK_VICE: u8 = 2;
pub const RANK_RANGE_CAP: u8 = 3;
pub const RANK_NORMAL: u8 = 4;

/// No-progress draw threshold (in plies). If 1000 consecutive plies pass
/// with no capture and no promotion, the game is automatically drawn.
pub const DRAW_PLIES: u32 = 1000;

/// Maximum number of pieces per side (402 initial + promotions/captures).
pub const MAX_PIECES_PER_SIDE: usize = 410;

/// Maximum number of royal pieces tracked per side (King + Crown Prince).
pub const MAX_ROYALS: usize = 8;
