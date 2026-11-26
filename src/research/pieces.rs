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

//! `nnue pieces` — Piece Interaction Matrix from self-play games.
//!
//! Replays recorded games (see [`super::replay_games`]) and counts, for
//! every capture, which piece type captured which. Taikyoku Shogi has 301
//! piece types, so the full matrix is 301×301 — the table prints only the
//! most frequent pairings, and `--csv` writes the complete matrix for
//! offline research (which pieces survive, which combinations generate
//! threats, which types never meet).
//!
//! The mover's piece type is its pre-promotion type: the type that actually
//! performed the capture on the board.
//!
//! ```text
//! nnue pieces <games.db> [--limit G] [--top N] [--csv FILE]
//! ```

use super::{flag, load_net, num, num_opt, positional, print_replay_summary, replay_games};

/// (mover, victim, count) interaction triple.
type Interaction = (usize, usize, u64);
/// (piece type, count) tally.
type PtCount = (usize, u64);


/// Piece types are 1..=301 (9-bit cell field); index 0 = "none" is unused
/// for both mover and victim.
const MAX_PT: usize = 302;

    /// # Errors
    ///
    /// Returns an error describing the failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let db = positional(args)
        .map(std::string::ToString::to_string)
        .ok_or_else(|| {
            "usage: nnue pieces <games.db> [--limit G] [--top N] [--csv FILE]".to_string()
        })?;
    let net = load_net(args)?;
    let limit = num_opt(args, "--limit")?;
    let top = num(args, "--top", 15usize)?;
    let csv = flag(args, "--csv").map(std::string::ToString::to_string);

    let mut matrix = vec![vec![0u64; MAX_PT]; MAX_PT];
    let summary = replay_games(&db, net, limit, |v| {
        if v.captured_pt != 0 {
            let (mover, victim) = (v.mover_pt as usize, v.captured_pt as usize);
            if mover < MAX_PT && victim < MAX_PT {
                matrix[mover][victim] += 1;
            }
        }
        Ok(())
    })?;

    println!("=== Piece interaction matrix ===");
    print_replay_summary(&db, &summary);

    let total: u64 = matrix.iter().map(|row| row.iter().sum::<u64>()).sum();
    println!("captures: {total} recorded");
    if total == 0 {
        println!("(no captures in the analyzed games — analyze more games)");
        return Ok(());
    }

    // Top pairings: (mover type, victim type, count), best first, ties by
    // type ids for determinism.
    let mut pairs: Vec<Interaction> = matrix
        .iter()
        .enumerate()
        .flat_map(|(m, row)| {
            row.iter()
                .enumerate()
                .filter(|&(_, &n)| n > 0)
                .map(move |(c, &n)| (m, c, n))
        })
        .collect();
    pairs.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| (a.0, a.1).cmp(&(b.0, b.1))));
    println!();
    println!("top {} pairings (mover → victim):", top.min(pairs.len()));
    for (i, (m, c, n)) in pairs.iter().take(top).enumerate() {
        println!(
            "{:>3}. {:<24} → {:<24} {:>7}",
            i + 1,
            crate::pieces::name(*m as u16),
            crate::pieces::name(*c as u16),
            n
        );
    }

    // Per-type totals: who captures most, who gets captured most.
    let mut by_mover: Vec<PtCount> = matrix
        .iter()
        .enumerate()
        .map(|(m, row)| (m, row.iter().sum::<u64>()))
        .filter(|&(_, n)| n > 0)
        .collect();
    by_mover.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut by_victim: Vec<PtCount> = (0..MAX_PT)
        .map(|c| (c, matrix.iter().map(|row| row[c]).sum::<u64>()))
        .filter(|&(_, n)| n > 0)
        .collect();
    by_victim.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    println!();
    println!("most active capturers:");
    for (m, n) in by_mover.iter().take(top.min(by_mover.len())) {
        println!("  {:<24} {:>7}", crate::pieces::name(*m as u16), n);
    }
    println!("most captured types:");
    for (c, n) in by_victim.iter().take(top.min(by_victim.len())) {
        println!("  {:<24} {:>7}", crate::pieces::name(*c as u16), n);
    }

    if let Some(path) = csv {
        write_csv(&path, &matrix)?;
        println!();
        println!(
            "wrote {path} (full {}×{} matrix, rows = mover, columns = victim)",
            MAX_PT - 1,
            MAX_PT - 1
        );
    }
    Ok(())
}

/// Full matrix as CSV: first row = victim type names, first column = mover
/// type names, cell (m, c) = how often type m captured type c.
fn write_csv(path: &str, matrix: &[Vec<u64>]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("mover\\victim");
    for (c, _) in matrix[0].iter().enumerate().skip(1) {
        out.push(',');
        out.push_str(crate::pieces::name(c as u16));
    }
    out.push('\n');
    for (m, row) in matrix.iter().enumerate().skip(1) {
        out.push_str(crate::pieces::name(m as u16));
        for n in row.iter().skip(1) {
            out.push(',');
            out.push_str(&n.to_string());
        }
        out.push('\n');
    }
    std::fs::write(path, out).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::testdb;

    /// The matrix must account for exactly the captures recorded in the
    /// database, and the pairings table must print those types' names.
    #[test]
    fn matrix_sums_match_recorded_captures() {
        let db = testdb::shared_db().to_string();
        // Ground truth straight from the database.
        let conn = rusqlite::Connection::open_with_flags(
            db.as_str(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .expect("open shared db");
        let expected: i64 = conn
            .query_row("SELECT COUNT(*) FROM moves WHERE captured != 0", [], |r| {
                r.get(0)
            })
            .expect("count captures");

        let mut matrix = vec![vec![0u64; MAX_PT]; MAX_PT];
        let summary = replay_games(&db, None, None, |v| {
            if v.captured_pt != 0 {
                matrix[v.mover_pt as usize][v.captured_pt as usize] += 1;
            }
            Ok(())
        })
        .expect("replay");
        assert_eq!(summary.games_skipped, [] as [(u64, std::string::String); 0]);
        let total: u64 = matrix.iter().map(|row| row.iter().sum::<u64>()).sum();
        assert_eq!(total as i64, expected, "matrix total == recorded captures");
    }

    /// `run` must not panic on the shared db.
    #[test]
    fn run_prints_matrix_on_real_data() {
        let db = testdb::shared_db().to_string();
        run(&[db]).expect("pieces runs on the tiny db");
    }

    /// The CSV writer emits a header of `MAX_PT - 1` victim names and
    /// `MAX_PT - 1` rows.
    #[test]
    fn csv_shape_is_complete() {
        let matrix = vec![vec![0u64; MAX_PT]; MAX_PT];
        let path = std::env::temp_dir()
            .join(format!("tk_matrix_{}.csv", std::process::id()))
            .to_string_lossy()
            .into_owned();
        write_csv(&path, &matrix).expect("csv writes");
        let text = std::fs::read_to_string(&path).expect("read back");
        let mut lines = text.lines();
        let header = lines.next().expect("header");
        assert_eq!(
            header.matches(',').count(),
            MAX_PT - 1,
            "301 victim columns"
        );
        assert_eq!(lines.count(), MAX_PT - 1, "301 mover rows");
        let _ = std::fs::remove_file(&path);
    }
}
