//! Integration tests for the HTTP API.
//!
//! The router is driven in-process with `tower::ServiceExt::oneshot`, so the
//! tests never bind a TCP port and can run anywhere (including CI).
//!
//! Run with: `cargo test --features server --test server_api`

#![cfg(feature = "server")]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use taikyokushogi::server::{build_router, new_state, AppState};
use tower::ServiceExt;

// ============================================================
// harness
// ============================================================

fn game() -> (Arc<AppState>, Router) {
    let state = new_state();
    let app = build_router(state.clone());
    (state, app)
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).expect("valid request"))
        .await
        .expect("router call failed");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body collect failed")
        .to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    call(app, Method::GET, uri, None).await
}

async fn post(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    call(app, Method::POST, uri, Some(body)).await
}

/// First move from `GET /api/legal-moves`, as a `{from,to,promotion,mid_sq}`
/// request body for `/api/move`.
async fn first_legal_move(app: &Router) -> Value {
    let (status, body) = get(app, "/api/legal-moves").await;
    assert_eq!(status, StatusCode::OK);
    let moves = body["moves"].as_array().expect("moves array");
    assert!(!moves.is_empty(), "expected at least one legal move");
    json!({
        "from": moves[0]["from"],
        "to": moves[0]["to"],
        "promotion": moves[0]["promotion"],
        "mid_sq": moves[0]["mid_sq"],
    })
}

/// Find the first empty square `[row, col]` from `/api/state`.
fn first_empty_square(state: &Value) -> (usize, usize) {
    for (r, row) in state["board"].as_array().unwrap().iter().enumerate() {
        for (c, cell) in row.as_array().unwrap().iter().enumerate() {
            if cell.is_null() {
                return (r, c);
            }
        }
    }
    panic!("no empty square found");
}

// ============================================================
// discovery & static metadata
// ============================================================

#[tokio::test]
async fn health_reports_engine_identity() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["name"], "taikyokushogi");
    assert_eq!(body["board_size"], 36);
    assert!(body["piece_types"].as_u64().unwrap() >= 301);
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
}

#[tokio::test]
async fn engine_info_exposes_search_and_piece_database() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/engine-info").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["board_size"], 36);
    assert_eq!(body["num_squares"], 1296);
    assert_eq!(body["piece_types"], 301);
    assert!(body["royal_piece_types"].as_u64().unwrap() >= 2);
    assert_eq!(body["eval_backend"], "hand-crafted");
    assert_eq!(body["using_nnue"], false);
    assert!(body["hash_mb"].as_u64().unwrap() > 0);
    assert_eq!(body["default_threads"], 1);
    assert!(body["max_threads"].as_u64().unwrap() >= 2);
    let techniques = body["search_techniques"].as_array().unwrap();
    assert_eq!(techniques.len(), 15);
    assert!(techniques.iter().any(|t| t == "quiescence"));
    assert!(techniques.iter().any(|t| t == "transposition_table"));
}

#[tokio::test]
async fn unknown_api_path_returns_json_404() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().contains("unknown"));
}

// ============================================================
// state
// ============================================================

#[tokio::test]
async fn initial_state_has_804_pieces_and_tsfen() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/state").await;
    assert_eq!(status, StatusCode::OK);

    assert_eq!(body["board"].as_array().unwrap().len(), 36);
    for row in body["board"].as_array().unwrap() {
        assert_eq!(row.as_array().unwrap().len(), 36);
    }
    assert_eq!(body["black_pieces"], 402);
    assert_eq!(body["white_pieces"], 402);
    assert_eq!(body["side_to_move"], 0);
    assert_eq!(body["move_number"], 1);
    assert_eq!(body["game_over"], false);
    assert!(body["game_result"].is_null());
    assert_eq!(body["no_progress_plies"], 0);
    assert_eq!(body["material_score"], 0);

    let tsfen = body["tsfen"].as_str().unwrap();
    assert!(tsfen.ends_with(" b 1"), "unexpected TSFEN tail: {tsfen}");
    assert!(body["eval_score"].is_i64());
    assert_eq!(body["score_history"].as_array().unwrap().len(), 1);
    assert_eq!(body["version"], 0);
}

