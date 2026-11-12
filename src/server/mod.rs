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

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, HeaderValue, Method};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::ServeDir;

use crate::{Board, Color, GameResult, Network};

use types::{BoardResponse, CellData, MoveData};

// ============================================================
// State
// ============================================================

/// Configuration for [`AppState`] (env-driven builders fill it from the
/// `TAIKYOKU_*` variables; tests and embedders can set it explicitly).
#[derive(Debug, Clone, Default)]
pub struct AppStateConfig {
    /// Give every session id its own isolated [`GameState`] instead of the
    /// single shared board (`TAIKYOKU_SESSIONS` in the env-driven builders).
    pub sessions: bool,
    /// Maximum number of live session games before the least recently used
    /// one is evicted. `None` = default ([`DEFAULT_MAX_SESSIONS`]).
    pub max_sessions: Option<usize>,
    /// Exact CORS origins to allow. Empty = mirror the request origin (the
    /// historical, permissive behaviour meant for local use).
    pub cors_origins: Vec<String>,
}

/// Default cap on the number of live session games.
pub const DEFAULT_MAX_SESSIONS: usize = 64;

/// Shared application state.
///
/// Two modes:
///
/// - **shared** (default): one [`GameState`] per server process. Fine for
///   local play (default bind `127.0.0.1:8000`), but every visitor shares
///   the same board, move history and undo stack — not a multi-user demo.
/// - **sessions** (`TAIKYOKU_SESSIONS=1` or `taikyokushogi-server
///   --sessions`): clients that present a session id ([`SessionGame`] lists
///   where it can come from) each get their own isolated game. Requests
///   without an id keep using the shared board, and the session cap
///   (`TAIKYOKU_MAX_SESSIONS`, default [`DEFAULT_MAX_SESSIONS`]) evicts the
///   least recently used session games.
pub struct AppState {
    /// The single game in shared mode; the fallback for id-less requests in
    /// sessions mode.
    pub shared: Arc<Mutex<GameState>>,
    /// Per-session game slots, `None` in shared mode.
    sessions: Option<SessionMap>,
    /// Network attached to every game created by this state (survives
    /// `new game`, TSFEN loads and undo within each game).
    network: Option<Arc<Network>>,
    /// Exact CORS origins for [`build_router`] (empty = mirror request).
    pub cors_origins: Vec<String>,
}

impl AppState {
    /// The game a request with the given (optional) session id operates on.
    fn game_for(&self, session: Option<&str>) -> Arc<Mutex<GameState>> {
        match (&self.sessions, session) {
            (Some(map), Some(id)) => map.get_or_create(id, self.network.clone()),
            _ => Arc::clone(&self.shared),
        }
    }
}

/// Per-session game slots, active only in session mode.
struct SessionMap {
    games: Mutex<HashMap<String, SessionEntry>>,
    max: usize,
}

struct SessionEntry {
    game: Arc<Mutex<GameState>>,
    last_used: std::time::Instant,
}

impl SessionMap {
    /// The caller's session game: reused when the id is known, otherwise
    /// created. At capacity the least recently used session is evicted (its
    /// next request simply starts a fresh game).
    fn get_or_create(&self, id: &str, net: Option<Arc<Network>>) -> Arc<Mutex<GameState>> {
        let now = std::time::Instant::now();
        let mut games = self.games.lock().unwrap();
        if let Some(entry) = games.get_mut(id) {
            entry.last_used = now;
            return entry.game.clone();
        }
        if games.len() >= self.max {
            if let Some(oldest) = games
                .iter()
                .min_by_key(|(_, e)| e.last_used)
                .map(|(k, _)| k.clone())
            {
                games.remove(&oldest);
            }
        }
        let game = Arc::new(Mutex::new(GameState::new_with_network(net)));
        games.insert(
            id.to_string(),
            SessionEntry {
                game: game.clone(),
                last_used: now,
            },
        );
        game
    }
}

/// Create the initial [`AppState`] used by [`build_router`] and [`run`]
/// (configuration comes from the `TAIKYOKU_*` environment variables).
pub fn new_state() -> Arc<AppState> {
    new_state_with_network(None)
}

/// [`new_state`] with an NNUE network attached.
///
/// The network is stored in the game state (not just on the initial board) so
/// it survives `new game`, TSFEN loads and undo.
pub fn new_state_with_network(net: Option<Arc<Network>>) -> Arc<AppState> {
    new_state_with_config(
        net,
        AppStateConfig {
            sessions: sessions_enabled(),
            max_sessions: max_sessions(),
            cors_origins: cors_origins_env(),
        },
    )
}

/// [`new_state`] with fully explicit configuration (tests and embedders that
/// must not depend on the process environment).
pub fn new_state_with_config(net: Option<Arc<Network>>, cfg: AppStateConfig) -> Arc<AppState> {
    let sessions = cfg.sessions.then(|| SessionMap {
        games: Mutex::new(HashMap::new()),
        max: cfg.max_sessions.unwrap_or(DEFAULT_MAX_SESSIONS).max(1),
    });
    Arc::new(AppState {
        shared: Arc::new(Mutex::new(GameState::new_with_network(net.clone()))),
        sessions,
        network: net,
        cors_origins: cfg.cors_origins,
    })
}

/// Whether per-session games are requested: `TAIKYOKU_SESSIONS` set to a
/// truthy value (`1`, `true`, `on`, `yes` — case-insensitive).
pub fn sessions_enabled() -> bool {
    match std::env::var("TAIKYOKU_SESSIONS") {
        Ok(v) => matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "on" | "yes"
        ),
        Err(_) => false,
    }
}

