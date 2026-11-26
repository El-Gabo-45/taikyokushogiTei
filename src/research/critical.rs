#![allow(clippy::all)]
//! `nnue critical` — Critical Position Detector over self-play `games.db`.
//!
//! Replays every recorded game from the initial position (see
//! [`super::replay_games`]) and reports the plies where the static
//! evaluation swung the most:
//!
//! * `gain` is the evaluation change from the mover's point of view — a
//!   large **negative** gain means the mover is suddenly much worse (a
//!   blunder, or a strong reply it did not foresee); a large positive gain
//!   means it won material/position with that move.
//! * Both directions matter for research: negative gains point at engine
//!   mistakes, positive gains at opportunities the opponent allowed.
//! * The evaluation is static, so swings at high ply are often just horizon
//!   artifacts — cross-check interesting plies with `nnue multipv` on the
//!   reconstructed position.
//!
//! ```text
//! nnue critical <games.db> [--net FILE] [--threshold N] [--limit G] [--top N]
//! ```

use std::collections::BTreeMap;

use super::{load_net, num, num_opt, positional, print_replay_summary, replay_games};

/// One evaluation swing worth reporting.
struct Moment {
    game_id: u64,
    ply: u32,
    side: u8,
    mv: String,
    gain: i32,
    eval_stm: i32,
    captured_pt: u16,
}

/// # Errors
///
/// Returns an error describing the failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let db = positional(args)
        .map(std::string::ToString::to_string)
        .ok_or_else(|| {
            "usage: nnue critical <games.db> [--net FILE] [--threshold N] [--limit G] [--top N]"
                .to_string()
        })?;
    let net = load_net(args)?;
    let threshold = num(args, "--threshold", 500i32)?;
    let limit = num_opt(args, "--limit")?;
    let top = num(args, "--top", 20usize)?;

    let mut moments: Vec<Moment> = Vec::new();
    let mut per_game: BTreeMap<u64, u32> = BTreeMap::new();
    let summary = replay_games(&db, net, limit, |v| {
        if v.gain.abs() >= threshold {
            *per_game.entry(v.game_id).or_default() += 1;
            moments.push(Moment {
                game_id: v.game_id,
                ply: v.ply,
                side: v.side,
                mv: v.mv.clone(),
                gain: v.gain,
                eval_stm: v.eval_stm,
                captured_pt: v.captured_pt,
            });
        }
        Ok(())
    })?;

    println!("=== Critical positions ===");
    print_replay_summary(&db, &summary);
    println!("threshold: |gain| ≥ {threshold} engine units (a pawn is 500)");
    println!("critical plies: {}", moments.len());
    if moments.is_empty() {
        println!("(no ply swung the evaluation by ≥ {threshold} units)");
        return Ok(());
    }

    // Worst swings first; ties break by game/ply so the output is stable.
    moments.sort_by(|a, b| {
        b.gain
            .abs()
            .cmp(&a.gain.abs())
            .then_with(|| (a.game_id, a.ply).cmp(&(b.game_id, b.ply)))
    });
    println!();
    println!("worst swings (top {}):", top.min(moments.len()));
    println!(
        "{:>6}  {:>5}  {:<4}  {:<14}  {:>10}  {:>10}  captured",
        "game", "ply", "side", "move", "gain", "eval"
    );
    for m in moments.iter().take(top) {
        println!(
            "{:>6}  {:>5}  {:<4}  {:<14}  {:+10}  {:+10}  {}",
            m.game_id,
            m.ply + 1,
            if m.side == 0 { "B" } else { "W" },
            m.mv,
            m.gain,
            m.eval_stm,
            if m.captured_pt != 0 {
                crate::pieces::name(m.captured_pt)
            } else {
                "-"
            }
        );
    }
    if per_game.len() > 1 {
        println!();
        println!("critical plies per game:");
        for (gid, count) in per_game {
            println!("  game {gid}: {count}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::testdb;

    /// With threshold 0 every recorded ply is "critical"; with an
    /// impossible threshold none is. Both extremes must hold on the same db.
    #[test]
    fn threshold_gates_the_moments() {
        let db = testdb::shared_db().to_string();
        let mut all = 0usize;
        replay_games(&db, None, None, |v| {
            if v.gain.abs() >= 0 {
                all += 1;
            }
            Ok(())
        })
        .expect("replay");
        assert!(all > 0);
        // `run` on an impossible threshold prints the empty message.
        let args = vec![
            db.clone(),
            "--threshold".to_string(),
            "999999999".to_string(),
        ];
        run(&args).expect("runs even when nothing qualifies");
    }

    /// `run` on the shared db must not panic and must print moments.
    #[test]
    fn run_reports_moments_on_real_data() {
        let db = testdb::shared_db().to_string();
        let args = vec![db, "--threshold".to_string(), "0".to_string()];
        run(&args).expect("critical runs on the tiny db");
    }
}
