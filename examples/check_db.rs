// Benchmarks/tools: panic-on-failure is intentional for a diagnostic.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::unreadable_literal,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::type_complexity,
    clippy::format_push_string,
    clippy::items_after_statements,
    clippy::doc_markdown,
    clippy::needless_borrow,
    clippy::redundant_closure,
    clippy::similar_names,
    clippy::must_use_candidate,
    clippy::map_unwrap_or,
    clippy::unnecessary_cast
)]

use rusqlite::Connection;
fn main() {
    let conn = Connection::open("training_data/games.db").unwrap();
    let mut stmt = conn
        .prepare("SELECT id, result, total_moves, depth, duration_ms FROM games")
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .unwrap();
    for r in rows {
        let (id, result, moves, depth, dur) = r.unwrap();
        println!("Game {id}: result={result} moves={moves} depth={depth} ms={dur}");
    }
    let mut stmt2 = conn.prepare("SELECT game_id, move_number, from_sq, to_sq, promo, captured, side FROM moves LIMIT 10").unwrap();
    let rows2 = stmt2
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .unwrap();
    for r in rows2 {
        let (gid, mn, from, to, promo, cap, side) = r.unwrap();
        println!(
            "  Move: game={gid} #{mn} from={from} to={to} promo={promo} cap={cap} side={side}"
        );
    }
}
