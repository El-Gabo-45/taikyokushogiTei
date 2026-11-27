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

// Exports piece metadata needed by the Python NNUE training pipeline
// (training/features.py) to a small JSON file, so that piece-type
// properties (specifically: which piece_type IDs are royal, i.e. count as
// "the king" for HalfKP feature indexing) never have to be hand-copied or
// re-derived in Python. If pieces.rs ever changes the piece set, just
// re-run this and re-export -- training/features.py always reads from
// this file rather than hardcoding anything.
//
// Usage: cargo run --release --example export_piece_metadata > training/piece_metadata.json

fn main() {
    let n = taikyokushogi::num_piece_types();
    let mut royal_types: Vec<u16> = Vec::new();
    for pt in 1..=(n as u16) {
        if taikyokushogi::is_royal_piece_type(pt) {
            royal_types.push(pt);
        }
    }

    println!("{{");
    println!("  \"num_piece_types\": {n},");
    println!(
        "  \"royal_piece_types\": [{}]",
        royal_types
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("}}");

    eprintln!("num_piece_types = {n}");
    eprintln!("royal_piece_types = {royal_types:?}");
}
