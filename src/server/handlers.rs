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

//! HTTP route handlers for the Taikyoku Shogi API.
//!
//! Handlers only orchestrate: acquire the lock briefly, dispatch CPU-bound
//! search work to the blocking pool, and serialize snapshots. Shared state
//! helpers live in [`crate::server`].

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::{Arc, Mutex, OnceLock};

use crate::{Board, Color, GameResult, Move as EngineMove};

use crate::server::types::{
    AiMoveBody, BoardResponse, EngineInfoResponse, EvalResponse, HealthResponse, HistoryResponse,
    LastMoveData, LoadBody, MoveBody, MoveData, MoveListResponse, MoveQuery, MoveResponse,
    NewGameResponse, PieceInfoResponse, PieceListResponse, PositionResponse, SaveBody, SearchBody,
    SearchResponse, SearchStats, StatusResponse, UndoBody, UndoResponse,
};
use crate::server::{GameState as ServerGameState, SessionGame, apply_move_to_board, record_move};

// ============================================================
// Error helper — maps to HTTP status codes with a JSON body
// ============================================================

/// API error carrying an HTTP status and a JSON `{ok:false, error}` body.
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::CONFLICT,
            message: msg.into(),
        }
    }
    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        ApiError {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: msg.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry = self.status == StatusCode::TOO_MANY_REQUESTS;
        let mut res = (
            self.status,
            Json(json!({ "ok": false, "error": self.message })),
        )
            .into_response();
        if retry {
            // The search permit wait is SEARCH_PERMIT_WAIT_MS (2 s); tell the
            // client when it is worth trying again instead of guessing.
            res.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_static("2"),
            );
        }
        res
    }
}

const BOARD: usize = crate::BOARD_SIZE;

/// Clamp client-controlled search limits: unbounded depth/time on a 36×36
/// board with 500+ legal moves is a trivial local `DoS`.
const MAX_DEPTH: u32 = 8;
const MAX_TIME_MS: u64 = 120_000;
const DEFAULT_TIME_MS: u64 = 30_000;
/// Random move marker (mirrors the CLI): depth 0 = pick uniformly at random.
const RANDOM_DEPTH: u32 = 0;

// ============================================================
// Small helpers
// ============================================================

fn coords_of(m: &EngineMove) -> (usize, usize, usize, usize, bool, Option<usize>) {
    let (from, to) = (m.from(), m.to());
    (
        from.row,
        from.col,
        to.row,
        to.col,
        m.is_promotion(),
        m.mid_sq(),
    )
}

fn move_to_data(mv: &EngineMove, include_from: bool) -> MoveData {
    MoveData {
        from: include_from.then(|| {
            let f = mv.from();
            [f.row, f.col]
        }),
        to: {
            let t = mv.to();
            [t.row, t.col]
        },
        promotion: mv.is_promotion(),
        is_igui: mv.is_igui(),
        captured: mv.captured().map(std::string::ToString::to_string),
        mid_sq: mv.mid_sq(),
    }
}

fn game_result_string(board: &Board) -> Option<String> {
    board.game_result().map(|r| match r {
        GameResult::BlackWins => "black_wins".to_string(),
        GameResult::WhiteWins => "white_wins".to_string(),
        GameResult::Draw => "draw".to_string(),
    })
}

fn failed_move_response(msg: impl Into<String>) -> Json<MoveResponse> {
    Json(MoveResponse {
        ok: false,
        error: Some(msg.into()),
        board: None,
        last_move: None,
        search: None,
    })
}

// ============================================================
// GET /api/health — liveness / client discovery
// ============================================================

pub async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        name: "taikyokushogi",
        version: crate::VERSION,
        board_size: BOARD,
        piece_types: crate::num_piece_types(),
    })
}

// ============================================================
// GET /api/state — full board snapshot
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_state(SessionGame(game): SessionGame) -> Json<BoardResponse> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(crate::server::board_to_response(&state))
}

// ============================================================
// GET /api/history and GET /api/status
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_history(SessionGame(game): SessionGame) -> Json<HistoryResponse> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(HistoryResponse {
        move_log: state.move_log.clone(),
        half_move: state.half_move,
        game_over: state.game_over,
        score_history: state.score_history.clone(),
    })
}

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_status(SessionGame(game): SessionGame) -> Json<StatusResponse> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(StatusResponse {
        game_over: state.game_over,
        result: game_result_string(&state.board),
        half_move: state.half_move,
        move_number: state.board.move_number(),
        black_pieces: state.board.piece_count(Color::Black),
        white_pieces: state.board.piece_count(Color::White),
        move_log: state.move_log.clone(),
        tsfen: state.board.to_tsfen(),
        version: state.version,
    })
}

