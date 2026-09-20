//! Interactive command-line interface for the Taikyoku Shogi engine.
//!
//! A real user-facing CLI (not just a debug harness): play against the
//! engine, inspect positions, run searches and perft, benchmark components,
//! and toggle the evaluation backend.
//!
//! ## Coordinate notation
//! Squares are `file + rank`, following the TSFEN listing order:
//!   - files: `a`–`z` for columns 1–26, `A`–`J` for columns 27–36
//!   - rank: row number + 1, counted from the TOP (row 0 = rank 1)
//!   - a move is `from` + `to` (e.g. `f19f20`), optional `+` suffix promotes
//!   - numeric form: `18,17-19,17` (row,col 1-based)
//!   - igui (capture without moving) is written `from`=`to` (e.g. `c5c5`)
//!
//! ## Commands
//!   help                       command list
//!   new | startpos             reset to the initial position
//!   position <tsfen>           load a TSFEN position
//!   board                      print the board
//!   status                     side to move, material, result, legal count
//!   legal [<square>]           list legal moves (optionally from one square)
//!   move <mv>                  apply a move, e.g. `f19f20` or `f19f20+`
//!   undo                       take back the last move
//!   go [depth N | movetime MS] search and print the best move (no auto-apply)
//!   play <black|white|both>    engine plays the given side after your moves
//!   stop                       engine stops auto-playing
//!   eval                       static evaluation (side-to-move perspective)
//!   perft <N>                  node count from the current position
//!   bench                      micro-benchmark: movegen / eval / apply+undo
//!   perft <N>                  node count from the current position
//!   bench                      micro-benchmark: movegen / eval / apply+undo
//!   setoption depth <N>        engine search depth (default 3)
//!   setoption movetime <MS>    engine time budget per move (0 = no limit)
//!   setoption nnue <on|off>    switch evaluation backend
//!   fen                        print the current TSFEN
//!   history                    list the moves played in this session
//!   hint                       search and print the best move (= `go`)
//!   save <file>                save the game (TSFEN + move list) as JSON
//!   load <file>                load a saved game
//!   selfplay [G D PLIES MS]    engine-vs-engine games
//!   match <G> <dA> <dB> ...    match between two configs + Elo estimate
//!   sprt <elo0> <elo1> ...     SPRT test between two configs
//!   about                      engine version / build info
//!   quit
//!
//! Run: `taikyokushogi-cli` (REPL), or `taikyokushogi-cli perft 2` etc.

use std::io::{BufRead, Write};
use taikyokushogi::elo::{self, EngineConfig, MatchConfig, Sprt};
use taikyokushogi::{Board, Color, Move};

// ── Engine configuration ────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
enum EngineSide { None, Black, White, Both }

struct Options {
    depth: u32,
    movetime: u64,
    nnue: bool,
    engine_side: EngineSide,
}

impl Default for Options {
    fn default() -> Self {
        Options { depth: 3, movetime: 0, nnue: false, engine_side: EngineSide::None }
    }
}

// ── Coordinate parsing ──────────────────────────────────────────
fn file_char(col: usize) -> char {
    if col < 26 { (b'a' + col as u8) as char } else { (b'A' + (col - 26) as u8) as char }
}

fn parse_file(c: char) -> Option<usize> {
    if c.is_ascii_lowercase() { Some(c as usize - 'a' as usize) }
    else if c.is_ascii_uppercase() { Some(c as usize - 'A' as usize + 26) }
    else { None }
}

/// Algebraic `f19` or numeric `18,17` (row,col 1-based) -> (row, col).
fn parse_square(tok: &str) -> Option<(usize, usize)> {
    let tok = tok.trim();
    if let Some((r, c)) = tok.split_once(',') {
        let row: usize = r.trim().parse().ok()?;
        let col: usize = c.trim().parse().ok()?;
        return if row >= 1 && row <= 36 && col >= 1 && col <= 36 {
            Some((row - 1, col - 1))
        } else { None };
    }
    let mut chars = tok.chars();
    let file = parse_file(chars.next()?)?;
    let rank: usize = chars.as_str().parse().ok()?;
    if rank >= 1 && rank <= 36 { Some((rank - 1, file)) } else { None }
}

