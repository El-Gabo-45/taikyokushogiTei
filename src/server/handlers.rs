//! HTTP route handlers for the Taikyoku Shogi API.
//!
//! Handlers only orchestrate: acquire the lock briefly, dispatch CPU-bound
//! search work to the blocking pool, and serialize snapshots. Shared state
//! helpers live in [`crate::server`].

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::sync::Arc;

use crate::{Board, Color, GameResult, Move as EngineMove};

use crate::server::types::*;
use crate::server::{apply_move_to_board, record_move, AppState};

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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "ok": false, "error": self.message })),
        )
            .into_response()
    }
}

const BOARD: usize = crate::BOARD_SIZE;

/// Clamp client-controlled search limits: unbounded depth/time on a 36×36
/// board with 500+ legal moves is a trivial local DoS.
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
        captured: mv.captured().map(|s| s.to_string()),
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

pub async fn get_state(State(app): State<Arc<AppState>>) -> Json<BoardResponse> {
    let state = app.board.lock().unwrap();
    Json(crate::server::board_to_response(&state))
}

// ============================================================
// GET /api/history and GET /api/status
// ============================================================

pub async fn get_history(State(app): State<Arc<AppState>>) -> Json<HistoryResponse> {
    let state = app.board.lock().unwrap();
    Json(HistoryResponse {
        move_log: state.move_log.clone(),
        half_move: state.half_move,
        game_over: state.game_over,
        score_history: state.score_history.clone(),
    })
}