// ============================================================
// GET /api/moves?r=&c= — legal destinations for one piece
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn get_moves(
    SessionGame(game): SessionGame,
    Query(params): Query<MoveQuery>,
) -> Result<Json<MoveListResponse>, ApiError> {
    let (r, c) = (params.r, params.c);
    if r >= BOARD || c >= BOARD {
        return Err(ApiError::bad_request(format!(
            "coordinates out of bounds: r={r}, c={c} (expected 0..={})",
            BOARD - 1
        )));
    }
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    let moves = crate::server::get_moves_for_square(&state, r, c);
    Ok(Json(MoveListResponse {
        moves,
        from: [r, c],
    }))
}

// ============================================================
// GET /api/legal-moves — every legal move of the side to move
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_legal_moves(SessionGame(game): SessionGame) -> Json<serde_json::Value> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    let stm = state.board.side_to_move() as u8;
    let moves: Vec<MoveData> = if state.game_over {
        Vec::new()
    } else {
        let all = state.board.legal_moves();
        let mut seen = std::collections::HashSet::new();
        all.iter()
            .filter(|m| {
                let raw = m.raw();
                seen.insert((
                    raw.from_sq,
                    raw.to_sq,
                    raw.promotion,
                    raw.is_igui,
                    raw.mid_sq,
                ))
            })
            .map(|m| move_to_data(m, true))
            .collect()
    };
    Json(json!({
        "moves": moves,
        "count": moves.len(),
        "side_to_move": stm,
        "game_over": state.game_over,
    }))
}

// ============================================================
// GET /api/evaluate — static evaluation + material balance
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_evaluate(SessionGame(game): SessionGame) -> Json<EvalResponse> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(EvalResponse {
        eval_score: state.board.evaluate(),
        material_score: state.board.material_score(),
        black_pieces: state.board.piece_count(Color::Black),
        white_pieces: state.board.piece_count(Color::White),
        side_to_move: state.board.side_to_move() as u8,
    })
}

// ============================================================
// GET /api/engine-info — engine metadata for the web UI
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_engine_info(SessionGame(game): SessionGame) -> Json<EngineInfoResponse> {
    const TECHNIQUES: &[&str] = &[
        "razoring",
        "reverse_futility",
        "futility",
        "null_move_pruning",
        "static_eval_prune",
        "transposition_table",
        "killer_moves",
        "history_heuristic",
        "counter_moves",
        "internal_iterative_deepening",
        "late_move_reductions",
        "late_move_pruning",
        "capture_beam",
        "hash_move_extension",
        "quiescence",
    ];
    let list = piece_list();
    let royal = list.pieces.iter().filter(|p| p.is_royal).count();
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    let using_nnue = state.board.network().is_some();
    Json(EngineInfoResponse {
        name: "taikyokushogi",
        version: crate::VERSION,
        board_size: BOARD,
        num_squares: crate::NUM_SQUARES,
        piece_types: list.count,
        royal_piece_types: royal,
        black_pieces: state.board.piece_count(Color::Black),
        white_pieces: state.board.piece_count(Color::White),
        side_to_move: state.board.side_to_move() as u8,
        move_number: state.board.move_number(),
        eval_backend: if using_nnue { "nnue" } else { "hand-crafted" },
        using_nnue,
        hash_mb: crate::hash_mb(),
        default_threads: crate::search::DEFAULT_THREADS,
        max_threads: crate::search::MAX_THREADS,
        search_techniques: TECHNIQUES.to_vec(),
    })
}

// ============================================================
// Piece database: GET /api/pieces, GET /api/piece-info/:abbrev
// ============================================================

fn piece_response(p: &crate::PieceInfo) -> PieceInfoResponse {
    PieceInfoResponse {
        abbrev: p.abbrev.to_string(),
        name: p.name.to_string(),
        value: p.value,
        promotes_to: p.promotes_to.map(std::string::ToString::to_string),
        slide_directions: p.slide_directions,
        jump_destinations: p.jump_destinations,
        has_hook: p.flags.has_hook(),
        area_steps: p.area_steps,
        has_range_capture: p.flags.has_range_capture(),
        has_igui: p.flags.has_igui(),
        is_royal: p.flags.is_royal(),
        betza: p.betza.map(std::string::ToString::to_string),
    }
}