fn sq_name(row: usize, col: usize) -> String {
    format!("{}{}", file_char(col), row + 1)
}

fn move_name(m: &Move) -> String {
    let r = m.raw();
    let mut s = format!("{}{}", sq_name((r.from_sq / 36) as usize, (r.from_sq % 36) as usize),
                        sq_name((r.to_sq / 36) as usize, (r.to_sq % 36) as usize));
    if r.promotion { s.push('+'); }
    s
}

/// Match a user move string against the legal move list.
/// Returns Err with a helpful message on failure.
fn parse_move(board: &Board, s: &str) -> Result<Move, String> {
    let s = s.trim().trim_end_matches('-').replace('-', "");
    let s = s.trim();
    let promo_suffix = s.ends_with('+');
    let body = s.trim_end_matches('+');
    if body.is_empty() { return Err("empty move".into()); }

    let (from, to) = if body.contains(',') {
        let parts: Vec<&str> = body.splitn(4, ',').collect();
        if parts.len() != 4 { return Err(format!("numeric moves need 4 numbers: `row,col-row,col`, got `{}`", body)); }
        let mut nums = [0usize; 4];
        for (i, p) in parts.iter().enumerate() {
            nums[i] = p.trim().parse().map_err(|_| format!("bad number `{}`", p))?;
        }
        ((nums[0] - 1, nums[1] - 1), (nums[2] - 1, nums[3] - 1))
    } else {
        // algebraic: letter + digits, letter + digits
        let mut segments = Vec::new();
        let mut cur = String::new();
        for ch in body.chars() {
            if !cur.is_empty() && parse_file(ch).is_some() && cur.chars().last().map_or(false, |c| c.is_ascii_digit()) {
                segments.push(cur.clone());
                cur.clear();
            }
            cur.push(ch);
        }
        segments.push(cur);
        if segments.len() != 2 { return Err(format!("cannot parse move `{}` (expected e.g. `f19f20`)", s)); }
        (parse_square(&segments[0]).ok_or_else(|| format!("bad square `{}`", segments[0]))?,
         parse_square(&segments[1]).ok_or_else(|| format!("bad square `{}`", segments[1]))?)
    };

    let legal = board.legal_moves();
    let matching: Vec<&Move> = legal.iter().filter(|m| {
        let r = m.raw();
        let f = (r.from_sq as usize / 36, r.from_sq as usize % 36);
        let t = (r.to_sq as usize / 36, r.to_sq as usize % 36);
        f == from && t == to
    }).collect();

    if matching.is_empty() {
        // helpful hint: moves FROM that square
        let from_moves: Vec<String> = legal.iter()
            .filter(|m| {
                let r = m.raw();
                (r.from_sq as usize / 36, r.from_sq as usize % 36) == from
            })
            .take(8).map(move_name).collect();
        return Err(if from_moves.is_empty() {
            format!("no legal moves from {}", sq_name(from.0, from.1))
        } else {
            format!("`{}` is not legal. Moves from {}: {}", s, sq_name(from.0, from.1), from_moves.join(", "))
        });
    }
    let promo_variants = matching.iter().filter(|m| m.raw().promotion).count();
    if promo_variants > 0 && promo_variants < matching.len() {
        // both variants exist -> promotion is optional, suffix decides
        let want = promo_suffix;
        return matching.into_iter().find(|m| m.raw().promotion == want).cloned()
            .ok_or_else(|| format!("`{}`: add `+` to promote, remove it to keep the piece", s));
    }
    // single variant (or promo-only): accept as-is
    Ok(matching[0].clone())
}

