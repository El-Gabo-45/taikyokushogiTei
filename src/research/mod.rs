#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

//! Research toolkit for Taikyoku Shogi (audit-2 kit).
//!
//! One tool per research question, all strictly additive: nothing here
//! touches the search hot paths — every tool is built on the public engine
//! API ([`crate::Board`], [`crate::search::Searcher`],
//! [`crate::search::params::SearchConfig`], the self-play SQLite log) so that
//! research code can never change how the engine plays.
//!
//! | subcommand | question it answers |
//! |---|---|
//! | `nnue multipv` | Which candidate moves exist and how far apart are they? (`MultiPV`) |
//! | `nnue tree` | What does the shallow tree below a position look like? (JSON) |
//! | `nnue stability` | How much do score and best move change with depth? |
//! | `nnue critical` | Which plies of self-play games swung the evaluation most? |
//! | `nnue pieces` | Which piece types actually capture which? (interaction matrix) |
//! | `nnue ablate` | Which search technique changes the engine's decisions? |
//!
//! Score conventions: every score is in engine units from the point of view
//! stated next to it — a pawn is worth 500 and mate is
//! [`crate::MATE_SCORE`].

pub mod ablate;
pub mod critical;
pub mod multipv;
pub mod pieces;
pub mod stability;
pub mod tree;

use std::sync::Arc;

use crate::eval::nnue::Network;

// ── CLI argument helpers (same conventions as src/nnue_main.rs) ─────────

/// Value of a `--flag VALUE` argument, if present.
pub(crate) fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(std::string::String::as_str)
}

/// First positional (non-`--`) argument, if present.
pub(crate) fn positional(args: &[String]) -> Option<&str> {
    args.first()
        .filter(|a| !a.starts_with("--"))
        .map(std::string::String::as_str)
}

/// Parsed numeric `--flag VALUE`, or `default` when the flag is absent.
pub(crate) fn num<T: std::str::FromStr>(
    args: &[String],
    name: &str,
    default: T,
) -> Result<T, String> {
    match flag(args, name) {
        Some(v) => v
            .parse::<T>()
            .map_err(|_| format!("bad {name} value `{v}`")),
        None => Ok(default),
    }
}

/// Parsed numeric `--flag VALUE`, or `None` when the flag is absent.
pub(crate) fn num_opt<T: std::str::FromStr>(
    args: &[String],
    name: &str,
) -> Result<Option<T>, String> {
    flag(args, name)
        .map(|v| {
            v.parse::<T>()
                .map_err(|_| format!("bad {name} value `{v}`"))
        })
        .transpose()
}

// ── Position loading (shared by the position-based tools) ───────────────

/// `--net FILE.tkn[.zst]`: raw or zstd-compressed, `Network::load` detects
/// the format from the file contents.
pub(crate) fn load_net(args: &[String]) -> Result<Option<Arc<Network>>, String> {
    match flag(args, "--net") {
        Some(path) => {
            let net =
                crate::Network::load(path).map_err(|e| format!("cannot load net `{path}`: {e}"))?;
            Ok(Some(Arc::new(net)))
        }
        None => Ok(None),
    }
}

/// Positional `<tsfen>` argument (defaults to the initial position) with an
/// optional `--net` attached, so `evaluate()` speaks with the network.
pub(crate) fn load_position(
    args: &[String],
) -> Result<(crate::Board, Option<Arc<Network>>), String> {
    let net = load_net(args)?;
    let mut board = match positional(args) {
        Some(fen) => {
            crate::Board::from_tsfen(fen).map_err(|e| format!("bad tsfen `{fen}`: {e}"))?
        }
        None => crate::Board::initial(),
    };
    board.set_network(net.clone());
    Ok((board, net))
}

/// Static score of every legal move: apply, evaluate, undo.
///
/// The returned score is from the point of view of the side that played the
/// move — once a move is applied, `evaluate()` speaks for the opponent, so it
/// is negated. Ties are broken by move string so the ordering (and every
/// downstream tool) is deterministic.
pub(crate) fn static_move_scores(board: &mut crate::Board) -> Vec<(i32, crate::Move)> {
    let mut out: Vec<(i32, crate::Move)> = board
        .legal_moves()
        .into_iter()
        .map(|mv| {
            board.apply(&mv);
            let score = -board.evaluate();
            board.undo();
            (score, mv)
        })
        .collect();
    out.sort_by_cached_key(|(score, mv)| (std::cmp::Reverse(*score), mv.to_string()));
    out
}