/// The piece database is immutable for the life of the process. Building the
/// response clones ~1000 strings, so cache it once instead of doing that work
/// on every UI load/refresh.
static PIECES_CACHE: OnceLock<PieceListResponse> = OnceLock::new();

fn piece_list() -> &'static PieceListResponse {
    PIECES_CACHE.get_or_init(|| {
        let all = crate::all_pieces();
        PieceListResponse {
            count: all.len(),
            pieces: all.iter().map(piece_response).collect(),
        }
    })
}

pub async fn get_pieces() -> Json<PieceListResponse> {
    Json(piece_list().clone())
}

/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn get_piece_info(
    Path(abbrev): Path<String>,
) -> Result<Json<PieceInfoResponse>, ApiError> {
    match crate::piece_info(&abbrev) {
        Some(info) => Ok(Json(piece_response(&info))),
        None => Err(ApiError::not_found(format!(
            "unknown piece abbrev: {abbrev}"
        ))),
    }
}

/// Fallback for unknown `/api/*` paths: JSON 404, never the SPA HTML.
pub async fn api_not_found() -> ApiError {
    ApiError::not_found("unknown API endpoint")
}

// ============================================================
// TSFEN: GET/POST /api/position, GET/POST /api/save, POST /api/load
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_position(SessionGame(game): SessionGame) -> Json<PositionResponse> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(PositionResponse {
        tsfen: state.board.to_tsfen(),
        side_to_move: state.board.side_to_move() as u8,
        move_number: state.board.move_number(),
    })
}

/// `POST /api/position` and `POST /api/load` — install a position from
/// TSFEN. The change counter advances and history restarts from the
/// imported position.
/// # Panics
///
/// Panics if the game mutex is poisoned.
/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn post_position(
    SessionGame(game): SessionGame,
    Json(body): Json<LoadBody>,
) -> Result<Json<BoardResponse>, ApiError> {
    let mut state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    let board = Board::from_tsfen(&body.tsfen).map_err(ApiError::bad_request)?;
    state.set_board(board);
    Ok(Json(crate::server::board_to_response(&state)))
}

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn get_save(SessionGame(game): SessionGame) -> Json<SaveBody> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    Json(SaveBody {
        ok: true,
        tsfen: state.board.to_tsfen(),
        move_log: state.move_log.clone(),
        version: state.version,
    })
}

/// POST variant of `/api/save` for clients that prefer a single verb set.
pub async fn post_save(sess: SessionGame) -> Json<SaveBody> {
    get_save(sess).await
}

/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn post_load(
    sess: SessionGame,
    Json(body): Json<LoadBody>,
) -> Result<Json<BoardResponse>, ApiError> {
    post_position(sess, Json(body)).await
}

// ============================================================
// POST /api/new-game
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn post_new_game(SessionGame(game): SessionGame) -> Json<NewGameResponse> {
    let mut state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    state.reset();
    Json(NewGameResponse {
        ok: true,
        version: state.version,
    })
}

// ============================================================
// POST /api/move — human move by coordinates
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn post_move(
    SessionGame(game): SessionGame,
    Json(body): Json<MoveBody>,
) -> Result<Json<MoveResponse>, ApiError> {
    let (fr, fc, tr, tc) = (body.from[0], body.from[1], body.to[0], body.to[1]);
    let coords = [
        ("from row", fr),
        ("from col", fc),
        ("to row", tr),
        ("to col", tc),
    ];
    for (which, v) in coords {
        if v >= BOARD {
            return Err(ApiError::bad_request(format!(
                "{which} out of bounds: {v} (expected 0..={})",
                BOARD - 1
            )));
        }
    }
    // `mid_sq` is a flat square index, not a row/col. Reject nonsense early
    // with a 400 instead of a misleading "Illegal move".
    if let Some(ms) = body.mid_sq {
        if ms >= crate::NUM_SQUARES {
            return Err(ApiError::bad_request(format!(
                "mid_sq out of bounds: {ms} (expected 0..={})",
                crate::NUM_SQUARES - 1
            )));
        }
    }

    let mut state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    if state.game_over {
        return Err(ApiError::conflict("Game is over"));
    }

    let ok = apply_move_to_board(
        &mut state,
        fr,
        fc,
        tr,
        tc,
        body.promotion.unwrap_or(false),
        body.mid_sq,
    );
    if !ok {
        return Ok(failed_move_response("Illegal move"));
    }

    let resp = crate::server::board_to_response(&state);
    Ok(Json(MoveResponse {
        ok: true,
        error: None,
        board: Some(resp),
        last_move: Some(LastMoveData {
            from: body.from,
            to: body.to,
        }),
        search: None,
    }))
}