// ── Command execution ───────────────────────────────────────────
fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = board.legal_moves();
    if depth == 1 { return moves.len() as u64; }
    let mut n = 0;
    let mut b = board.clone();
    for m in &moves {
        b.apply(m);
        n += perft(&b, depth - 1);
        b.undo();
    }
    n
}

fn cmd_go(board: &mut Board, opt: &Options) {
    let r = board.search(opt.depth, opt.movetime);
    match &r.best_move {
        Some(m) => println!(
            "bestmove {} score {} nodes {} time {}ms",
            move_name(m), r.score, r.nodes, r.time_ms
        ),
        None => println!("bestmove none (no legal moves) score {}", r.score),
    }
}

fn engine_turn(board: &mut Board, opt: &Options) -> Option<String> {
    let side = board.side_to_move();
    let engine_moves = match opt.engine_side {
        EngineSide::None => return None,
        EngineSide::Black => side == Color::Black,
        EngineSide::White => side == Color::White,
        EngineSide::Both => true,
    };
    if !engine_moves { return None; }
    taikyokushogi::set_use_nnue(opt.nnue);
    let r = board.search(opt.depth, opt.movetime);
    match r.best_move {
        Some(m) => {
            let name = move_name(&m);
            board.apply(&m);
            println!("[engine {}] {}  (score {}, nodes {}, {}ms)",
                     if side == Color::Black { "black" } else { "white" },
                     name, r.score, r.nodes, r.time_ms);
            Some(name)
        }
        None => { println!("[engine] no legal moves — side to move loses"); None }
    }
}

fn check_terminal(board: &Board) {
    if let Some(result) = board.game_result() {
        println!(">>> GAME OVER: {}", result);
    }
}

/// Per-game progress callback used by `selfplay` / `match` / `sprt`.
fn print_game_result(game: usize, result: taikyokushogi::GameResult, a_was_white: bool) {
    let winner = match result {
        taikyokushogi::GameResult::BlackWins => if a_was_white { "B" } else { "A" },
        taikyokushogi::GameResult::WhiteWins => if a_was_white { "A" } else { "B" },
        taikyokushogi::GameResult::Draw => "draw",
    };
    println!("  game {}: {}", game + 1, winner);
}

// ── Save / load ─────────────────────────────────────────────────
#[derive(serde::Serialize, serde::Deserialize)]
struct GameFile {
    /// Current position in TSFEN.
    tsfen: String,
    /// Move list of this session (informational, in CLI notation).
    #[serde(default)]
    moves: Vec<String>,
    /// Side to move ("b" / "w").
    side: String,
    /// Full-move number.
    move_number: u32,
}

fn save_game(path: &str, board: &Board, hist: &[String]) -> Result<(), String> {
    let gf = GameFile {
        tsfen: board.to_tsfen(),
        moves: hist.to_vec(),
        side: if board.side_to_move() == Color::Black { "b".into() } else { "w".into() },
        move_number: board.move_number(),
    };
    let json = serde_json::to_string_pretty(&gf).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

fn load_game(path: &str, board: &mut Board, hist: &mut Vec<String>) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let gf: GameFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    *board = Board::from_tsfen(&gf.tsfen)?;
    *hist = gf.moves;
    Ok(())
}

fn status(board: &Board, opt: &Options) {
    let stm = board.side_to_move();
    println!("side to move: {}", stm);
    println!("move number:  {}", board.move_number());
    println!("material:     {} (black-white)", board.material_score());
    println!("legal moves:  {}", board.legal_moves().len());
    println!("eval:         {}", board.evaluate());
    println!("result:       {}", board.game_result().map_or("ongoing".to_string(), |r| r.to_string()));
    println!("engine:       depth {} movetime {}ms nnue {} plays {:?}",
             opt.depth, opt.movetime, if opt.nnue { "on" } else { "off" }, opt.engine_side);
    println!("tsfen:        {}", board.to_tsfen());
}