pub async fn get_status(State(app): State<Arc<AppState>>) -> Json<StatusResponse> {
    let state = app.board.lock().unwrap();
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

pub async fn get_moves(
    State(app): State<Arc<AppState>>,
    Query(params): Query<MoveQuery>,
) -> Result<Json<MoveListResponse>, ApiError> {
    let (r, c) = (params.r, params.c);
    if r >= BOARD || c >= BOARD {
        return Err(ApiError::bad_request(format!(
            "coordinates out of bounds: r={r}, c={c} (expected 0..={})",
            BOARD - 1
        )));
    }
    let state = app.board.lock().unwrap();
    let moves = crate::server::get_moves_for_square(&state, r, c);
    Ok(Json(MoveListResponse {
        moves,
        from: [r, c],
    }))
}

// ============================================================
// GET /api/legal-moves — every legal move of the side to move
// ============================================================

pub async fn get_legal_moves(State(app): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let state = app.board.lock().unwrap();
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

pub async fn get_evaluate(State(app): State<Arc<AppState>>) -> Json<EvalResponse> {
    let state = app.board.lock().unwrap();
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

pub async fn get_engine_info(State(app): State<Arc<AppState>>) -> Json<EngineInfoResponse> {
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
    let all = crate::all_pieces();
    let royal = all.iter().filter(|p| p.is_royal).count();
    let state = app.board.lock().unwrap();
    let using_nnue = state.board.network().is_some();
    Json(EngineInfoResponse {
        name: "taikyokushogi",
        version: crate::VERSION,
        board_size: BOARD,
        num_squares: crate::NUM_SQUARES,
        piece_types: all.len(),
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
        promotes_to: p.promotes_to.map(|s| s.to_string()),
        slide_directions: p.slide_directions,
        jump_destinations: p.jump_destinations,
        has_hook: p.has_hook,
        area_steps: p.area_steps,
        has_range_capture: p.has_range_capture,
        has_igui: p.has_igui,
        is_royal: p.is_royal,
        betza: p.betza.map(|s| s.to_string()),
    }
}

pub async fn get_pieces(State(_app): State<Arc<AppState>>) -> Json<PieceListResponse> {
    let all = crate::all_pieces();
    Json(PieceListResponse {
        count: all.len(),
        pieces: all.iter().map(piece_response).collect(),
    })
}

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

pub async fn get_position(State(app): State<Arc<AppState>>) -> Json<PositionResponse> {
    let state = app.board.lock().unwrap();
    Json(PositionResponse {
        tsfen: state.board.to_tsfen(),
        side_to_move: state.board.side_to_move() as u8,
        move_number: state.board.move_number(),
    })
}

/// `POST /api/position` and `POST /api/load` — install a position from
/// TSFEN. The change counter advances and history restarts from the
/// imported position.
pub async fn post_position(
    State(app): State<Arc<AppState>>,
    Json(body): Json<LoadBody>,
) -> Result<Json<BoardResponse>, ApiError> {
    let mut state = app.board.lock().unwrap();
    let board = Board::from_tsfen(&body.tsfen).map_err(ApiError::bad_request)?;
    state.set_board(board);
    Ok(Json(crate::server::board_to_response(&state)))
}

pub async fn get_save(State(app): State<Arc<AppState>>) -> Json<SaveBody> {
    let state = app.board.lock().unwrap();
    Json(SaveBody {
        ok: true,
        tsfen: state.board.to_tsfen(),
        move_log: state.move_log.clone(),
        version: state.version,
    })
}

/// POST variant of `/api/save` for clients that prefer a single verb set.
pub async fn post_save(State(app): State<Arc<AppState>>) -> Json<SaveBody> {
    get_save(State(app)).await
}

pub async fn post_load(
    State(app): State<Arc<AppState>>,
    Json(body): Json<LoadBody>,
) -> Result<Json<BoardResponse>, ApiError> {
    post_position(State(app), Json(body)).await
}

// ============================================================
// POST /api/new-game
// ============================================================

pub async fn post_new_game(State(app): State<Arc<AppState>>) -> Json<NewGameResponse> {
    let mut state = app.board.lock().unwrap();
    state.reset();
    Json(NewGameResponse {
        ok: true,
        version: state.version,
    })
}

// ============================================================
// POST /api/move — human move by coordinates
// ============================================================

pub async fn post_move(
    State(app): State<Arc<AppState>>,
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

    let mut state = app.board.lock().unwrap();
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
fn snapshot_for_search(app: &AppState) -> Result<(Board, u64), ApiError> {
    let state = app.board.lock().unwrap();
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
async fn execute_search(snapshot: Board, depth: u32, time_limit: u64) -> SearchOutcome {
    tokio::task::spawn_blocking(move || {
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
    })
}

// ============================================================
// POST /api/ai-move — search and apply for the side to move
// ============================================================

pub async fn post_ai_move(
    State(app): State<Arc<AppState>>,
    Json(body): Json<AiMoveBody>,
) -> Result<Json<MoveResponse>, ApiError> {
    let (depth, time_limit) = search_plan(body.depth, body.time_limit);

    let (snapshot, snapshot_version) = snapshot_for_search(&app)?;
    let outcome = execute_search(snapshot, depth, time_limit).await;

    if outcome.no_moves {
        // Distinguish a terminal position from a search that stopped early
        // (e.g. a very short time limit): only the former ends the game.
        let mut state = app.board.lock().unwrap();
        if state.board.legal_moves().is_empty() {
            state.game_over = true;
            return Err(ApiError::conflict("No legal moves"));
        }
        return Ok(failed_move_response(
            "Search produced no move (time limit reached before completing depth 1?)",
        ));
    }

    let mv = match outcome.best_move {
        Some(m) => m,
        None => return Err(ApiError::conflict("Search task failed")),
    };
    let (fr, fc, tr, tc, promotion, mid_sq) = coords_of(&mv);

    let mut state = app.board.lock().unwrap();

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

pub async fn post_search(
    State(app): State<Arc<AppState>>,
    Json(body): Json<SearchBody>,
) -> Result<Json<SearchResponse>, ApiError> {
    let (depth, time_limit) = search_plan(body.depth, body.time_limit);

    let (snapshot, _) = snapshot_for_search(&app)?;
    let tsfen_of_snapshot = snapshot.to_tsfen();

    let outcome = execute_search(snapshot, depth, time_limit).await;

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

pub async fn post_undo(
    State(app): State<Arc<AppState>>,
    body: Option<Json<UndoBody>>,
) -> Json<UndoResponse> {
    let plies = body.and_then(|b| b.plies).unwrap_or(1).clamp(1, 100);

    let mut state = app.board.lock().unwrap();
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