// ============================================================
// Search plumbing shared by /api/ai-move and /api/search
// ============================================================

/// Snapshot (board clone + counter) taken under the app lock and released
/// before the CPU-bound search, so the async runtime keeps serving the
/// other endpoints. The counter lets callers detect a stale position.
fn snapshot_for_search(game: &Arc<Mutex<ServerGameState>>) -> Result<(Board, u64), ApiError> {
    let state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    if state.game_over {
        return Err(ApiError::conflict("Game is over"));
    }
    Ok((state.board.clone(), state.version))
}

/// Per-call depth/time semantics (finite, clamped) and the resulting
/// search telemetry, shared by ai-move (applies) and search (analysis).
fn search_plan(body_depth: Option<u32>, body_time: Option<u64>) -> (u32, u64) {
    (
        body_depth.unwrap_or(2).min(MAX_DEPTH),
        body_time.unwrap_or(DEFAULT_TIME_MS).min(MAX_TIME_MS),
    )
}

/// What a search produced: the chosen move (if any) plus engine telemetry.
struct SearchOutcome {
    best_move: Option<EngineMove>,
    score: i32,
    nodes: u64,
    time_ms: u64,
    no_moves: bool,
}

/// Run the search on the blocking pool. `depth == RANDOM_DEPTH` picks a
/// uniformly random legal move instead of running alphabeta.
///
/// A shared semaphore caps how many searches run at once. Callers wait up to
/// `SEARCH_PERMIT_WAIT_MS` for a slot, then get `429` rather than piling up
/// CPU-bound work that would starve the async executor.
async fn execute_search(
    snapshot: Board,
    depth: u32,
    time_limit: u64,
    permits: std::sync::Arc<tokio::sync::Semaphore>,
) -> Result<SearchOutcome, ApiError> {
    let permit = tokio::time::timeout(
        std::time::Duration::from_millis(SEARCH_PERMIT_WAIT_MS),
        permits.acquire_owned(),
    )
    .await
    .map_err(|_| ApiError::too_many_requests("search queue full — retry shortly"))?
    .map_err(|_| ApiError::too_many_requests("search service unavailable"))?;

    Ok(tokio::task::spawn_blocking(move || {
        // Keep the permit alive for the whole search.
        let _permit = permit;
        let started = std::time::Instant::now();
        let mut board = snapshot;
        if depth == RANDOM_DEPTH {
            let mv = board.random_move();
            SearchOutcome {
                best_move: mv,
                score: 0,
                nodes: 0,
                time_ms: started.elapsed().as_millis() as u64,
                no_moves: mv.is_none(),
            }
        } else {
            let r = board.search(depth, time_limit);
            SearchOutcome {
                best_move: r.best_move,
                score: r.score,
                nodes: r.nodes,
                time_ms: r.time_ms,
                no_moves: r.best_move.is_none(),
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        eprintln!("[api] search join error: {e}");
        SearchOutcome {
            best_move: None,
            score: 0,
            nodes: 0,
            time_ms: 0,
            no_moves: false,
        }
    }))
}

/// How long a caller waits for a search permit before getting a 429.
const SEARCH_PERMIT_WAIT_MS: u64 = 2_000;

// ============================================================
// POST /api/ai-move — search and apply for the side to move
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn post_ai_move(
    State(app): State<Arc<crate::server::AppState>>,
    SessionGame(game): SessionGame,
    Json(body): Json<AiMoveBody>,
) -> Result<Json<MoveResponse>, ApiError> {
    let (depth, time_limit) = search_plan(body.depth, body.time_limit);

    let (snapshot, snapshot_version) = snapshot_for_search(&game)?;
    let outcome = execute_search(snapshot, depth, time_limit, app.search_permits.clone()).await?;

    if outcome.no_moves {
        // Distinguish a terminal position from a search that stopped early
        // (e.g. a very short time limit): only the former ends the game.
        let mut state = game
            .lock()
            .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
        if state.board.legal_moves().is_empty() {
            state.game_over = true;
            return Err(ApiError::conflict("No legal moves"));
        }
        return Ok(failed_move_response(
            "Search produced no move (time limit reached before completing depth 1?)",
        ));
    }

    let Some(mv) = outcome.best_move else {
        return Err(ApiError::conflict("Search task failed"));
    };
    let (fr, fc, tr, tc, promotion, mid_sq) = coords_of(&mv);

    let mut state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));

    // ── CONCURRENCY GUARD ──
    // If another request changed the board while the AI was thinking
    // (new game, undo, load, another move), the snapshot is stale: discard
    // the move instead of applying something invalid.
    if state.version != snapshot_version {
        return Ok(failed_move_response(
            "Board changed while computing AI move",
        ));
    }
    if state.game_over {
        return Ok(failed_move_response("Game is over"));
    }
    // Sanity check: the AI's move must belong to the side to move.
    let stm = state.board.side_to_move();
    let piece_at_from = state.board.get(fr, fc);
    if !piece_at_from.is_some_and(|p| p.color == stm) {
        return Ok(failed_move_response("AI move is not for the side to move"));
    }

    let piece = piece_at_from.map_or("?".into(), |p| p.abbrev().to_string());
    let side = stm.to_string();

    if let Err(e) = state
        .board
        .apply_by_coord(fr, fc, tr, tc, promotion, mid_sq)
    {
        return Ok(failed_move_response(format!(
            "AI move could not be applied: {e}"
        )));
    }
    record_move(&mut state, &side, &piece, fr, fc, tr, tc, promotion);
    if state.board.game_result().is_some() {
        state.game_over = true;
    }

    let resp = crate::server::board_to_response(&state);
    Ok(Json(MoveResponse {
        ok: true,
        error: None,
        board: Some(resp),
        last_move: Some(LastMoveData {
            from: [fr, fc],
            to: [tr, tc],
        }),
        search: Some(SearchStats {
            depth,
            score: outcome.score,
            nodes: outcome.nodes,
            time_ms: outcome.time_ms,
        }),
    }))
}