fn execute(line: &str, board: &mut Board, opt: &mut Options, hist: &mut Vec<String>) -> bool {
    let line = line.trim();
    if line.is_empty() { return true; }
    let mut tokens = line.split_whitespace();
    let cmd = tokens.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<&str> = tokens.collect();
    match cmd.as_str() {
        "help" | "?" => { print_help(); }
        "quit" | "exit" => { return false; }
        "new" | "startpos" => { *board = Board::initial(); hist.clear(); println!("new game"); }
        "position" => {
            let tsfen = rest.join(" ");
            let tsfen = tsfen.strip_prefix("tsfen ").unwrap_or(&tsfen);
            match Board::from_tsfen(tsfen) {
                Ok(b) => { *board = b; println!("position loaded"); }
                Err(e) => println!("error: {}", e),
            }
        }
        "board" => { println!("{}", board.display()); }
        "status" => { status(board, opt); }
        "legal" => {
            let moves = board.legal_moves();
            let filtered: Vec<_> = match rest.first() {
                Some(sq) => match parse_square(sq) {
                    Some((row, col)) => moves.iter().filter(|m| {
                        let r = m.raw();
                        (r.from_sq as usize / 36, r.from_sq as usize % 36) == (row, col)
                    }).collect(),
                    None => { println!("error: bad square `{}`", sq); return true; }
                },
                None => moves.iter().collect(),
            };
            if filtered.is_empty() { println!("no legal moves"); }
            for (i, m) in filtered.iter().enumerate() {
                let r = m.raw();
                let mut extra = String::new();
                if r.captured_piece != 0 { extra.push_str(&format!(" x{}", r.captured_piece)); }
                if r.mid_piece != 0 { extra.push_str(&format!(" +mid{}", r.mid_piece)); }
                if r.range_cap && r.caps_value != 0 { extra.push_str(&format!(" rc({})", r.caps_value)); }
                if r.is_igui { extra.push_str(" igui"); }
                println!("{:>4}: {}{}", i, move_name(m), extra);
            }
        }
        "move" => {
            if rest.is_empty() { println!("error: usage `move f19f20`"); return true; }
            match parse_move(board, &rest.join(" ")) {
                Ok(m) => {
                    let name = move_name(&m);
                    board.apply(&m);
                    hist.push(name);
                    println!("played {}", hist.last().unwrap());
                    check_terminal(board);
                    if board.game_result().is_none() {
                        if let Some(em) = engine_turn(board, opt) { hist.push(em); }
                        check_terminal(board);
                    }
                }
                Err(e) => println!("error: {}", e),
            }
        }
        "undo" => {
            if board.undo() {
                hist.pop();
                println!("undone");
            } else { println!("nothing to undo"); }
        }
        "fen" => { println!("{}", board.to_tsfen()); }
        "hint" => { taikyokushogi::set_use_nnue(opt.nnue); cmd_go(board, opt); }
        "history" => {
            if hist.is_empty() { println!("no moves played yet"); }
            for (i, mv) in hist.iter().enumerate() {
                if i % 2 == 0 { print!("{}. {}", i / 2 + 1, mv); }
                else { println!(" {}", mv); }
            }
            println!();
        }
        "save" => {
            match rest.first() {
                Some(p) => match save_game(p, board, hist) {
                    Ok(()) => println!("saved to {}", p),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: usage `save <file.json>`"),
            }
        }
        "load" => {
            match rest.first() {
                Some(p) => match load_game(p, board, hist) {
                    Ok(()) => println!("loaded {} ({} moves)", p, hist.len()),
                    Err(e) => println!("error: {}", e),
                },
                None => println!("error: usage `load <file.json>`"),
            }
        }
        "selfplay" => {
            // selfplay [games] [depth] [maxplies] [movetime]
            let games: usize = rest.first().and_then(|s| s.parse().ok()).unwrap_or(2);
            let depth: u32 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(opt.depth);
            let max_plies: u32 = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
            let movetime: u64 = rest.get(3).and_then(|s| s.parse().ok()).unwrap_or(opt.movetime);
            let cfg = MatchConfig {
                games,
                a: EngineConfig { depth, time_ms: 0, nnue: false },
                b: EngineConfig { depth, time_ms: movetime, nnue: false },
                max_plies,
                on_game: Some(print_game_result),
            };
            let r = elo::run_match(&cfg);
            r.print();
        }
        "match" => {
            // match <games> <depthA> <depthB> [movetime MS] [maxplies N]
            if rest.len() < 3 {
                println!("error: usage `match <games> <depthA> <depthB> [movetime MS] [maxplies N]`");
                return true;
            }
            let games: usize = rest[0].parse().unwrap_or(10);
            let da: u32 = rest[1].parse().unwrap_or(2);
            let db: u32 = rest[2].parse().unwrap_or(3);
            let mut movetime = 0u64;
            let mut max_plies = 400u32;
            let mut it = rest[3..].iter();
            while let Some(k) = it.next() {
                match *k {
                    "movetime" => movetime = it.next().and_then(|v| v.parse().ok()).unwrap_or(0),
                    "maxplies" => max_plies = it.next().and_then(|v| v.parse().ok()).unwrap_or(400),
                    other => println!("warning: ignored `{}`", other),
                }
            }
            println!("match: {} games — A depth {} vs B depth {} (movetime {}ms, max plies {})",
                     games, da, db, movetime, max_plies);
            let cfg = MatchConfig {
                games,
                a: EngineConfig { depth: da, time_ms: 0, nnue: false },
                b: EngineConfig { depth: db, time_ms: movetime, nnue: false },
                max_plies,
                on_game: Some(print_game_result),
            };
            let r = elo::run_match(&cfg);
            r.print();
        }
        "sprt" => {
            // sprt <elo0> <elo1> [games] [depthA] [depthB] [movetime MS]
            let elo0: f64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let elo1: f64 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(10.0);
            let games: usize = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
            let da: u32 = rest.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);
            let db: u32 = rest.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
            let movetime: u64 = rest.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
            println!("SPRT: H0 ≤ {:+.0} vs H1 ≥ {:+.0}, max {} games (A depth {}, B depth {})",
                     elo0, elo1, games, da, db);
            let cfg = MatchConfig {
                games,
                a: EngineConfig { depth: da, time_ms: 0, nnue: false },
                b: EngineConfig { depth: db, time_ms: movetime, nnue: false },
                max_plies: 400,
                on_game: Some(print_game_result),
            };
            let sprt = Sprt { elo0, elo1, alpha: 0.05, beta: 0.05 };
            let r = elo::run_sprt(&cfg, &sprt);
            r.print();
        }
        "about" => {
            println!("taikyokushogi v{} — Taikyoku Shogi engine", env!("CARGO_PKG_VERSION"));
            println!("  board: 36x36 (1296 squares), 804 pieces, {} piece types", taikyokushogi::num_piece_types());
            println!("  evaluator: {} (setoption nnue on|off)", if taikyokushogi::using_nnue() { "NNUE" } else { "hand-crafted" });
        }
        "go" => {
            // inline overrides: `go depth 5` / `go movetime 1000`
            let mut d = opt.depth; let mut t = opt.movetime;
            let mut args = rest.iter();
            while let Some(k) = args.next() {
                match *k {
                    "depth" => d = args.next().and_then(|v| v.parse().ok()).unwrap_or(d),
                    "movetime" => t = args.next().and_then(|v| v.parse().ok()).unwrap_or(t),
                    other => println!("warning: ignored `{}`", other),
                }
            }
            taikyokushogi::set_use_nnue(opt.nnue);
            cmd_go(board, &Options { depth: d, movetime: t, ..*opt });
        }
        "play" => {
            opt.engine_side = match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("black") | Some("b") => EngineSide::Black,
                Some("white") | Some("w") => EngineSide::White,
                Some("both") => EngineSide::Both,
                _ => { println!("error: `play black|white|both`"); return true; }
            };
            println!("engine takes: {:?}", opt.engine_side);
            if let Some(em) = engine_turn(board, opt) { hist.push(em); }
            check_terminal(board);
        }
        "stop" => { opt.engine_side = EngineSide::None; println!("engine stopped"); }
        "eval" => { println!("eval = {}", board.evaluate()); }
        "perft" => {
            let depth: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(2);
            let t = std::time::Instant::now();
            let n = perft(board, depth);
            let secs = t.elapsed().as_secs_f64().max(1e-9);
            println!("perft({}) = {}  ({:.1}ms, {:.1} Mnps)", depth, n,
                     secs * 1000.0, n as f64 / secs / 1e6);
        }
        "bench" => {
            const ITERS: usize = 50;
            let t = std::time::Instant::now();
            for _ in 0..ITERS { let _ = board.legal_moves(); }
            let mg = t.elapsed().as_micros() as f64 / ITERS as f64;
            let t = std::time::Instant::now();
            for _ in 0..ITERS { let _ = board.evaluate(); }
            let ev = t.elapsed().as_micros() as f64 / ITERS as f64;
            let t = std::time::Instant::now();
            for _ in 0..ITERS {
                let m = board.legal_moves().into_iter().next();
                if let Some(m) = m { board.apply(&m); board.undo(); }
            }
            let au = t.elapsed().as_micros() as f64 / ITERS as f64;
            println!("movegen {:.1}us  eval {:.1}us  apply+undo {:.1}us  ({} iters each)",
                     mg, ev, au, ITERS);
        }
        "setoption" => {
            if rest.len() < 2 { println!("error: `setoption depth <N>` | `setoption movetime <MS>` | `setoption nnue <on|off>`"); return true; }
            let key = rest[0].to_ascii_lowercase();
            let val = rest[1].to_ascii_lowercase();
            match key.as_str() {
                "depth" => match val.parse() {
                    Ok(d) => { opt.depth = d; println!("depth = {}", d); }
                    Err(_) => println!("error: bad depth"),
                },
                "movetime" => match val.parse() {
                    Ok(t) => { opt.movetime = t; println!("movetime = {}ms", t); }
                    Err(_) => println!("error: bad movetime"),
                },
                "nnue" => {
                    opt.nnue = matches!(val.as_str(), "on" | "true" | "1");
                    taikyokushogi::set_use_nnue(opt.nnue);
                    println!("nnue = {}", if opt.nnue { "on" } else { "off" });
                    if opt.nnue && std::env::var_os("TAIKYOKU_NNUE_PATH").is_none() {
                        println!("warning: TAIKYOKU_NNUE_PATH not set — using random untrained weights");
                    }
                }
                other => println!("error: unknown option `{}`", other),
            }
        }
        _ => println!("unknown command `{}` — try `help`", cmd),
    }
    true
}