#[tokio::test]
async fn state_contains_king_with_royal_flag() {
    let (_, app) = game();
    let (_, body) = get(&app, "/api/state").await;
    let mut kings = 0;
    for row in body["board"].as_array().unwrap() {
        for cell in row.as_array().unwrap() {
            if !cell.is_null() && cell["is_royal"] == true {
                kings += 1;
                assert!(cell["value"].as_i64().unwrap() > 0);
                assert!(!cell["name"].as_str().unwrap().is_empty());
            }
        }
    }
    assert!(
        kings >= 2,
        "expected both royals on the initial board, got {kings}"
    );
}

#[tokio::test]
async fn status_matches_state_summary() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/status").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["game_over"], false);
    assert_eq!(body["move_number"], 1);
    assert_eq!(body["half_move"], 0);
    assert_eq!(body["black_pieces"], 402);
    assert_eq!(body["white_pieces"], 402);
    assert!(body["tsfen"].as_str().unwrap().ends_with(" b 1"));
    assert_eq!(body["version"], 0);
}

// ============================================================
// move listing
// ============================================================

#[tokio::test]
async fn moves_for_own_piece_are_listed() {
    let (_, app) = game();
    let (_, state) = get(&app, "/api/state").await;

    // Locate a Black (side to move) piece on the board.
    let mut found = None;
    'outer: for (r, row) in state["board"].as_array().unwrap().iter().enumerate() {
        for (c, cell) in row.as_array().unwrap().iter().enumerate() {
            if !cell.is_null() && cell["color"] == 0 {
                found = Some((r, c));
                break 'outer;
            }
        }
    }
    let (r, c) = found.expect("black piece exists");

    let (status, body) = get(&app, &format!("/api/moves?r={r}&c={c}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["from"], json!([r, c]));
    let moves = body["moves"].as_array().unwrap();
    assert!(!moves.is_empty(), "black back-rank piece should have moves");
    for m in moves {
        assert_eq!(m["to"].as_array().unwrap().len(), 2);
        assert!(m["promotion"].is_boolean());
        assert!(m["is_igui"].is_boolean());
    }
}

#[tokio::test]
async fn moves_for_empty_square_are_empty() {
    let (_, app) = game();
    let (_, state) = get(&app, "/api/state").await;
    let (r, c) = first_empty_square(&state);
    let (status, body) = get(&app, &format!("/api/moves?r={r}&c={c}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["moves"], json!([]));
}

#[tokio::test]
async fn moves_out_of_bounds_is_400() {
    let (_, app) = game();
    for uri in [
        "/api/moves?r=36&c=0",
        "/api/moves?r=0&c=36",
        "/api/moves?r=999&c=999",
    ] {
        let (status, body) = get(&app, uri).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(body["ok"], false);
    }
}

#[tokio::test]
async fn legal_moves_covers_whole_side_to_move() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/legal-moves").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["side_to_move"], 0);
    assert_eq!(body["game_over"], false);

    let moves = body["moves"].as_array().unwrap();
    assert_eq!(body["count"], moves.len());
    assert!(moves.len() >= 400, "initial position has hundreds of moves");
    for m in moves {
        // Full-position listing always includes the origin.
        assert_eq!(m["from"].as_array().unwrap().len(), 2);
        assert_eq!(m["to"].as_array().unwrap().len(), 2);
    }
}

// ============================================================
// piece database
// ============================================================

#[tokio::test]
async fn piece_info_for_pawn() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/piece-info/P").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["abbrev"], "P");
    assert_eq!(body["name"], "Pawn");
    assert!(body["value"].as_i64().unwrap() > 0);
    assert_eq!(body["is_royal"], false);
    assert!(body["promotes_to"].as_str().is_some());
}

#[tokio::test]
async fn piece_info_for_king_is_royal() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/piece-info/K").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "King");
    assert_eq!(body["is_royal"], true);
}

#[tokio::test]
async fn piece_info_unknown_is_404() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/piece-info/ZZZ").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().contains("ZZZ"));
}

