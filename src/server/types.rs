//! JSON request/response types for the HTTP API.
//!
//! Every mutating endpoint answers with a body that contains `ok`, and
//! errors additionally carry a human-readable `error` string. HTTP status
//! codes are used on top of that (400 = bad request, 404 = unknown piece,
//! 409 = conflicting state) so generic HTTP clients can also make sense of
//! the responses without knowing the domain.

use serde::{Deserialize, Serialize};

// ============================================================
// Responses — /api/state, /api/move, /api/ai-move
// ============================================================

/// One piece on the board, as seen by the client.
#[derive(Serialize)]
pub struct CellData {
    /// Engine abbreviation (ASCII-only).
    pub piece: String,
    /// 0 = Black, 1 = White.
    pub color: u8,
    /// Full English name.
    pub name: String,
    /// Material value.
    pub value: i32,
    /// Capturing this piece ends the game.
    pub is_royal: bool,
}

/// Full board snapshot. Served by `GET /api/state` and embedded in every
/// successful move response, so clients never need a second round-trip.
#[derive(Serialize)]
pub struct BoardResponse {
    /// 36x36 grid (row-major). `null` = empty square.
    pub board: Vec<Vec<Option<CellData>>>,
    /// 0 = Black, 1 = White.
    pub side_to_move: u8,
    /// Full move number (1-based), only counts *both* sides like in chess.
    pub move_number: u32,
    pub black_pieces: usize,
    pub white_pieces: usize,
    pub game_result: Option<String>,
    /// Monotonic state-change counter — lets the client skip re-renders
    /// and detect that a pending AI move targeted a stale position.
    pub version: u64,
    /// Current position in TSFEN (round-trips through `POST /api/position`).
    pub tsfen: String,
    /// Static evaluation from the side to move's perspective (centipawns).
    pub eval_score: i32,
    /// Material balance from Black's perspective.
    pub material_score: i32,
    /// Consecutive plies without capture or promotion (draw rule: 1,000).
    pub no_progress_plies: u32,
    pub mode: &'static str,
    pub move_log: Vec<String>,
    pub score_history: Vec<i32>,
    pub game_over: bool,
}

/// A legal destination for (or a full origin/target pair within) a move.
#[derive(Serialize)]
pub struct MoveData {
    /// `[row, col]` of the moving piece. Always filled; only meaningful in
    /// responses that span multiple origins (`GET /api/legal-moves`,
    /// `POST /api/search`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<[usize; 2]>,
    /// `[row, col]` of the destination square.
    pub to: [usize; 2],
    /// Move promotes the piece (mandatory when the zone rules force it).
    pub promotion: bool,
    /// Line capture that does not move the piece.
    pub is_igui: bool,
    /// Abbreviation of the captured piece, if any.
    pub captured: Option<String>,
    /// Flat square index (row * 36 + col) of the bystander captured by an
    /// igui/range move. Disambiguates several captures with equal from/to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mid_sq: Option<usize>,
}

#[derive(Serialize)]
pub struct MoveListResponse {
    pub moves: Vec<MoveData>,
    pub from: [usize; 2],
}

/// Server-side search statistics, included so the web UI can display
/// engine telemetry (score / nodes / time) without scraping stdout.
#[derive(Serialize, Clone)]
pub struct SearchStats {
    /// Depth limit that was requested.
    pub depth: u32,
    /// Evaluation score from the side to move's perspective.
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u64,
}

#[derive(Serialize)]
pub struct MoveResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub board: Option<BoardResponse>,
    pub last_move: Option<LastMoveData>,
    /// Present in `/api/ai-move` and `/api/search` responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<SearchStats>,
}

#[derive(Serialize)]
pub struct LastMoveData {
    pub from: [usize; 2],
    pub to: [usize; 2],
}

// ============================================================
// Responses — informational endpoints
// ============================================================