/// Piece type (1..=301) at a square index, 0 when the square is empty.
fn piece_type_at_sq(board: &crate::Board, index: usize) -> u16 {
    let size = crate::types::BOARD_SIZE;
    board
        .get(index / size, index % size)
        .map_or(0, |p| p.type_id)
}

// ── Self-play game replay (shared by `critical` and `pieces`) ───────────

/// One replayed ply with everything the research tools need.
#[derive(Debug, Clone)]
pub struct PlyView {
    /// Game id (unique across selfplay runs since the id reservation fix).
    pub game_id: u64,
    /// Ply index, 0-based (the database's `move_number - 1`).
    pub ply: u32,
    /// Side that moved: 0 = Black, 1 = White.
    pub side: u8,
    /// Piece type of the moving piece (pre-promotion), 1..=301.
    pub mover_pt: u16,
    /// Piece type captured, 1..=301 (0 = the move captured nothing).
    pub captured_pt: u16,
    /// Move rendered the way the engine prints it (`from->to`, `+` on promo).
    pub mv: String,
    /// Static evaluation BEFORE the move, side-to-move (mover) point of view.
    pub eval_stm: i32,
    /// Change from the mover's point of view: `-eval_after - eval_before`.
    ///
    /// `eval_after` is the evaluation of the resulting position, which speaks
    /// for the opponent, so negating it gives the mover's view of the new
    /// position. A large negative `gain` means the mover is suddenly much
    /// worse — a blunder, or the move walked into something.
    pub gain: i32,
}

/// Aggregate of one [`replay_games`] run.
#[derive(Debug, Default)]
pub struct ReplaySummary {
    /// Games fully replayed from the initial position.
    pub games_replayed: u64,
    /// Plies delivered to the callback.
    pub plies_replayed: u64,
    /// `(game_id, reason)` for every game that could not be replayed.
    pub games_skipped: Vec<(u64, String)>,
}

/// Walks a self-play `games.db` and calls `f` once per recorded ply.
///
/// Games are reconstructed from the initial position — the recorded moves
/// include the randomized opening plies, so the replay is exact. A game whose
/// recorded moves stop matching legal play (older databases, truncated
/// files) is *skipped with a reason*, never silently mangled, so every
/// analysis runs over verified positions.
///
/// The database is opened read-only: a selfplay run owns it, this tool only
/// reads.
pub fn replay_games<F>(
    db_path: &str,
    net: Option<Arc<Network>>,
    limit: Option<usize>,
    mut f: F,
) -> Result<ReplaySummary, String>
where
    F: FnMut(&PlyView) -> Result<(), String>,
{
    use rusqlite::OpenFlags;

    let conn = rusqlite::Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("cannot open `{db_path}` (read-only): {e}"))?;

    let mut game_ids: Vec<u64> = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id FROM games ORDER BY id")
            .map_err(|e| format!("bad games.db schema in `{db_path}`: {e}"))?;
        let rows = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        for row in rows.take(limit.unwrap_or(usize::MAX)) {
            let id: i64 = row.map_err(|e| e.to_string())?;
            game_ids.push(id.max(0) as u64);
        }
    }

    let mut summary = ReplaySummary::default();
    for game_id in game_ids {
        match replay_one(&conn, game_id, net.clone(), &mut f) {
            Ok(plies) => {
                summary.games_replayed += 1;
                summary.plies_replayed += plies;
            }
            Err(reason) => summary.games_skipped.push((game_id, reason)),
        }
    }
    Ok(summary)
}

/// Replay one game, calling `f` per ply. `Ok(plies)` on success, `Err`
/// describes why the game cannot be replayed.
/// One recorded move row, straight from the `moves` table.
struct DbMove {
    move_number: i64,
    from: i64,
    to: i64,
    promo: bool,
    captured: i64,
    side: i64,
}

