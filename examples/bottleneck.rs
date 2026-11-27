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

// Focused bottleneck micro-benchmark: per-component timing on the initial
// position, plus a fixed-depth search. Writes one line per measurement so it
// can run in background and be polled.
use std::time::Instant;
use taikyokushogi::Board;

fn bench(label: &str, iters: usize, mut f: impl FnMut()) {
    for _ in 0..iters.min(5) {
        f();
    } // warmup
    let t = Instant::now();
    for _ in 0..iters {
        f();
    }
    let us = t.elapsed().as_micros() as f64 / iters as f64;
    println!("{label}: {us:.1} us/call ({iters} iters)");
}

fn main() {
    let b = Board::initial();
    println!(
        "pieces B={} W={}",
        b.piece_count(taikyokushogi::Color::Black),
        b.piece_count(taikyokushogi::Color::White)
    );
    let moves = b.legal_moves();
    println!("legal move count: {}", moves.len());

    bench("legal_moves", 20, || {
        let b = Board::initial();
        let _ = b.legal_moves();
    });
    bench("evaluate", 100, || {
        let b = Board::initial();
        let _ = b.evaluate();
    });
    bench("material_score", 100, || {
        let b = Board::initial();
        let _ = b.material_score();
    });
    let fm = moves[0];
    bench("apply+undo", 100, || {
        let mut b = Board::initial();
        b.apply(&fm);
        b.undo();
    });
    bench("clone", 100, || {
        let b = Board::initial();
        let _ = b.clone();
    });
    // quiescence-only proxy: depth-1 search
    for depth in [1u32, 2, 3] {
        let mut b = Board::initial();
        let t = Instant::now();
        let r = b.search(depth, 0);
        let ms = t.elapsed().as_millis();
        println!(
            "search depth={} nodes={} time={}ms nps={:.0}",
            depth,
            r.nodes,
            ms,
            r.nodes as f64 / (ms as f64 / 1000.0).max(1e-9)
        );
    }
    println!("DONE");
}
