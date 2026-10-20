//! Quick component benchmark: measures movegen, eval, apply/undo in
//! isolation with tight iteration counts so it finishes in seconds.
use taikyokushogi::Board;
use std::time::Instant;

fn bench(name: &str, iters: u32, mut f: impl FnMut()) {
    // warmup
    for _ in 0..5 { f(); }
    let t = Instant::now();
    for _ in 0..iters { f(); }
    let us = t.elapsed().as_micros() as f64;
    println!("  {:22} {:>8.1} us/iter  ({} iters)", name, us / iters as f64, iters);
}

fn main() {
    let board = Board::initial();
    let n_moves = board.legal_moves().len();
    println!("initial: {} legal moves", n_moves);

    // NOTE: board setup is OUTSIDE the closure — we measure only the op.
    bench("legal_moves()", 200, || {
        let _ = board.legal_moves();
    });
    bench("evaluate()", 500, || {
        let _ = board.evaluate();
    });
    bench("material_score()", 500, || {
        let _ = board.material_score();
    });
    let mv = board.legal_moves()[0].clone();
    {
        let mut b = Board::initial();
        // warmup + preallocate history capacity like a real search would
        for _ in 0..5 { b.apply(&mv); b.undo(); }
        let t = Instant::now();
        let iters = 2000u32;
        for _ in 0..iters { b.apply(&mv); b.undo(); }
        let us = t.elapsed().as_micros() as f64;
        println!("  {:22} {:>8.1} us/iter  ({} iters)", "apply()+undo()", us / iters as f64, iters);
    }
    bench("Board::clone()", 500, || {
        let _ = board.clone();
    });
    bench("Board::initial() [setup]", 50, || {
        let _ = Board::initial();
    });
    // perft(2) — pure movegen throughput
    {
        let mut b = Board::initial();
        let t = Instant::now();
        let n = perft(&mut b, 2);
        let s = t.elapsed().as_secs_f64();
        println!("  perft(2): {} nodes in {:.2}s = {:.0} nps", n, s, n as f64 / s);
    }
}

fn perft(board: &mut Board, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = board.legal_moves();
    if depth == 1 { return moves.len() as u64; }
    let mut n = 0;
    for m in moves {
        board.apply(&m);
        n += perft(board, depth - 1);
        board.undo();
    }
    n
}