fn print_help() {
    println!(
"Comandos (commands):
  help | ?                    esta ayuda (this help)
  new | startpos              posición inicial (new game)
  position <tsfen>            cargar posición TSFEN
  board                       imprimir tablero
  status                      lado, material, resultado, nº de jugadas
  legal [<casilla>]           jugadas legales (opcionalmente desde una casilla)
  move <jugada>               aplicar jugada (f19f20, f19-f20, f19f20+, 19,17-20,17)
  undo                        deshacer la última jugada
  fen                         imprimir el TSFEN actual
  history                     lista de jugadas de la sesión
  go [depth N | movetime MS]  buscar y mostrar la mejor jugada (no la aplica)
  hint                        igual que `go`
  play <black|white|both>     el motor juega ese lado tras tus movimientos
  stop                        el motor deja de jugar
  eval                        evaluación estática
  perft <N>                   conteo de nodos desde la posición actual
  bench                       micro-benchmark (movegen/eval/apply+undo)
  save <archivo.json>         guardar la partida (TSFEN + jugadas, JSON)
  load <archivo.json>         cargar una partida guardada
  selfplay [G D PLIES MS]     partidas motor contra motor
  match <G> <dA> <dB> [movetime MS] [maxplies N]
                              match entre 2 configs + estimación de ELO
  sprt <elo0> <elo1> [G] [dA] [dB] [movetime MS]
                              test SPRT entre 2 configs
  setoption depth <N>         profundidad del motor (por defecto 3)
  setoption movetime <MS>     presupuesto de tiempo por jugada (0 = sin límite)
  setoption nnue <on|off>     cambiar evaluador (NNUE requiere TAIKYOKU_NNUE_PATH)
  about                       versión e info del motor
  quit | exit                 salir

Coordenadas: archivos a-z (cols 1-26) y A-J (27-36); fila+1 desde ARRIBA (fila 0 = rank 1).
La notación coincide con el orden de filas del TSFEN. El sufijo '+' promociona.");
}

fn main() {
    let mut board = Board::initial();
    let mut opt = Options::default();
    let mut hist: Vec<String> = Vec::new();

    // Non-interactive: argv joined as one command line (`&&` chains commands).
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if !argv.is_empty() {
        for line in argv.join(" ").split("&&") {
            if !execute(line, &mut board, &mut opt, &mut hist) { return; }
        }
        return;
    }

    println!("Taikyoku Shogi CLI — `help` para comandos.");
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => { if !execute(&l, &mut board, &mut opt, &mut hist) { break; } }
            Err(_) => break,
        }
    }
    let _ = std::io::stdout().flush();
}

