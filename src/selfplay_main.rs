use taikyokushogi::selfplay::{run_selfplay, SelfPlayConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // args: <num_games> <depth> <max_moves> <num_workers> <time_limit_ms>
    let num_games = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    let depth = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
    let max_moves = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    let num_workers = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(4);
    let time_limit_ms = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(500);

    let out_dir_for_print = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "training_data".to_string());
    println!("=== Taikyoku Shogi Self-Play ===");
    println!("Games:         {}", num_games);
    println!("Depth:         {}", depth);
    println!(
        "Max moves:     {}",
        if max_moves == 0 {
            "unlimited".to_string()
        } else {
            max_moves.to_string()
        }
    );
    println!("Workers:       {}", num_workers);
    println!("Time limit/ms: {}", time_limit_ms);
    println!("Data:          {}/", out_dir_for_print);
    println!("SQLite:        {}/games.db", out_dir_for_print);
    println!();

    // Optional flags: --net PATH  --opening-plies N  --seed N  --out DIR
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let net = flag("--net").map(|path| {
        std::sync::Arc::new(taikyokushogi::Network::load(&path).unwrap_or_else(|e| {
            eprintln!("error: cannot load network `{path}`: {e}");
            std::process::exit(2);
        }))
    });
    let out_dir = flag("--out").unwrap_or_else(|| "training_data".to_string());
    let config = SelfPlayConfig {
        opening_plies: flag("--opening-plies")
            .and_then(|v| v.parse().ok())
            .unwrap_or(8),
        seed: flag("--seed").and_then(|v| v.parse().ok()).unwrap_or(1),
        net,
        num_games,
        num_workers,
        depth,
        time_limit_ms,
        max_moves,
        output_dir: out_dir.clone(),
        save_interval: 100,
        sqlite_path: Some(format!("{}/games.db", out_dir)),
    };

    run_selfplay(Some(config));
}
