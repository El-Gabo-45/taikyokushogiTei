use std::time::Instant;
use taikyokushogi::Board;
fn main() {
    let b = Board::initial();
    for f in ["game_result", "evaluate", "king_safety-x2", "legal_moves"] {
        let _ = f;
    }
    let t = Instant::now();
    let n = 2000;
    for _ in 0..n {
        let _ = b.game_result();
    }
    println!(
        "game_result: {:.2} us",
        t.elapsed().as_micros() as f64 / n as f64
    );
    let t = Instant::now();
    let n = 2000;
    for _ in 0..n {
        let _ = b.evaluate();
    }
    println!(
        "evaluate: {:.2} us",
        t.elapsed().as_micros() as f64 / n as f64
    );
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b.legal_moves();
    }
    println!(
        "legal_moves: {:.1} us",
        t.elapsed().as_micros() as f64 / n as f64
    );
}
