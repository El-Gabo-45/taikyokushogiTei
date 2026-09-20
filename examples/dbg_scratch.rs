use taikyokushogi::Board;

fn main() {
    let board = Board::initial();
    println!("(32,5) = {:?}  (33,4) = {:?}", board.get(32, 5).map(|p| p.to_string()), board.get(33, 4).map(|p| p.to_string()));
    let ms = board.legal_moves();
    println!("total legal = {}", ms.len());
    for m in ms.iter().take(5) {
        println!("move {} captured={:?} igui={} rc={}", m, m.captured(), m.is_igui(), m.raw().range_cap);
    }
}
