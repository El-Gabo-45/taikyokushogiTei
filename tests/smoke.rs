//! Production smoke tests for the Taikyoku Shogi engine library.
//!
//! These tests exercise the public API directly, verifying the audit-fix items:
//!   1. igui mid_sq identity — `apply_by_coord` distinguishes moves from the
//!      same coordinates via mid_sq
//!   2. out-of-bounds coordinate rejection — returns `false`, no panic
//!   3. piece info — `piece_info("P")` returns valid data
//!   4. board state — `Board::initial()` is well-formed
//!   5. search — `Board::search()` returns a legal move at various depths
//!
//! Additionally, every search technique can be independently enabled/disabled
//! via `SearchConfig`, mirroring the CLI `--no-*` flags for ablation testing.
//!
//! Run with: `cargo test --test smoke`

use taikyokushogi::search::params::SearchConfig;
use taikyokushogi::{Board, Color};
// ── Tests ────────────────────────────────────────────────────────

#[test]
fn piece_info_p_returns_valid_data() {
    let info = piece_info("P").expect("piece P must exist");
    assert_eq!(info.abbrev, "P");
    assert!(!info.name.is_empty());
    assert!(info.value > 0);
}

#[test]
fn piece_info_unknown_returns_none() {
    assert!(piece_info("ZZZ").is_none());
}

#[test]
fn board_initial_is_well_formed() {
    let board = Board::initial();
    assert_eq!(board.side_to_move(), Color::Black);
    assert_eq!(board.move_number(), 1);
    assert_eq!(
        board.piece_count(Color::Black),
        board.piece_count(Color::White)
    );
    assert!(board.piece_count(Color::Black) > 0);
    assert!(board.game_result().is_none());
}

#[test]
fn board_has_36x36_cells() {
    let board = Board::initial();
    for r in 0..36 {
        for c in 0..36 {
            let _ = board.get(r, c);
        }
    }
}

#[test]
fn search_depth_two_returns_move() {
    let mut board = Board::initial();
    let r = board.search(2, 0);
    assert!(r.best_move.is_some(), "depth-2 search must return a move");
    assert!(r.nodes > 0);
    // Score should not be a mate score (position is not terminal).
    assert!(r.score.abs() < 1_000_000);
}

#[test]
fn search_depth_four_returns_move() {
    let mut board = Board::initial();
    let r = board.search(4, 0);
    assert!(r.best_move.is_some(), "depth-4 search must return a move");
    assert!(r.nodes > 0);
}

#[test]
fn apply_by_coord_out_of_bounds_returns_false_no_panic() {
    let mut board = Board::initial();
    for &(fr, fc, tr, tc) in &[
        (36, 0, 35, 0),
        (0, 36, 35, 0),
        (35, 0, 36, 0),
        (35, 0, 35, 36),
        (100, 100, 101, 101),
    ] {
        assert!(
            !board.apply_by_coord(fr, fc, tr, tc, false, None),
            "apply_by_coord({fr},{fc}->{tr},{tc}) must return false"
        );
    }
    assert_eq!(board.side_to_move(), Color::Black);
}

#[test]
fn apply_by_coord_legal_move_returns_true() {
    let mut board = Board::initial();
    let moves = board.legal_moves();
    assert!(!moves.is_empty());
    let m = &moves[0];
    let raw = m.raw();
    let fr = raw.from_sq as usize / 36;
    let fc = raw.from_sq as usize % 36;
    let tr = raw.to_sq as usize / 36;
    let tc = raw.to_sq as usize % 36;
    assert!(
        board.apply_by_coord(fr, fc, tr, tc, false, None),
        "legal move must be accepted"
    );
    assert_eq!(board.side_to_move(), Color::White);
}

#[test]
fn apply_by_coord_mid_sq_rejects_mismatch() {
    let mut board = Board::initial();
    let moves = board.legal_moves();
    assert!(!moves.is_empty());
    let m = &moves[0];
    let raw = m.raw();
    let fr = raw.from_sq as usize / 36;
    let fc = raw.from_sq as usize % 36;
    let tr = raw.to_sq as usize / 36;
    let tc = raw.to_sq as usize % 36;
    assert!(
        board.apply_by_coord(fr, fc, tr, tc, false, None),
        "mid_sq=None must match a normal move"
    );
    board.undo();
    assert!(
        !board.apply_by_coord(fr, fc, tr, tc, false, Some(500)),
        "mid_sq=Some(500) must not match a normal move"
    );
    assert_eq!(board.side_to_move(), Color::Black);
}
// ── SearchConfig technique toggle tests ──────────────────────────────

/// Every technique flag can be independently disabled; a search still returns
/// a move. Mirrors the CLI `--no-*` flags.
#[test]
fn search_config_all_toggles_work() {
    // Production config: everything on.
    let mut board = Board::initial();
    let cfg = SearchConfig::production();
    let mut s = taikyokushogi::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut s, 1, 0);
    assert!(r.best_move.is_some());

    // Disable each technique one at a time — none may crash.
    let techniques: Vec<(fn(&mut SearchConfig), &'static str)> = vec![
        (|c| c.razoring = false, "razoring"),
        (|c| c.reverse_futility = false, "rfp"),
        (|c| c.futility = false, "futility"),
        (|c| c.null_move_pruning = false, "null-move"),
        (|c| c.probcut = false, "probcut"),
        (|c| c.transposition_table = false, "tt"),
        (|c| c.killer_moves = false, "killers"),
        (|c| c.history_heuristic = false, "history"),
        (|c| c.counter_moves = false, "counter"),
        (|c| c.internal_iterative_deepening = false, "iid"),
        (|c| c.late_move_reductions = false, "lmr"),
        (|c| c.late_move_pruning = false, "lmp"),
        (|c| c.capture_beam = false, "beam"),
        (|c| c.hash_move_extension = false, "hash-ext"),
        (|c| c.quiescence = false, "quiescence"),
    ];

    for (disable, name) in techniques {
        let mut board = Board::initial();
        let mut cfg = SearchConfig::production();
        disable(&mut cfg);
        let mut s = taikyokushogi::search::Searcher::with_config(cfg);
        let r = board.search_with(&mut s, 1, 0);
        assert!(
            r.best_move.is_some(),
            "search with {} disabled must return a move",
            name
        );
    }
}
/// All techniques off (minimal config) still produces a best move — plain
/// alpha-beta is always correct.
#[test]
fn search_config_minimal_produces_move() {
    let mut board = Board::initial();
    let mut s = taikyokushogi::search::Searcher::with_config(SearchConfig::minimal());
    let r = board.search_with(&mut s, 3, 0);
    assert!(r.best_move.is_some());
    assert!(r.nodes > 0);
}

/// The board is not mutated by a search — side to move is unchanged.
#[test]
fn search_does_not_mutate_board() {
    let mut board = Board::initial();
    let before = board.side_to_move();
    let mut s = taikyokushogi::search::Searcher::with_config(SearchConfig::production());
    let _ = board.search_with(&mut s, 3, 0);
    assert_eq!(board.side_to_move(), before);
}
use taikyokushogi::piece_info;