// ============================================================
// Tests — parsing helpers and command plumbing
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── square parsing ─────────────────────────────────────────
    #[test]
    fn parse_square_algebraic() {
        assert_eq!(parse_square("a1"), Some((0, 0)));
        assert_eq!(parse_square("f19"), Some((18, 5)));
        assert_eq!(parse_square("z36"), Some((35, 25)));
        assert_eq!(parse_square("A36"), Some((35, 26))); // files 27-36 = A-J
        assert_eq!(parse_square("J36"), Some((35, 35)));
        assert_eq!(parse_square("a37"), None); // off board
        assert_eq!(parse_square("1a"), None);
        assert_eq!(parse_square(""), None);
    }

    #[test]
    fn parse_square_numeric() {
        assert_eq!(parse_square("1,1"), Some((0, 0)));
        assert_eq!(parse_square("19,6"), Some((18, 5)));
        assert_eq!(parse_square("36,36"), Some((35, 35)));
        assert_eq!(parse_square("0,1"), None); // 1-based only
        assert_eq!(parse_square("37,1"), None);
        assert_eq!(parse_square("19,"), None);
    }

    #[test]
    fn file_char_roundtrip() {
        for col in 0..36 {
            let c = file_char(col);
            assert_eq!(parse_file(c), Some(col), "col {}", col);
        }
        assert_eq!(parse_file('1'), None);
        assert_eq!(parse_file('k'), None); // only a-z / A-J are valid files
    }

    // ── move naming ────────────────────────────────────────────
    #[test]
    fn move_name_format_and_roundtrip() {
        let board = Board::initial();
        let m = board.legal_moves().into_iter().next().expect("startpos has moves");
        let name = move_name(&m);
        assert_eq!(name.len(), 8, "from(4) + to(4), got `{}`", name);
        let parsed = parse_move(&board, &name).expect("move_name must be parseable");
        assert_eq!(parsed.raw().from_sq, m.raw().from_sq);
        assert_eq!(parsed.raw().to_sq, m.raw().to_sq);
    }

    #[test]
    fn parse_move_variants() {
        let board = Board::initial();
        let m1 = parse_move(&board, "f19f20").expect("algebraic");
        let m2 = parse_move(&board, "f19-f20").expect("dashed");
        assert_eq!(m1.raw().from_sq, m2.raw().from_sq);
        assert_eq!(m1.raw().to_sq, m2.raw().to_sq);
        let m3 = parse_move(&board, "19,6-20,6").expect("numeric");
        assert_eq!(m1.raw().from_sq, m3.raw().from_sq);
        assert_eq!(m1.raw().to_sq, m3.raw().to_sq);
        // illegal / malformed
        assert!(parse_move(&board, "a1a2").is_err());
        assert!(parse_move(&board, "zzzz").is_err());
        assert!(parse_move(&board, "").is_err());
    }

    #[test]
    fn parse_move_error_is_helpful() {
        let board = Board::initial();
        let err = parse_move(&board, "a1a2").unwrap_err();
        assert!(err.contains("no legal moves from a1"), "got: {}", err);
        let err2 = parse_move(&board, "q99q99").unwrap_err();
        assert!(err2.contains("bad square"), "got: {}", err2);
    }

    // ── command execution ──────────────────────────────────────
    fn run_cmds(lines: &[&str]) -> (Board, Options, Vec<String>) {
        let mut board = Board::initial();
        let mut opt = Options::default();
        let mut hist = Vec::new();
        for l in lines {
            if !execute(l, &mut board, &mut opt, &mut hist) { break; }
        }
        (board, opt, hist)
    }

    #[test]
    fn execute_quit_stops_repl() {
        assert!(!execute("quit", &mut Board::initial(), &mut Options::default(), &mut Vec::new()));
        assert!(!execute("exit", &mut Board::initial(), &mut Options::default(), &mut Vec::new()));
        assert!(execute("help", &mut Board::initial(), &mut Options::default(), &mut Vec::new()));
    }

    #[test]
    fn execute_move_undo_and_history() {
        let (board, _, hist) = run_cmds(&["move f19f20", "move g17g18", "undo"]);
        assert_eq!(hist.len(), 1, "one move remains after undo, hist = {:?}", hist);
        assert_eq!(hist[0], "f19f20");
        // 2 plies were applied, 1 undone → move_number 2, black to move
        assert_eq!(board.move_number(), 2);
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn execute_new_clears_history() {
        let (board, _, hist) = run_cmds(&["move f19f20", "move g17g18", "new"]);
        assert!(hist.is_empty());
        assert_eq!(board.move_number(), 1);
        assert_eq!(board.piece_count(Color::Black), 402);
    }

    #[test]
    fn execute_unknown_command_is_not_fatal() {
        let (board, _, _) = run_cmds(&["definitely-not-a-command", "fen"]);
        assert_eq!(board.piece_count(Color::Black), 402);
    }

    #[test]
    fn execute_fen_command() {
        let (board, _, _) = run_cmds(&["fen"]);
        assert!(board.to_tsfen().ends_with(" b 1"));
    }

    #[test]
    fn execute_save_load_roundtrip() {
        let path = "/tmp/taikyoku_cli_test_save.json";
        let _ = std::fs::remove_file(path);
        let (mut board, mut opt, mut hist) = run_cmds(&["move f19f20", "move g17g18"]);
        assert!(execute(&format!("save {}", path), &mut board, &mut opt, &mut hist));
        // reset and load
        assert!(execute("new", &mut board, &mut opt, &mut hist));
        assert!(hist.is_empty());
        assert!(execute(&format!("load {}", path), &mut board, &mut opt, &mut hist));
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0], "f19f20");
        // board matches the saved TSFEN
        let gf: GameFile = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(board.to_tsfen(), gf.tsfen);
        assert_eq!(board.move_number(), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn execute_position_command() {
        let tsfen = Board::initial().to_tsfen();
        let (board, _, _) = run_cmds(&[&format!("position {}", tsfen)]);
        assert_eq!(board.piece_count(Color::Black), 402);
    }

    #[test]
    fn execute_bad_position_keeps_board() {
        let (board, _, _) = run_cmds(&["position garbage-tsfen"]);
        // board unchanged (still the initial position)
        assert_eq!(board.piece_count(Color::Black), 402);
        assert_eq!(board.move_number(), 1);
    }
}