#[derive(Serialize)]
pub struct PieceInfoResponse {
    pub abbrev: String,
    pub name: String,
    pub value: i32,
    pub promotes_to: Option<String>,
    pub slide_directions: usize,
    pub jump_destinations: usize,
    pub has_hook: bool,
    pub area_steps: u8,
    pub has_range_capture: bool,
    pub has_igui: bool,
    pub is_royal: bool,
    pub betza: Option<String>,
}

/// Liveness / discovery endpoint. No board state, always available.
#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub name: &'static str,
    pub version: &'static str,
    pub board_size: usize,
    pub piece_types: usize,
}

/// Description of the loaded engine: sizes, piece database, evaluation
/// backend and search features. Powers the "Engine" panel in the web UI.
#[derive(Serialize)]
pub struct EngineInfoResponse {
    pub name: &'static str,
    pub version: &'static str,
    pub board_size: usize,
    pub num_squares: usize,
    pub piece_types: usize,
    /// Royal piece types among the defined piece classes (King…).
    pub royal_piece_types: usize,
    pub black_pieces: usize,
    pub white_pieces: usize,
    pub side_to_move: u8,
    pub move_number: u32,
    pub eval_backend: &'static str,
    pub using_nnue: bool,
    pub hash_mb: usize,
    pub default_threads: usize,
    pub max_threads: usize,
    /// Search techniques compiled in (all on by default).
    pub search_techniques: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct EvalResponse {
    /// Static evaluation from the side to move's perspective.
    pub eval_score: i32,
    /// Material balance from Black's perspective.
    pub material_score: i32,
    pub black_pieces: usize,
    pub white_pieces: usize,
    pub side_to_move: u8,
}

#[derive(Serialize)]
pub struct PositionResponse {
    pub tsfen: String,
    pub side_to_move: u8,
    pub move_number: u32,
}

#[derive(Serialize)]
pub struct SaveBody {
    pub ok: bool,
    pub tsfen: String,
    pub move_log: Vec<String>,
    /// Public server version counter, for change detection on save/load.
    pub version: u64,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    pub move_log: Vec<String>,
    pub half_move: u32,
    pub game_over: bool,
    pub score_history: Vec<i32>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub game_over: bool,
    pub result: Option<String>,
    pub half_move: u32,
    pub move_number: u32,
    pub black_pieces: usize,
    pub white_pieces: usize,
    pub move_log: Vec<String>,
    pub tsfen: String,
    pub version: u64,
}

/// Result of `POST /api/search` — pure analysis, board is never mutated.
#[derive(Serialize)]
pub struct SearchResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_move: Option<MoveData>,
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u64,
    pub depth: u32,
    /// Position the search ran on, for cross-checking.
    pub tsfen: String,
}

// ============================================================
// request bodies
// ============================================================

#[derive(Deserialize)]
pub struct MoveBody {
    pub from: [usize; 2],
    pub to: [usize; 2],
    pub promotion: Option<bool>,
    pub mid_sq: Option<usize>,
}

#[derive(Deserialize)]
pub struct AiMoveBody {
    pub depth: Option<u32>,
    pub time_limit: Option<u64>,
}

/// `POST /api/search` — analysis only, board is never mutated.
#[derive(Deserialize)]
pub struct SearchBody {
    pub depth: Option<u32>,
    pub time_limit: Option<u64>,
}

/// `POST /api/undo` — `plies` defaults to 1.
#[derive(Deserialize)]
pub struct UndoBody {
    pub plies: Option<u32>,
}

#[derive(Deserialize)]
pub struct LoadBody {
    pub tsfen: String,
}

#[derive(Deserialize)]
pub struct MoveQuery {
    pub r: usize,
    pub c: usize,
}

#[derive(Serialize)]
pub struct NewGameResponse {
    pub ok: bool,
    pub version: u64,
}

#[derive(Serialize)]
pub struct UndoResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub board: Option<BoardResponse>,
    pub undone_plies: u32,
}

#[derive(Serialize)]
pub struct PieceListResponse {
    pub count: usize,
    pub pieces: Vec<PieceInfoResponse>,
}