// ============================================================
// POST /api/search — pure analysis, board is never mutated
// ============================================================

/// # Errors
///
/// Returns an error for invalid input or conflicting game state.
pub async fn post_search(
    State(app): State<Arc<crate::server::AppState>>,
    SessionGame(game): SessionGame,
    Json(body): Json<SearchBody>,
) -> Result<Json<SearchResponse>, ApiError> {
    let (depth, time_limit) = search_plan(body.depth, body.time_limit);

    let (snapshot, _) = snapshot_for_search(&game)?;
    let tsfen_of_snapshot = snapshot.to_tsfen();

    let outcome = execute_search(snapshot, depth, time_limit, app.search_permits.clone()).await?;

    Ok(Json(SearchResponse {
        ok: outcome.best_move.is_some(),
        best_move: outcome.best_move.as_ref().map(|m| move_to_data(m, true)),
        score: outcome.score,
        nodes: outcome.nodes,
        time_ms: outcome.time_ms,
        depth,
        tsfen: tsfen_of_snapshot,
    }))
}

// ============================================================
// POST /api/undo — revert N plies (default 1)
// ============================================================

/// # Panics
///
/// Panics if the game mutex is poisoned.
pub async fn post_undo(
    SessionGame(game): SessionGame,
    body: Option<Json<UndoBody>>,
) -> Json<UndoResponse> {
    let plies = body.and_then(|b| b.plies).unwrap_or(1).clamp(1, 100);

    let mut state = game
        .lock()
        .unwrap_or_else(|e| panic!("game mutex should not be poisoned: {e}"));
    let mut undone = 0u32;
    while undone < plies {
        if !state.board.undo() {
            break;
        }
        state.move_log.pop();
        if state.score_history.len() > 1 {
            state.score_history.pop();
        }
        if state.half_move > 0 {
            state.half_move -= 1;
        }
        state.version += 1;
        state.game_over = state.board.game_result().is_some();
        undone += 1;
    }

    if undone == 0 {
        return Json(UndoResponse {
            ok: false,
            error: Some("Nothing to undo".into()),
            board: Some(crate::server::board_to_response(&state)),
            undone_plies: 0,
        });
    }
    Json(UndoResponse {
        ok: true,
        error: None,
        board: Some(crate::server::board_to_response(&state)),
        undone_plies: undone,
    })
}
