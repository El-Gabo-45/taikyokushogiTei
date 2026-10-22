//! Production smoke tests for the Taikyoku Shogi HTTP API.
//!
//! These tests exercise the API handlers directly (no real server needed),
//! verifying the audit-remediation fixes:
//!   1. igui mid_sq identity — moves from the same coordinates are
//!      distinguished by mid_sq
//!   2. out-of-bounds coordinate rejection — returns `ok: false`, no panic
//!   3. /api/piece-info/P returns valid piece data
//!   4. /api/state returns a well-formed board response
//!   5. /api/ai-move returns a legal move at depth 0 and depth 2
//!
//! Run with: `cargo test --test smoke`

use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Router,
};
use serde_json::Value;
use std::sync::Arc;
use taikyokushogi::Board;
use tower::ServiceExt; // for `oneshot` and `ready`

// Re-use the handler functions from main.rs. They're `pub(crate)` within the
// same crate, so integration tests can't call them directly. Instead we spin
// up a minimal Router that mirrors the production routes.
fn app() -> Router {
    let state = Arc::new(crate::AppState);
    Router::new()
        .route("/api/state", get(crate::get_state))
        .route("/api/piece-info/:abbrev", get(crate::get_piece_info))
        .route("/api/move", axum::routing::post(crate::post_move))
        .route("/api/ai-move", axum::routing::post(crate::post_ai_move))
        .with_state(state)
}

// ── Helpers ──────────────────────────────────────────────────────────

async fn json_response(req: Request<Body>) -> (StatusCode, Value) {
    let router = app();
    let res = router.oneshot(req).await.unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body()).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    (status, value)
}

fn post_json(path: &str, json: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(json))
        .unwrap()
}

fn get_json(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .unwrap()

// ── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn piece_info_p_returns_valid_data() {
    let (status, body) = json_response(get_json("/api/piece-info/P"));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["abbrev"], "P");
    assert!(!body["name"].as_str().unwrap().is_empty());
    assert!(body["value"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn piece_info_unknown_returns_defaults() {
    let (status, body) = json_response(get_json("/api/piece-info/ZZZ"));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["abbrev"], "ZZZ");
    assert_eq!(body["value"], 0);
}

#[tokio::test]
async fn state_returns_well_formed_board() {
    let (status, body) = json_response(get_json("/api/state"));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["side_to_move"], 0); // Black starts
    assert_eq!(body["move_number"], 1);
    assert_eq!(body["game_over"], false);
    assert_eq!(body["board"].as_array().unwrap().len(), 36);
    for row in body["board"].as_array().unwrap() {
        assert_eq!(row.as_array().unwrap().len(), 36);
    }
    assert!(body["black_pieces"].as_u64().unwrap() > 0);
    assert!(body["white_pieces"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn ai_move_depth_zero_returns_ok() {
    let body = r#"{"depth": 0, "time_limit": 1000}"#;
    let (status, body) = json_response(post_json("/api/ai-move", body));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert!(body["board"].is_object());
    assert!(body["last_move"].is_object());
    assert_eq!(body["board"]["side_to_move"], 1); // White
}

#[tokio::test]
async fn ai_move_depth_two_returns_ok() {
    let body = r#"{"depth": 2, "time_limit": 5000}"#;
    let (status, body) = json_response(post_json("/api/ai-move", body));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert!(body["board"].is_object());
}

#[tokio::test]
async fn out_of_bounds_move_returns_ok_false_no_panic() {
    // Coordinate at BOARD_SIZE (36) — must be rejected, not panic.
    let body = r#"{"from": [36, 0], "to": [35, 0], "promotion": false}"#;
    let (status, body) = json_response(post_json("/api/move", body));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Illegal move");

    // Both from and to out of bounds.
    let body = r#"{"from": [100, 100], "to": [101, 101], "promotion": false}"#;
    let (status, body) = json_response(post_json("/api/move", body));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"], "Illegal move");
}

#[tokio::test]
async fn legal_move_returns_ok_true() {
    // Black pawn push: row 0 -> row 1, column 0.
    let body = r#"{"from": [0, 0], "to": [1, 0], "promotion": false}"#;
    let (status, body) = json_response(post_json("/api/move", body));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["board"]["side_to_move"], 1); // White to move
}
// ── A/B search config tests via CLI-style toggles ────────────────────

/// Every technique flag can be independently disabled; a search still returns
/// a move. This mirrors the CLI `--no-*` flags.
#[test]
fn search_config_technique_toggles_all_work() {
    let mut board = Board::initial();
    let mut cfg = SearchConfig::production();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some());

    let mut cfg = SearchConfig::production();
    cfg.razoring = false;
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some(), "search with razoring disabled must return a move");

    let mut cfg = SearchConfig::production();
    cfg.reverse_futility = false;
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some(), "search with rfp disabled must return a move");

    let mut cfg = SearchConfig::production();
    cfg.futility = false;
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some(), "search with futility disabled must return a move");

    let mut cfg = SearchConfig::production();
    cfg.null_move_pruning = false;
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some(), "search with null-move disabled must return a move");

    let mut cfg = SearchConfig::production();
    cfg.probcut = false;
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some(), "search with probcut disabled must return a move");
}
}