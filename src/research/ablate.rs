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

//! `nnue ablate` — decision-level ablation of the search techniques.
//!
//! For every search technique, the tool searches the same positions with the
//! production config and with that ONE technique disabled — via
//! [`crate::search::Searcher::with_config`], so no engine code is touched —
//! and reports how often the best move, the score and the node count change.
//!
//! This is the decision-level half of the audit's "Pruning Analyzer / Ablation
//! Testing Framework": it finds techniques whose removal changes decisions on
//! your positions. The strength-level half is `make version-race` — use this
//! tool first to pick the candidates worth an Elo race.
//!
//! ```text
//! nnue ablate [<tsfen>] [--net FILE] [--depth D] [--time-ms T]
//! nnue ablate --positions FILE [--net FILE] [--depth D]   # one tsfen per line
//! ```

use super::{flag, load_net, num, positional};
use crate::search::params::SearchConfig;

/// Positions collected for an ablation run.
type Positions = Vec<Position>;
/// One technique-vs-baseline row.
type AblationRow<'a> = (&'a str, Aggregate);

/// Every `SearchConfig` switch, in struct order.
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

/// Turn `name` off in `cfg`. `None` when the name is unknown.
fn disable(cfg: &mut SearchConfig, name: &str) -> Option<()> {
    let field = match name {
        "razoring" => &mut cfg.razoring,
        "reverse_futility" => &mut cfg.reverse_futility,
        "futility" => &mut cfg.futility,
        "null_move_pruning" => &mut cfg.null_move_pruning,
        "static_eval_prune" => &mut cfg.static_eval_prune,
        "transposition_table" => &mut cfg.transposition_table,
        "killer_moves" => &mut cfg.killer_moves,
        "history_heuristic" => &mut cfg.history_heuristic,
        "counter_moves" => &mut cfg.counter_moves,
        "internal_iterative_deepening" => &mut cfg.internal_iterative_deepening,
        "late_move_reductions" => &mut cfg.late_move_reductions,
        "late_move_pruning" => &mut cfg.late_move_pruning,
        "capture_beam" => &mut cfg.capture_beam,
        "hash_move_extension" => &mut cfg.hash_move_extension,
        "quiescence" => &mut cfg.quiescence,
        _ => return None,
    };
    *field = false;
    Some(())
}

/// One search outcome, compared across configs.
struct Outcome {
    best: String,
    score: i32,
    nodes: u64,
}

/// Search `board` with an explicit technique config, fresh searcher: cold TT
/// and heuristics every time, so the runs are comparable and deterministic
/// (single-threaded).
fn search_cfg(board: &mut crate::Board, cfg: SearchConfig, depth: u32, time_ms: u64) -> Outcome {
    let mut searcher = crate::search::Searcher::with_config(cfg);
    let r = board.search_with(&mut searcher, depth, time_ms);
    Outcome {
        best: r.best_move.map(|m| m.to_string()).unwrap_or_default(),
        score: r.score,
        nodes: r.nodes,
    }
}

/// One position under analysis.
struct Position {
    label: String,
    board: crate::Board,
}

fn load_positions(
    args: &[String],
    net: Option<&std::sync::Arc<crate::eval::nnue::Network>>,
) -> Result<Positions, String> {
    let net = net.cloned();
    let attach = |mut board: crate::Board| {
        board.set_network(net.clone());
        board
    };
    if let Some(fen) = positional(args) {
        let board = crate::Board::from_tsfen(fen).map_err(|e| format!("bad tsfen `{fen}`: {e}"))?;
        return Ok(vec![Position {
            label: truncate(fen, 28),
            board: attach(board),
        }]);
    }
    if let Some(path) = flag(args, "--positions") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let mut positions = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let board = crate::Board::from_tsfen(line)
                .map_err(|e| format!("bad tsfen in {path}: `{line}`: {e}"))?;
            positions.push(Position {
                label: truncate(line, 28),
                board: attach(board),
            });
        }
        if positions.is_empty() {
            return Err(format!("{path} contains no positions"));
        }
        return Ok(positions);
    }
    Ok(vec![Position {
        label: "initial".to_string(),
        board: attach(crate::Board::initial()),
    }])
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

/// Running aggregate for one technique across all positions.
#[derive(Default)]
struct Aggregate {
    changed: usize,
    delta_abs_sum: i64,
    nodes_sum: u64,
    baseline_nodes_sum: u64,
    positions: usize,
}