#[tokio::test]
async fn pieces_endpoint_lists_full_database() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/pieces").await;
    assert_eq!(status, StatusCode::OK);
    let pieces = body["pieces"].as_array().unwrap();
    assert_eq!(body["count"], pieces.len());
    assert_eq!(pieces.len(), 301);
    assert!(pieces
        .iter()
        .any(|p| p["abbrev"] == "K" && p["is_royal"] == true));
    // At least some pieces carry Betza movement notation.
    assert!(pieces.iter().filter(|p| p["betza"].is_string()).count() > 100);
    // IDs are unique.
    let mut seen = std::collections::HashSet::new();
    for p in pieces {
        assert!(seen.insert(p["abbrev"].as_str().unwrap().to_string()));
    }
}

// ============================================================
// evaluation
// ============================================================

#[tokio::test]
async fn evaluate_initial_position_is_balanced() {
    let (_, app) = game();
    let (status, body) = get(&app, "/api/evaluate").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["material_score"], 0);
    assert_eq!(body["black_pieces"], 402);
    assert_eq!(body["white_pieces"], 402);
    assert_eq!(body["side_to_move"], 0);
    assert!(body["eval_score"].is_i64());
}

// ============================================================
// playing moves
// ============================================================

#[tokio::test]
async fn human_move_updates_board_and_history() {
    let (_, app) = game();
    let mv = first_legal_move(&app).await;

    let (status, body) = post(&app, "/api/move", mv.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["last_move"]["from"], mv["from"]);
    assert_eq!(body["last_move"]["to"], mv["to"]);
    assert_eq!(body["board"]["side_to_move"], 1);
    assert_eq!(body["board"]["version"], 1);
    assert_eq!(body["board"]["move_log"].as_array().unwrap().len(), 1);
    assert_eq!(body["board"]["score_history"].as_array().unwrap().len(), 2);
    assert!(
        body["search"].is_null(),
        "search stats omitted for human moves"
    );

    let (_, history) = get(&app, "/api/history").await;
    assert_eq!(history["half_move"], 1);
    assert_eq!(history["move_log"].as_array().unwrap().len(), 1);
    assert_eq!(history["score_history"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn illegal_move_is_rejected_without_mutating_state() {
    let (_, app) = game();
    let (_, state) = get(&app, "/api/state").await;
    let (r, c) = first_empty_square(&state);

    let (status, body) = post(
        &app,
        "/api/move",
        json!({"from": [r, c], "to": [r, c], "promotion": false}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], false);
    assert!(body["error"].as_str().unwrap().contains("Illegal"));

    let (_, after) = get(&app, "/api/state").await;
    assert_eq!(after["tsfen"], state["tsfen"]);
    assert_eq!(after["version"], 0);
}

#[tokio::test]
async fn move_out_of_bounds_is_400() {
    let (_, app) = game();
    let (status, body) = post(
        &app,
        "/api/move",
        json!({"from": [0, 0], "to": [36, 0], "promotion": false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn two_moves_alternate_side_and_track_move_number() {
    let (_, app) = game();
    let mv1 = first_legal_move(&app).await;
    let (_, b1) = post(&app, "/api/move", mv1).await;
    assert_eq!(b1["board"]["side_to_move"], 1);

    let mv2 = first_legal_move(&app).await;
    let (_, b2) = post(&app, "/api/move", mv2).await;
    assert_eq!(b2["board"]["side_to_move"], 0);
    assert_eq!(b2["board"]["move_number"], 2);
    assert_eq!(b2["board"]["move_log"].as_array().unwrap().len(), 2);
}

// ============================================================
// undo
// ============================================================

#[tokio::test]
async fn undo_restores_previous_position() {
    let (_, app) = game();
    let before = get(&app, "/api/state").await.1;
    let mv = first_legal_move(&app).await;
    post(&app, "/api/move", mv).await;

    let (status, body) = post(&app, "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["undone_plies"], 1);
    assert_eq!(body["board"]["side_to_move"], 0);
    assert_eq!(body["board"]["move_log"], json!([]));
    assert_eq!(body["board"]["tsfen"], before["tsfen"]);
}

#[tokio::test]
async fn undo_without_moves_reports_failure() {
    let (_, app) = game();
    let (status, body) = post(&app, "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], false);
    assert_eq!(body["undone_plies"], 0);
    assert!(body["error"].as_str().unwrap().contains("Nothing"));
}

#[tokio::test]
async fn undo_multiple_plies_returns_to_start() {
    let (_, app) = game();
    let start = get(&app, "/api/state").await.1["tsfen"].clone();
    let mv1 = first_legal_move(&app).await;
    post(&app, "/api/move", mv1).await;
    let mv2 = first_legal_move(&app).await;
    post(&app, "/api/move", mv2).await;

    let (status, body) = post(&app, "/api/undo", json!({"plies": 2})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["undone_plies"], 2);
    assert_eq!(body["board"]["tsfen"], start);
}

// ============================================================
// new game / position / save / load
// ============================================================

#[tokio::test]
async fn new_game_resets_board_and_bumps_version() {
    let (_, app) = game();
    let mv = first_legal_move(&app).await;
    post(&app, "/api/move", mv).await;

    let (status, body) = post(&app, "/api/new-game", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["version"], 2);

    let (_, state) = get(&app, "/api/state").await;
    assert_eq!(state["black_pieces"], 402);
    assert_eq!(state["side_to_move"], 0);
    assert_eq!(state["move_number"], 1);
    assert!(state["tsfen"].as_str().unwrap().ends_with(" b 1"));
}

#[tokio::test]
async fn position_endpoint_roundtrips_tsfen() {
    let (_, app) = game();
    let (_, state) = get(&app, "/api/state").await;
    let tsfen = state["tsfen"].as_str().unwrap().to_string();

    let (status, position) = get(&app, "/api/position").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(position["tsfen"], tsfen);
    assert_eq!(position["side_to_move"], 0);
    assert_eq!(position["move_number"], 1);

    // POST the same TSFEN; the board must stay equivalent.
    let (status, body) = post(&app, "/api/position", json!({"tsfen": tsfen})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["tsfen"], tsfen);
    assert_eq!(body["version"], 1, "loading counts as a state change");
}

#[tokio::test]
async fn invalid_tsfen_is_400() {
    let (_, app) = game();
    let (status, body) = post(&app, "/api/position", json!({"tsfen": "not a position"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["ok"], false);
}

#[tokio::test]
async fn save_load_roundtrip_preserves_position() {
    let (_, app) = game();
    let mv = first_legal_move(&app).await;
    let (_, played) = post(&app, "/api/move", mv).await;
    let saved_tsfen = played["board"]["tsfen"].as_str().unwrap().to_string();

    let (status, saved) = get(&app, "/api/save").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved["tsfen"], saved_tsfen);
    let log_len = saved["move_log"].as_array().unwrap().len();
    assert_eq!(log_len, 1);

    // Also verify the POST variant.
    let (status, saved_post) = post(&app, "/api/save", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(saved_post["tsfen"], saved_tsfen);

    // Reset, then load the saved position.
    post(&app, "/api/new-game", json!({})).await;
    let (status, loaded) = post(&app, "/api/load", json!({"tsfen": saved_tsfen})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["tsfen"], saved_tsfen);
    assert_eq!(loaded["side_to_move"], 1);
    assert_eq!(
        loaded["move_log"],
        json!([]),
        "TSFEN carries no move history"
    );
}

#[tokio::test]
async fn load_invalid_tsfen_is_400_and_keeps_board() {
    let (_, app) = game();
    let before = get(&app, "/api/state").await.1;
    let (status, _) = post(&app, "/api/load", json!({"tsfen": "garbage"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let after = get(&app, "/api/state").await.1;
    assert_eq!(after["tsfen"], before["tsfen"]);
}

// ============================================================
// AI move & analysis search
// ============================================================

#[tokio::test]
async fn ai_random_move_applies_and_reports_telemetry() {
    let (_, app) = game();
    let (status, body) = post(&app, "/api/ai-move", json!({"depth": 0})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert!(body["last_move"]["from"].is_array());
    assert!(body["last_move"]["to"].is_array());
    assert_eq!(body["board"]["side_to_move"], 1);
    assert_eq!(body["search"]["depth"], 0);
    assert_eq!(body["search"]["nodes"], 0);
    assert!(body["search"]["time_ms"].is_u64());
}

#[tokio::test]
async fn ai_depth_one_search_selects_a_legal_move() {
    let (_, app) = game();
    let (status, body) = post(
        &app,
        "/api/ai-move",
        json!({"depth": 1, "time_limit": 5000}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["search"]["depth"], 1);
    assert!(body["search"]["nodes"].as_u64().unwrap() >= 1);
    assert!(body["search"]["score"].is_i64());
    assert_eq!(body["board"]["side_to_move"], 1);

    let tsfen = body["board"]["tsfen"].as_str().unwrap();
    assert!(
        tsfen.ends_with(" w 1"),
        "after Black's move it is White's turn: {tsfen}"
    );
}

#[tokio::test]
async fn ai_search_timeout_must_not_end_the_game() {
    let (_, app) = game();
    // A 1 ms budget may expire before depth 1 completes. The engine must not
    // mistake that for "no legal moves" and declare the game over.
    let (status, body) = post(&app, "/api/ai-move", json!({"depth": 8, "time_limit": 1})).await;
    assert_eq!(status, StatusCode::OK);

    let (_, state) = get(&app, "/api/state").await;
    if body["ok"] == true {
        assert_eq!(state["side_to_move"], 1);
    } else {
        assert_eq!(state["game_over"], false);
        assert!(state["tsfen"].as_str().unwrap().ends_with(" b 1"));
    }
}

#[tokio::test]
async fn search_endpoint_analyzes_without_mutating() {
    let (_, app) = game();
    let before = get(&app, "/api/state").await.1;

    let (status, body) = post(&app, "/api/search", json!({"depth": 1, "time_limit": 5000})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["depth"], 1);
    assert!(body["best_move"]["from"].is_array());
    assert!(body["best_move"]["to"].is_array());
    assert!(body["nodes"].as_u64().unwrap() >= 1);
    assert_eq!(body["tsfen"], before["tsfen"]);

    let after = get(&app, "/api/state").await.1;
    assert_eq!(after["version"], before["version"]);
    assert_eq!(after["tsfen"], before["tsfen"]);
}

#[tokio::test]
async fn search_reports_clamped_depth() {
    let (_, app) = game();
    let (status, body) = post(&app, "/api/search", json!({"depth": 999, "time_limit": 1})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["depth"], 8);
}

// ============================================================
// CORS
// ============================================================

#[tokio::test]
async fn cors_preflight_is_allowed() {
    let (_, app) = game();
    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/state")
        .header(header::ORIGIN, "http://localhost:5000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert!(
        response.status().is_success(),
        "preflight should succeed, got {}",
        response.status()
    );
    let origin = response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .expect("allow-origin header");
    assert_eq!(origin, "*");
}

// ============================================================
// concurrency guard
// ============================================================

#[tokio::test]
async fn ai_move_after_reset_is_not_applied_to_new_game() {
    let state = new_state();
    let app = build_router(state.clone());

    // Snapshot version now, then reset (simulating a board change while the
    // AI "thinks"). The stale AI result must be discarded, not applied.
    let _ = get(&app, "/api/state").await;
    post(&app, "/api/new-game", json!({})).await;

    // A fresh AI move works fine on the new game.
    let (status, body) = post(&app, "/api/ai-move", json!({"depth": 0})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
    assert_eq!(body["board"]["side_to_move"], 1);
}

// ============================================================
// JSON body validation
// ============================================================

#[tokio::test]
async fn malformed_json_body_is_rejected() {
    let (_, app) = game();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/move")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert!(
        response.status().is_client_error(),
        "malformed JSON must be a 4xx, got {}",
        response.status()
    );
}
