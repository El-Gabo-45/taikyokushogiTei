use std::time::Instant;
use taikyokushogi::Board;
fn main() {
    let b0 = Board::initial();
    let moves = b0.legal_moves();
    println!("legal moves from start: {}", moves.len());
    // legal_moves timing
    let t = Instant::now();
    let n = 50;
    for _ in 0..n {
        let _ = b0.legal_moves();
    }
    println!(
        "legal_moves: {:.1} us/call",
        t.elapsed().as_micros() as f64 / n as f64
    );
    // evaluate timing
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.evaluate();
    }
    println!(
        "evaluate: {:.1} us/call",
        t.elapsed().as_micros() as f64 / n as f64
    );
    // apply+undo timing
    let m = moves[0];
    let t = Instant::now();
    let n = 500;
    let mut b = Board::initial();
    for _ in 0..n {
        b.apply(&m);
        b.undo();
    }
    println!(
        "apply+undo: {:.1} us/call",
        t.elapsed().as_micros() as f64 / n as f64
    );
    // material_score
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.material_score();
    }
    println!(
        "material_score: {:.1} us/call",
        t.elapsed().as_micros() as f64 / n as f64
    );
    // clone
    let t = Instant::now();
    let n = 200;
    for _ in 0..n {
        let _ = b0.clone();
    }
    println!(
        "clone: {:.1} us/call",
        t.elapsed().as_micros() as f64 / n as f64
    );
}