/// # Errors
///
/// Returns an error describing the failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let depth = num(args, "--depth", 2u32)?;
    let time_ms = num(args, "--time-ms", 0u64)?;
    if depth == 0 {
        return Err("--depth must be at least 1".into());
    }
    let net = load_net(args)?;
    let mut positions = load_positions(args, net.as_ref())?;

    println!("=== Technique ablation ===");
    println!(
        "positions: {}   depth: {depth}   eval: {}   (decision-level ablation; for Elo use `make version-race`)",
        positions.len(),
        if net.is_some() {
            "NNUE"
        } else {
            "hand-crafted"
        }
    );

    let mut rows: Vec<AblationRow<'_>> = TECHNIQUES
        .iter()
        .map(|&name| (name, Aggregate::default()))
        .collect();
    rows.push(("ALL-OFF (minimal)", Aggregate::default()));

    let mut per_position: Vec<String> = Vec::new();
    let single = positions.len() == 1;
    for pos in &mut positions {
        let baseline = search_cfg(&mut pos.board, SearchConfig::production(), depth, time_ms);
        if single {
            per_position.push(format!("position: {}", pos.label));
            per_position.push(format!(
                "baseline: best {}  score {:+}  nodes {}",
                baseline.best, baseline.score, baseline.nodes
            ));
        }
        for (name, agg) in &mut rows {
            let cfg = if *name == "ALL-OFF (minimal)" {
                SearchConfig::minimal()
            } else {
                let mut cfg = SearchConfig::production();
                let _ = disable(&mut cfg, name);
                cfg
            };
            let r = search_cfg(&mut pos.board, cfg, depth, time_ms);
            agg.positions += 1;
            // MATE is ±1e6, so the delta of two i32 scores always fits i32.
            agg.delta_abs_sum += i64::from((r.score - baseline.score).abs());
            agg.nodes_sum += r.nodes;
            agg.baseline_nodes_sum += baseline.nodes;
            if r.best != baseline.best {
                agg.changed += 1;
            }
            if single {
                per_position.push(format!(
                    "no {name:<26} best {}  score {:+}  nodes {}  ({} changed)",
                    r.best,
                    r.score,
                    r.nodes,
                    if r.best == baseline.best { " " } else { "⚠" }
                ));
            }
        }
    }

    if single {
        println!();
        for line in &per_position {
            println!("{line}");
        }
    }

    print_results(&mut rows);
    Ok(())
}

/// Sort by decision impact and print the ablation table.
fn print_results(rows: &mut [AblationRow<'_>]) {
    println!();
    println!(
        "{:<30} {:>9}  {:>13}  {:>8}",
        "technique off", "changed", "avg |Δscore|", "nodes×"
    );
    // Most decision-relevant first; ties keep the struct order.
    rows.sort_by(|a, b| {
        let fa = a.1.changed as f64 / a.1.positions.max(1) as f64;
        let fb = b.1.changed as f64 / b.1.positions.max(1) as f64;
        fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (name, agg) in rows.iter() {
        if agg.positions == 0 {
            continue;
        }
        let avg_delta = agg.delta_abs_sum as f64 / agg.positions as f64;
        let nodes_ratio = if agg.baseline_nodes_sum > 0 {
            agg.nodes_sum as f64 / agg.baseline_nodes_sum as f64
        } else {
            0.0
        };
        println!(
            "{:<30} {:>4}/{}   {:>13.0}  {:>7.2}×",
            name, agg.changed, agg.positions, avg_delta, nodes_ratio
        );
    }
    println!();
    println!(
        "reading: `changed` counts positions where the best move differs from production; avg |Δscore| is in engine units (a pawn is 500, mate ±1,000,000); nodes× is the search-cost ratio."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every technique name must map to a real field, and disabling it must
    /// not touch any other switch.
    #[test]
    fn disable_flips_exactly_one_field() {
        let cfg = SearchConfig::production();
        let all_on: Vec<bool> = vec![
            cfg.razoring,
            cfg.reverse_futility,
            cfg.futility,
            cfg.null_move_pruning,
            cfg.static_eval_prune,
            cfg.transposition_table,
            cfg.killer_moves,
            cfg.history_heuristic,
            cfg.counter_moves,
            cfg.internal_iterative_deepening,
            cfg.late_move_reductions,
            cfg.late_move_pruning,
            cfg.capture_beam,
            cfg.hash_move_extension,
            cfg.quiescence,
        ];
        assert_eq!(TECHNIQUES.len(), all_on.len(), "names match the struct");
        for (i, name) in TECHNIQUES.iter().enumerate() {
            let mut cfg = SearchConfig::production();
            disable(&mut cfg, name).expect("known technique");
            let mut after: Vec<bool> = vec![
                cfg.razoring,
                cfg.reverse_futility,
                cfg.futility,
                cfg.null_move_pruning,
                cfg.static_eval_prune,
                cfg.transposition_table,
                cfg.killer_moves,
                cfg.history_heuristic,
                cfg.counter_moves,
                cfg.internal_iterative_deepening,
                cfg.late_move_reductions,
                cfg.late_move_pruning,
                cfg.capture_beam,
                cfg.hash_move_extension,
                cfg.quiescence,
            ];
            let target = after.get_mut(i).expect("index");
            assert!(!*target, "{name} must be off");
            *target = true;
            assert_eq!(after, all_on, "{name} must be the only flipped field");
        }
        assert!(disable(&mut SearchConfig::production(), "bogus").is_none());
    }

    /// End-to-end on the initial position at depth 1: baseline, 15
    /// technique rows and the ALL-OFF row, without panicking.
    #[test]
    fn run_ablates_the_initial_position() {
        let args: Vec<String> = vec![];
        run(&args).expect("ablation on the initial position");
    }
}