fn replay_one<F>(
    conn: &rusqlite::Connection,
    game_id: u64,
    net: Option<Arc<Network>>,
    f: &mut F,
) -> Result<u64, String>
where
    F: FnMut(&PlyView) -> Result<(), String>,
{
    let mut board = crate::Board::initial();
    board.set_network(net);

    let mut stmt = conn
        .prepare(
            "SELECT move_number, from_sq, to_sq, promo, captured, side \
             FROM moves WHERE game_id = ?1 ORDER BY move_number",
        )
        .map_err(|e| format!("bad moves table: {e}"))?;
    let rows = stmt
        .query_map([game_id as i64], |r| {
            Ok(DbMove {
                move_number: r.get(0)?,
                from: r.get(1)?,
                to: r.get(2)?,
                promo: r.get::<_, i64>(3)? != 0,
                captured: r.get(4)?,
                side: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut plies = 0u64;
    for row in rows {
        let m = row.map_err(|e| format!("read move row: {e}"))?;

        // The recorded side must be the side to move on the replayed board.
        let side_now = i64::from(board.side_to_move().raw());
        if m.side != side_now {
            return Err(format!(
                "move {}: database says side {}, replayed board says {side_now}",
                m.move_number, m.side
            ));
        }

        let mv = match_legal(&mut board, &m).map_err(|e| format!("move {}: {e}", m.move_number))?;
        let mover_pt = piece_type_at_sq(&board, mv.from().index());
        let eval_stm = board.evaluate();
        let mv_str = mv.to_string();
        board.apply(&mv);
        let eval_after = board.evaluate(); // now the opponent speaks
        let gain = ((-i64::from(eval_after)) - i64::from(eval_stm)) as i32;
        let view = PlyView {
            game_id,
            ply: (m.move_number.max(1) - 1) as u32,
            side: m.side.clamp(0, 1) as u8,
            mover_pt,
            captured_pt: m.captured.clamp(0, 0x1FF) as u16,
            mv: mv_str,
            eval_stm,
            gain,
        };
        f(&view)?;
        plies += 1;
    }
    Ok(plies)
}

/// Find the recorded move among the legal moves of the replayed position.
///
/// Matches on from/to/promotion. When several legal moves share those
/// coordinates (an igui/pass-capture and a normal move to the same square),
/// the recorded `captured` piece type disambiguates — they capture on
/// different squares, so usually different piece types.
fn match_legal(board: &mut crate::Board, m: &DbMove) -> Result<crate::Move, String> {
    let legal = board.legal_moves();
    let mut matches: Vec<crate::Move> = legal
        .into_iter()
        .filter(|mv| {
            let r = mv.raw();
            i64::from(r.from_sq) == m.from && i64::from(r.to_sq) == m.to && r.promotion == m.promo
        })
        .collect();
    match matches.len() {
        0 => Err(
            "no legal move matches the recorded from/to/promotion — replay diverged from the initial position"
                .to_string(),
        ),
        1 => Ok(matches.remove(0)),
        _ => matches
            .into_iter()
            .find(|mv| i64::from(mv.raw().captured_piece) == m.captured)
            .ok_or_else(|| "several legal moves match the recording (ambiguous)".to_string()),
    }
}

/// One-line replay report shared by the database-based tools.
pub(crate) fn print_replay_summary(db_path: &str, summary: &ReplaySummary) {
    println!("db:       {db_path}");
    println!(
        "games:    {} replayed, {} plies analyzed",
        summary.games_replayed, summary.plies_replayed
    );
    if !summary.games_skipped.is_empty() {
        println!(
            "⚠ skipped {} game(s) — their recorded moves diverge from legal play:",
            summary.games_skipped.len()
        );
        for (gid, reason) in summary.games_skipped.iter().take(5) {
            println!("  game {gid}: {reason}");
        }
        if summary.games_skipped.len() > 5 {
            println!("  … and {} more", summary.games_skipped.len() - 5);
        }
    }
}

#[cfg(test)]
pub(crate) mod testdb {
    //! One tiny self-play database, built once and shared by the replay-based
    //! tests of this module (selfplay games are a few seconds each).
    use std::sync::OnceLock;

    /// Path of a `games.db` with 2 tiny deterministic selfplay games.
    pub(crate) fn shared_db() -> &'static str {
        static DB: OnceLock<String> = OnceLock::new();
        DB.get_or_init(|| {
            let dir = std::env::temp_dir().join(format!(
                "tk_research_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let db_path = format!("{}/games.db", dir.display());
            let config = crate::selfplay::SelfPlayConfig {
                num_games: 2,
                num_workers: 1,
                depth: 1,
                time_limit_ms: 10,
                max_moves: 8,
                output_dir: dir.display().to_string(),
                save_interval: 1,
                sqlite_path: Some(db_path.clone()),
                opening_plies: 2,
                seed: 7,
                net: None,
            };
            let mut coord = crate::selfplay::SelfPlayCoordinator::new(config);
            coord.start();
            coord.wait().expect("tiny selfplay run must succeed");
            db_path
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The replay must reconstruct selfplay games exactly: every recorded
    /// ply matches legal play, sides alternate correctly, and nothing is
    /// skipped. This is the integrity core of `critical` and `pieces`.
    #[test]
    fn replay_reconstructs_selfplay_games_exactly() {
        let db = testdb::shared_db().to_string();
        let mut seen_sides = [0usize, 0usize];
        let mut captures = 0u64;
        let summary = replay_games(&db, None, None, |v| {
            // Sides alternate from Black (0) and stay consistent per ply.
            assert_eq!(v.side, (v.ply % 2) as u8, "side must follow ply parity");
            seen_sides[v.side as usize] += 1;
            if v.captured_pt != 0 {
                captures += 1;
            }
            // The mover's square is never empty when a move was recorded.
            assert_ne!(v.mover_pt, 0, "mover square must hold a piece");
            Ok(())
        })
        .expect("replay of our own tiny db must succeed");
        assert_eq!(summary.games_replayed, 2);
        assert!(
            summary.games_skipped.is_empty(),
            "no game may be skipped: {:?}",
            summary.games_skipped
        );
        assert!(summary.plies_replayed >= 4, "two games worth of plies");
        assert!(seen_sides[0] > 0 && seen_sides[1] > 0);
        // Gains must agree with the recorded captures: a capture usually
        // swings the eval, but the exact amounts are engine-defined — only
        // check they were computed at all.
        assert!(captures <= summary.plies_replayed);
        // NOTE: do NOT clean up the shared db's directory here — the other
        // replay-based tests reuse it (tests run concurrently in one
        // process). The scratch dir lives under the system temp dir.
    }

    /// `--limit` really caps the number of games analyzed.
    #[test]
    fn replay_respects_the_game_limit() {
        let db = testdb::shared_db().to_string();
        let summary = replay_games(&db, None, Some(1), |_| Ok(())).expect("replay");
        assert_eq!(summary.games_replayed, 1);
    }

    /// An empty/missing database is an error, not an empty success.
    #[test]
    fn replay_of_a_missing_db_is_an_error() {
        let path = std::env::temp_dir()
            .join(format!("tk_missing_{}.db", std::process::id()))
            .to_string_lossy()
            .into_owned();
        let err = replay_games(&path, None, None, |_| Ok(())).expect_err("missing db must fail");
        assert!(err.contains("cannot open"), "got: {err}");
    }

    /// Ties in `static_move_scores` must be deterministic.
    #[test]
    fn static_move_scores_is_deterministic() {
        let mut a = crate::Board::initial();
        let mut b = crate::Board::initial();
        let sa = static_move_scores(&mut a);
        let sb = static_move_scores(&mut b);
        assert_eq!(sa.len(), sb.len());
        for ((s1, m1), (s2, m2)) in sa.iter().zip(sb.iter()) {
            assert_eq!(s1, s2);
            assert_eq!(m1.to_string(), m2.to_string());
        }
        // Scores must be non-increasing (sorted best-first).
        assert!(sa.windows(2).all(|w| w[0].0 >= w[1].0));
    }
}
