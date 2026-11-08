//! HTTP server + REST API for the Taikyoku Shogi engine.
//!
//! Enabled with the `server` feature. The module is part of the library (not
//! the binary) so the full router can be exercised by integration tests with
//! `tower::ServiceExt::oneshot` — no TCP port required.
//!
//! ```rust,no_run
//! # async fn demo() {
//! let app = taikyokushogi::server::build_router(taikyokushogi::server::new_state());
//! let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();
//! axum::serve(listener, app).await.unwrap();
//! # }
//! ```

pub mod handlers;
pub mod types;

use axum::http::{header, HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::{Board, Color, GameResult};

use types::{BoardResponse, CellData, MoveData};

// ============================================================
// State
// ============================================================

/// Shared application state: one game per server process.
pub struct AppState {
    pub board: Mutex<GameState>,
}

/// Create the initial [`AppState`] used by [`build_router`] and [`run`].
pub fn new_state() -> Arc<AppState> {
    Arc::new(AppState {
        board: Mutex::new(GameState::new()),
    })
}

/// Authoritative game state behind the API mutex.
pub struct GameState {
    /// Incremented on every state change (new, reset, load, move, undo).
    /// Lets the server detect stale positions when an AI search finishes
    /// after another request modified the board.
    pub version: u64,
    pub board: Board,
    pub move_log: Vec<String>,
    pub score_history: Vec<i32>,
    pub half_move: u32,
    pub game_over: bool,
}

impl GameState {
    pub fn new() -> Self {
        let board = Board::initial();
        let score = board.material_score();
        GameState {
            version: 0,
            board,
            move_log: Vec::new(),
            score_history: vec![score],
            half_move: 0,
            game_over: false,
        }
    }

    pub fn reset(&mut self) {
        let version = self.version.wrapping_add(1);
        *self = GameState::new();
        self.version = version;
    }

    /// Replace the board from a TSFEN/load operation. The move log restarts
    /// at the imported position (the TSFEN format has no move history).
    pub fn set_board(&mut self, board: Board) {
        self.board = board;
        self.move_log.clear();
        self.score_history = vec![self.board.material_score()];
        self.half_move = (self.board.move_number() - 1) * 2
            + u32::from(self.board.side_to_move() == Color::White);
        self.game_over = self.board.game_result().is_some();
        self.version = self.version.wrapping_add(1);
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// Helpers shared by handlers
// ============================================================

/// Serialize the board for the client.
pub fn board_to_response(state: &GameState) -> BoardResponse {
    let mut board_data = Vec::with_capacity(crate::BOARD_SIZE);
    for r in 0..crate::BOARD_SIZE {
        let mut row = Vec::with_capacity(crate::BOARD_SIZE);
        for c in 0..crate::BOARD_SIZE {
            match state.board.get(r, c) {
                Some(piece) => row.push(Some(CellData {
                    piece: piece.abbrev().to_string(),
                    color: u8::from(piece.color == Color::White),
                    name: piece.name().to_string(),
                    value: piece.value(),
                    is_royal: piece.is_royal(),
                })),
                None => row.push(None),
            }
        }
        board_data.push(row);
    }

    let result = state.board.game_result().map(|r| match r {
        GameResult::BlackWins => "black_wins".into(),
        GameResult::WhiteWins => "white_wins".into(),
        GameResult::Draw => "draw".into(),
    });

    BoardResponse {
        board: board_data,
        side_to_move: u8::from(state.board.side_to_move() == Color::White),
        move_number: state.board.move_number(),
        black_pieces: state.board.piece_count(Color::Black),
        white_pieces: state.board.piece_count(Color::White),
        game_result: result,
        version: state.version,
        tsfen: state.board.to_tsfen(),
        eval_score: state.board.evaluate(),
        material_score: state.board.material_score(),
        no_progress_plies: state.board.no_progress_plies(),
        mode: "human_vs_ai",
        move_log: state.move_log.clone(),
        score_history: state.score_history.clone(),
        game_over: state.game_over,
    }
}

/// Legal destinations for the piece on `(r, c)`. Empty when the square is
/// empty or holds a piece of the side *not* to move.
pub fn get_moves_for_square(state: &GameState, r: usize, c: usize) -> Vec<MoveData> {
    let cell = state.board.get(r, c);
    match cell {
        Some(piece) if piece.color == state.board.side_to_move() => {}
        _ => return vec![],
    }

    let all_moves = state.board.legal_moves();
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in &all_moves {
        let from = m.from();
        if from.row == r && from.col == c {
            let to = m.to();
            let key = (to.row, to.col, m.is_promotion(), m.is_igui(), m.mid_sq());
            if seen.insert(key) {
                result.push(MoveData {
                    from: None,
                    to: [to.row, to.col],
                    promotion: m.is_promotion(),
                    is_igui: m.is_igui(),
                    captured: m.captured().map(|s| s.to_string()),
                    mid_sq: m.mid_sq(),
                });
            }
        }
    }
    result
}

/// Apply a coordinate move if a matching legal move exists. Returns `false`
/// for coordinates that are out of bounds, empty, or not legal.
pub fn apply_move_to_board(
    state: &mut GameState,
    fr: usize,
    fc: usize,
    tr: usize,
    tc: usize,
    promotion: bool,
    mid_sq: Option<usize>,
) -> bool {
    if fr >= crate::BOARD_SIZE
        || fc >= crate::BOARD_SIZE
        || tr >= crate::BOARD_SIZE
        || tc >= crate::BOARD_SIZE
    {
        return false;
    }

    let piece = match state.board.get(fr, fc) {
        Some(p) => p.abbrev().to_string(),
        None => return false,
    };
    let side = state.board.side_to_move().to_string();

    let from_sq = crate::BOARD_SIZE * fr + fc;
    let to_sq = crate::BOARD_SIZE * tr + tc;

    let legal_moves = state.board.legal_moves();
    for m in &legal_moves {
        let raw = m.raw();
        // For igui moves, `mid_sq` identifies which adjacent victim is
        // captured when from == to. Without it, several igui moves collide on
        // the same coordinates and the wrong one may be applied.
        let mid_match = mid_sq.is_none_or(|ms| raw.mid_sq as usize == ms);
        if raw.from_sq as usize == from_sq
            && raw.to_sq as usize == to_sq
            && m.is_promotion() == promotion
            && mid_match
        {
            state.board.apply(m);
            record_move(state, &side, &piece, fr, fc, tr, tc, promotion);
            if state.board.game_result().is_some() {
                state.game_over = true;
            }
            return true;
        }
    }
    false
}

/// Append a move to the log, refresh the material score history and bump the
/// state version. Must be called exactly once per applied move.
#[allow(clippy::too_many_arguments)] // unmistakable call sites > packing a throwaway struct
pub fn record_move(
    state: &mut GameState,
    side: &str,
    piece: &str,
    fr: usize,
    fc: usize,
    tr: usize,
    tc: usize,
    promotion: bool,
) {
    state.half_move += 1;
    let promo = if promotion { "+" } else { "" };
    state.move_log.push(format!(
        "{}. {}: {} ({},{})-({},{}){}",
        state.half_move, side, piece, fr, fc, tr, tc, promo
    ));
    state.score_history.push(state.board.material_score());
    state.version = state.version.wrapping_add(1);
}

// ============================================================
// Router
// ============================================================

/// Build the complete application router (API + static frontend).
///
/// Public so integration tests and embedders can drive it without binding a
/// TCP socket.
pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        // ── Game state & moves ──
        .route("/api/state", get(handlers::get_state))
        .route("/api/status", get(handlers::get_status))
        .route("/api/history", get(handlers::get_history))
        .route("/api/moves", get(handlers::get_moves))
        .route("/api/legal-moves", get(handlers::get_legal_moves))
        .route("/api/new-game", post(handlers::post_new_game))
        .route("/api/move", post(handlers::post_move))
        .route("/api/ai-move", post(handlers::post_ai_move))
        .route("/api/search", post(handlers::post_search))
        .route("/api/undo", post(handlers::post_undo))
        // ── Engine information ──
        .route("/api/health", get(handlers::get_health))
        .route("/api/engine-info", get(handlers::get_engine_info))
        .route("/api/evaluate", get(handlers::get_evaluate))
        .route("/api/pieces", get(handlers::get_pieces))
        .route("/api/piece-info/:abbrev", get(handlers::get_piece_info))
        // ── Position / persistence ──
        .route(
            "/api/position",
            get(handlers::get_position).post(handlers::post_position),
        )
        .route(
            "/api/save",
            get(handlers::get_save).post(handlers::post_save),
        )
        .route("/api/load", post(handlers::post_load))
        // Unknown /api/* paths get a JSON 404 instead of the SPA HTML.
        .route("/api/*rest", axum::routing::any(handlers::api_not_found))
        // ── Static frontend ──
        // ServeDir handles MIME types and confines serving to `web/dist`, so
        // nested Vite assets resolve and `..` cannot escape the directory.
        .nest_service("/assets", ServeDir::new(dist_dir().join("assets")))
        .fallback(get(serve_frontend))
        .layer(cors)
        .with_state(state)
}

/// Directory holding the built frontend (`TAIKYOKU_DIST` overrides).
pub fn dist_dir() -> std::path::PathBuf {
    std::env::var("TAIKYOKU_DIST")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("web/dist"))
}

/// SPA fallback: serve `index.html` for unknown non-API paths.
async fn serve_frontend() -> impl IntoResponse {
    let index = dist_dir().join("index.html");
    let content = tokio::fs::read_to_string(&index).await.unwrap_or_else(|_| {
        "<html><body><h1>Frontend not built</h1><p>Run: cd web && npm install && npm run build</p></body></html>"
            .to_string()
    });
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/html; charset=utf-8"),
        )],
        content,
    )
}

// ============================================================
// Run
// ============================================================

/// Start the HTTP server. Bind address defaults to `127.0.0.1:8000` and can
/// be overridden with `TAIKYOKU_BIND`.
pub async fn run() {
    let bind_addr = std::env::var("TAIKYOKU_BIND").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {bind_addr}: {e}"));
    println!("Taikyoku Shogi Server listening on http://{bind_addr}");
    axum::serve(listener, build_router(new_state()))
        .await
        .unwrap();
}