fn max_sessions() -> Option<usize> {
    std::env::var("TAIKYOKU_MAX_SESSIONS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
}

fn cors_origins_env() -> Vec<String> {
    std::env::var("TAIKYOKU_CORS_ORIGIN")
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================
// Session routing
// ============================================================

/// Per-request handle to the caller's game: the shared board in shared mode,
/// the caller's own session game in session mode.
///
/// Handlers take this as an extractor. The session id is looked up in the
/// `X-Session-Id` header first, then the `taikyoku_session` cookie, then the
/// `?session=` query parameter; requests without a (valid) id operate on the
/// shared board. Ids are limited to `[A-Za-z0-9_-]{1,64}`.
#[derive(Clone)]
pub struct SessionGame(pub Arc<Mutex<GameState>>);

// axum 0.7 declares `FromRequestParts` through `#[async_trait]`, so the impl
// needs the same attribute (it desugars to a boxed-future signature).
#[async_trait::async_trait]
impl FromRequestParts<Arc<AppState>> for SessionGame {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let id = session_id_from_parts(parts);
        Ok(SessionGame(state.game_for(id.as_deref())))
    }
}

/// Maximum accepted session id length (defensive cap on map keys).
const MAX_SESSION_ID_LEN: usize = 64;

/// Accept only sane session ids; anything else falls back to the shared
/// board (this also rejects URL-encoded query values instead of decoding
/// them — ids are caller-chosen, never generated by the server).
fn sanitize_session_id(raw: &str) -> Option<String> {
    let id = raw.trim();
    let ok = !id.is_empty()
        && id.len() <= MAX_SESSION_ID_LEN
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    ok.then(|| id.to_string())
}

/// Extract the caller's session id, if any: `X-Session-Id` header first,
/// then the `taikyoku_session` cookie, then the `?session=` query parameter.
fn session_id_from_parts(parts: &Parts) -> Option<String> {
    if let Some(v) = parts
        .headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(id) = sanitize_session_id(v) {
            return Some(id);
        }
    }
    if let Some(cookies) = parts
        .headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
    {
        for pair in cookies.split(';') {
            if let Some(val) = pair.trim().strip_prefix("taikyoku_session=") {
                if let Some(id) = sanitize_session_id(val) {
                    return Some(id);
                }
            }
        }
    }
    if let Some(query) = parts.uri.query() {
        for pair in query.split('&') {
            if let Some(val) = pair.strip_prefix("session=") {
                if let Some(id) = sanitize_session_id(val) {
                    return Some(id);
                }
            }
        }
    }
    None
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
    /// Network attached to every board created by this game (if any).
    network: Option<Arc<Network>>,
}

impl GameState {
    pub fn new() -> Self {
        Self::new_with_network(None)
    }

    pub fn new_with_network(net: Option<Arc<Network>>) -> Self {
        let mut board = Board::initial();
        board.set_network(net.clone());
        let score = board.material_score();
        GameState {
            version: 0,
            board,
            move_log: Vec::new(),
            score_history: vec![score],
            half_move: 0,
            game_over: false,
            network: net,
        }
    }

    pub fn reset(&mut self) {
        let version = self.version.wrapping_add(1);
        let network = self.network.clone();
        *self = GameState::new_with_network(network);
        self.version = version;
    }

    /// Replace the board from a TSFEN/load operation. The move log restarts
    /// at the imported position (the TSFEN format has no move history).
    pub fn set_board(&mut self, mut board: Board) {
        board.set_network(self.network.clone());
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
    let cors = cors_layer(&state.cors_origins);

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

/// CORS policy: mirrors the request origin by default (permissive, meant
/// for local use). A non-empty origin list restricts responses to those
/// exact origins instead — recommended together with session mode for
/// public deployments (`TAIKYOKU_CORS_ORIGIN`, comma-separated).
fn cors_layer(origins: &[String]) -> CorsLayer {
    let parsed: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
    let allow_origin = if parsed.is_empty() {
        AllowOrigin::mirror_request()
    } else {
        AllowOrigin::list(parsed)
    };
    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any)
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
/// be overridden with `TAIKYOKU_BIND`. Set `TAIKYOKU_NET` to a `.tkn` or
/// zstd-compressed `.tkn.zst` file to evaluate with NNUE, or to `none` for the
/// hand-crafted evaluator. Set `TAIKYOKU_SESSIONS=1` to give every visitor
/// an isolated game instead of one shared board (see [`AppState`]), with
/// `TAIKYOKU_MAX_SESSIONS` capping the live games and `TAIKYOKU_CORS_ORIGIN`
/// restricting the CORS policy to exact origins.
pub async fn run() {
    let net = match std::env::var("TAIKYOKU_NET")
        .ok()
        .map(|p| p.trim().to_string())
    {
        Some(path) if !path.is_empty() && !matches!(path.as_str(), "none" | "off" | "classic") => {
            match Network::load(&path) {
                Ok(net) => {
                    println!("NNUE network: {path}");
                    Some(Arc::new(net))
                }
                Err(e) => panic!("cannot load NNUE network `{path}`: {e}"),
            }
        }
        _ => None,
    };
    run_with_network(net).await;
}

/// [`run`] with an already-loaded network (useful for embedders and tests).
pub async fn run_with_network(net: Option<Arc<Network>>) {
    let bind_addr = std::env::var("TAIKYOKU_BIND").unwrap_or_else(|_| "127.0.0.1:8000".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {bind_addr}: {e}"));
    println!("Taikyoku Shogi Server listening on http://{bind_addr}");
    if sessions_enabled() {
        println!(
            "Session mode: every visitor gets an isolated game (TAIKYOKU_SESSIONS=1, cap {})",
            max_sessions().unwrap_or(DEFAULT_MAX_SESSIONS)
        );
    }
    axum::serve(listener, build_router(new_state_with_network(net)))
        .await
        .unwrap();
}
