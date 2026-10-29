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
    println!("{}: {:.1} us/call ({} iters)", label, us, iters);
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
    let fm = moves[0].clone();
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
