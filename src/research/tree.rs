#![allow(clippy::all)]
//! `nnue tree` — bounded static-eval tree below a position, as JSON.
//!
//! Every node is a legal move; `eval` is the static evaluation after the
//! move from the mover's point of view, and the children are the best
//! `--branching` moves of the resulting position, ranked the same way. It is
//! a *static* tree (no search) so it is fast and deterministic — use
//! `nnue multipv` for search-backed line comparison. The JSON is the data
//! layer for a visualizer (the audit's "Search Tree Visualizer": position,
//! candidate moves, replies, depth of each branch, evaluation per node,
//! pruned-by-budget nodes).
//!
//! ```text
//! nnue tree [<tsfen>] [--net FILE] [--depth D] [--branching B] [--max-nodes N] [--out FILE.json]
//! ```

use serde_json::{Value, json};

use super::{flag, load_position, num};

pub fn run(args: &[String]) -> Result<(), String> {
    let (mut board, net) = load_position(args)?;
    let depth = num(args, "--depth", 2u32)?;
    let branching = num(args, "--branching", 3usize)?;
    let max_nodes = num(args, "--max-nodes", 2000u64)?;
    let out = flag(args, "--out").map(std::string::ToString::to_string);
    if depth == 0 {
        return Err("--depth must be at least 1".into());
    }
    if branching == 0 {
        return Err("--branching must be at least 1".into());
    }

    let mut budget = max_nodes;
    let children = expand(&mut board, depth, branching, &mut budget);
    let expanded = max_nodes - budget;
    let root = json!({
        "tool": "nnue tree",
        "version": env!("CARGO_PKG_VERSION"),
        "tsfen": board.to_tsfen(),
        "side_to_move": side_name(board.side_to_move()),
        "eval_stm": board.evaluate(),
        "eval_note": "eval = static eval after the move (mover POV, a pawn is 500); eval_stm = POV of the side to move at that node",
        "eval_backend": if net.is_some() { "nnue" } else { "hand-crafted" },
        "depth": depth,
        "branching": branching,
        "nodes_expanded": expanded,
        "children": children,
    });
    let text = serde_json::to_string_pretty(&root).map_err(|e| format!("serialize tree: {e}"))?;
    match out {
        Some(path) => {
            std::fs::write(&path, text + "\n").map_err(|e| format!("cannot write {path}: {e}"))?;
            println!(
                "wrote {path} ({expanded} nodes expanded, depth {depth}, branching {branching})"
            );
        }
        None => println!("{text}"),
    }
    Ok(())
}

fn side_name(c: crate::Color) -> &'static str {
    if c == crate::Color::Black {
        "black"
    } else {
        "white"
    }
}

/// Expand one node's children: the top `branching` moves by static eval,
/// recursively until `depth_left` plies below the parent. `budget` counts
/// *expanded* nodes (each costs one move generation) and bounds the total
/// work; exhausted-budget stubs are recorded as `"pruned"` so the visualizer
/// can show them as cut branches.
fn expand(board: &mut crate::Board, depth_left: u32, branching: usize, budget: &mut u64) -> Value {
    let scored = super::static_move_scores(board);
    let mut children: Vec<Value> = Vec::new();
    for (score, mv) in scored.into_iter().take(branching) {
        if *budget == 0 {
            children.push(json!({
                "move": mv.to_string(),
                "eval": score,
                "pruned": "node budget exhausted",
            }));
            continue;
        }
        *budget -= 1;
        board.apply(&mv);
        let grandchildren = if depth_left > 1 {
            expand(board, depth_left - 1, branching, budget)
        } else {
            Value::Array(Vec::new())
        };
        let node = json!({
            "move": mv.to_string(),
            "eval": score,
            "side_to_move": side_name(board.side_to_move()),
            "eval_stm": board.evaluate(),
            "children": grandchildren,
        });
        board.undo();
        children.push(node);
    }
    Value::Array(children)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree must respect branching, depth and the node budget, and every
    /// expanded node must carry its static evaluation.
    #[test]
    fn tree_respects_branching_depth_and_budget() {
        let mut board = crate::Board::initial();
        let depth = 2u32;
        let branching = 2usize;
        let mut budget: u64 = 5;
        let children = expand(&mut board, depth, branching, &mut budget);
        let kids = children.as_array().expect("children is a JSON array");
        assert_eq!(kids.len(), 2, "branching respected at the root");
        let mut expanded = 0u64;
        for kid in kids {
            assert!(kid["eval"].is_i64(), "every child carries an eval");
            assert!(kid["move"].is_string());
            expanded += 1; // the child node itself was expanded
            let grand = kid["children"].as_array().expect("array");
            assert!(grand.len() <= branching);
            for g in grand {
                if g["pruned"].is_string() {
                    continue; // budget stub, not an expanded node
                }
                assert!(g["eval"].is_i64());
                expanded += 1;
            }
        }
        assert_eq!(
            expanded, 5,
            "1 + 2 + 2 expanded nodes before the budget ran out"
        );
        // The budget stub must be visible.
        let pruned: Vec<&serde_json::Value> = kids[1]["children"]
            .as_array()
            .expect("array")
            .iter()
            .filter(|g| g["pruned"].is_string())
            .collect();
        assert!(!pruned.is_empty(), "exhausted budget leaves pruned stubs");
        assert!(budget == 0);
    }

    /// A tiny budget on a fresh root still yields valid JSON.
    #[test]
    fn zero_budget_yields_only_pruned_stubs() {
        let mut board = crate::Board::initial();
        let mut budget: u64 = 0;
        let children = expand(&mut board, 2, 3, &mut budget);
        let kids = children.as_array().expect("array");
        assert_eq!(kids.len(), 3);
        assert!(kids.iter().all(|k| k["pruned"].is_string()));
    }
}
