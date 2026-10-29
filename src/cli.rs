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
//!   go [depth N | movetime MS | threads N]
//!                        search and print the best move (no auto-apply)
//!   play <black|white|both>    engine plays the given side after your moves
//!   stop                       engine stops auto-playing
//!   eval                       static evaluation (side-to-move perspective)
//!   perft <N>                  node count from the current position
//!   bench                      micro-benchmark: movegen / eval / apply+undo
//!   setoption depth <N>        engine search depth (default 3)
//!   setoption movetime <MS>    engine time budget per move (0 = no limit)
//!   setoption threads <N>      Lazy SMP workers (1..=64, default 1)
//!   setoption hash <MiB>       transposition table size in MiB
//!   setoption nnue <on|off>    switch evaluation backend
//!   fen                        print the current TSFEN
//!   history                    list the moves played in this session
//!   hint                       search and print the best move (= `go`)
//!   save <file>                save the game (TSFEN + move list) as JSON
//!   load <file>                load a saved game
//!   selfplay [G D PLIES MS] [threadsA N] [threadsB N]
//!                        engine-vs-engine games (per-side Lazy SMP workers)
//!   match <G> <dA> <dB> [movetime MS] [maxplies N] [threadsA N] [threadsB N]
//!                        match between two configs + Elo estimate
//!   sprt <elo0> <elo1> [G] [dA] [dB] [MS] [threadsA N] [threadsB N]
//!                        SPRT test between two configs
//!   about                      engine version / build info
//!   quit
//!
//! ## Command-line flags (non-interactive)
//! ```
//! taikyokushogi-cli [--depth N] [--movetime MS] [--threads N] [--hash MiB]
//!                   [--position <tsfen>|startpos] [--go | --perft N] [COMMANDS...]
//! ```
//! Remaining arguments are interactive commands (`&&`-chained), e.g.
//! `taikyokushogi-cli --threads 4 "position <tsfen>" && go depth 6`.
//! Run `taikyokushogi-cli --help` for the full usage.
//!
//! Run: `taikyokushogi-cli` (REPL), or `taikyokushogi-cli perft 2` etc.

use std::io::{BufRead, Write};
use taikyokushogi::elo::{self, EngineConfig, MatchConfig, Sprt};
use taikyokushogi::search::params::SearchConfig;
use taikyokushogi::{Board, Color, Move};

// ── Engine configuration ────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
enum EngineSide {
    None,
    Black,
    White,
    Both,
}

struct Options {
    depth: u32,
    movetime: u64,
    nnue: bool,
    engine_side: EngineSide,
    /// Lazy SMP workers used by `go` / the engine's own turns (1 = plain
    /// single-threaded search).
    threads: usize,
    /// Technique on/off switches — every search technique can be independently
    /// enabled/disabled for testing, A/B experiments, or precise ablation
    /// studies. Defaults to production (all on).
    cfg: SearchConfig,
}

impl Options {
    /// Build an `Options` with the given `SearchConfig`.
    fn with_config(cfg: SearchConfig) -> Self {
        Options {
            depth: 3,
            movetime: 0,
            nnue: false,
            engine_side: EngineSide::None,
            threads: taikyokushogi::search::DEFAULT_THREADS,
            cfg,
        }
    }

    /// Return a `Searcher` configured with this options' technique flags.
    fn searcher(&self) -> taikyokushogi::search::Searcher {
        taikyokushogi::search::Searcher::with_config(self.cfg)
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::with_config(SearchConfig::production())
    }
}

// ── Coordinate parsing ──────────────────────────────────────────
fn file_char(col: usize) -> char {
    if col < 26 {
        (b'a' + col as u8) as char
    } else {
        (b'A' + (col - 26) as u8) as char
    }
}

fn parse_file(c: char) -> Option<usize> {
    if c.is_ascii_lowercase() {
        let v = c as usize - 'a' as usize;
        if v < 26 {
            Some(v)
        } else {
            None
        }
    } else if c.is_ascii_uppercase() {
        let v = c as usize - 'A' as usize;
        if v < 10 {
            Some(v + 26)
        } else {
            None
        }
    } else {
        None
    }
}

/// Algebraic `f19` or numeric `18,17` (row,col 1-based) -> (row, col).
fn parse_square(tok: &str) -> Option<(usize, usize)> {
    let tok = tok.trim();
    if let Some((r, c)) = tok.split_once(',') {
        let row: usize = r.trim().parse().ok()?;
        let col: usize = c.trim().parse().ok()?;
        return if (1..=36).contains(&row) && (1..=36).contains(&col) {
            Some((row - 1, col - 1))
        } else {
            None
        };
    }
    let mut chars = tok.chars();
    let file = parse_file(chars.next()?)?;
    let rank: usize = chars.as_str().parse().ok()?;
    if (1..=36).contains(&rank) {
        Some((rank - 1, file))
    } else {
        None
    }
}

fn sq_name(row: usize, col: usize) -> String {
    format!("{}{}", file_char(col), row + 1)
}

fn move_name(m: &Move) -> String {
    let r = m.raw();
    let mut s = format!(
        "{}{}",
        sq_name((r.from_sq / 36) as usize, (r.from_sq % 36) as usize),
        sq_name((r.to_sq / 36) as usize, (r.to_sq % 36) as usize)
    );
    if r.promotion {
        s.push('+');
    }
    s
}

/// Match a user move string against the legal move list.
/// Returns Err with a helpful message on failure.
fn parse_move(board: &Board, s: &str) -> Result<Move, String> {
    let s = s.trim();
    let promo_suffix = s.ends_with('+');
    let body = s.trim_end_matches('+').trim();
    if body.is_empty() {
        return Err("empty move".into());
    }

    let (from, to) = if body.contains(',') {
        // numeric form: `row,col-row,col` (also tolerate spaces instead of the dash)
        let cleaned = body.replace('-', " ");
        let nums: Vec<usize> = cleaned
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|p| !p.is_empty())
            .map(|p| p.trim().parse().map_err(|_| format!("bad number `{}`", p)))
            .collect::<Result<_, _>>()?;
        if nums.len() != 4 {
            return Err(format!(
                "numeric moves need 4 numbers: `row,col-row,col`, got `{}`",
                body
            ));
        }
        for n in &nums {
            if *n < 1 || *n > 36 {
                return Err(format!("square `{}` is off the 36x36 board", n));
            }
        }
        ((nums[0] - 1, nums[1] - 1), (nums[2] - 1, nums[3] - 1))
    } else {
        // algebraic: letter + digits, letter + digits (dashes ignored)
        let body = body.replace('-', "");
        // algebraic: letter + digits, letter + digits
        let mut segments = Vec::new();
        let mut cur = String::new();
        for ch in body.chars() {
            if !cur.is_empty()
                && parse_file(ch).is_some()
                && cur.chars().last().is_some_and(|c| c.is_ascii_digit())
            {
                segments.push(cur.clone());
                cur.clear();
            }
            cur.push(ch);
        }
        segments.push(cur);
        if segments.len() != 2 {
            return Err(format!(
                "cannot parse move `{}` (expected e.g. `f19f20`)",
                s
            ));
        }
        (
            parse_square(&segments[0]).ok_or_else(|| format!("bad square `{}`", segments[0]))?,
            parse_square(&segments[1]).ok_or_else(|| format!("bad square `{}`", segments[1]))?,
        )
    };

    let legal = board.legal_moves();
    let matching: Vec<&Move> = legal
        .iter()
        .filter(|m| {
            let r = m.raw();
            let f = (r.from_sq as usize / 36, r.from_sq as usize % 36);
            let t = (r.to_sq as usize / 36, r.to_sq as usize % 36);
            f == from && t == to
        })
        .collect();

    if matching.is_empty() {
        // helpful hint: moves FROM that square
        let from_moves: Vec<String> = legal
            .iter()
            .filter(|m| {
                let r = m.raw();
                (r.from_sq as usize / 36, r.from_sq as usize % 36) == from
            })
            .take(8)
            .map(move_name)
            .collect();
        return Err(if from_moves.is_empty() {
            format!("no legal moves from {}", sq_name(from.0, from.1))
        } else {
            format!(
                "`{}` is not legal. Moves from {}: {}",
                s,
                sq_name(from.0, from.1),
                from_moves.join(", ")
            )
        });
    }
    let promo_variants = matching.iter().filter(|m| m.raw().promotion).count();
    if promo_variants > 0 && promo_variants < matching.len() {
        // both variants exist -> promotion is optional, suffix decides
        let want = promo_suffix;
        return matching
            .into_iter()
            .find(|m| m.raw().promotion == want)
            .cloned()
            .ok_or_else(|| format!("`{}`: add `+` to promote, remove it to keep the piece", s));
    }
    // single variant (or promo-only): accept as-is
    Ok(*matching[0])
}

// ── Command execution ───────────────────────────────────────────
fn perft(board: &Board, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = board.legal_moves();
    if depth == 1 {
        return moves.len() as u64;
    }
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
    let mut searcher = opt.searcher();
    let r = board.search_with(&mut searcher, opt.depth, opt.movetime);
    match &r.best_move {
        Some(m) => println!(
            "bestmove {} score {} nodes {} time {}ms",
            move_name(m),
            r.score,
            r.nodes,
            r.time_ms
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
    if !engine_moves {
        return None;
    }
    taikyokushogi::set_use_nnue(opt.nnue);
    let mut searcher = opt.searcher();
    let r = board.search_with(&mut searcher, opt.depth, opt.movetime);
    match r.best_move {
        Some(m) => {
            let name = move_name(&m);
            board.apply(&m);
            println!(
                "[engine {}] {}  (score {}, nodes {}, {}ms)",
                if side == Color::Black {
                    "black"
                } else {
                    "white"
                },
                name,
                r.score,
                r.nodes,
                r.time_ms
            );
            Some(name)
        }
        None => {
            println!("[engine] no legal moves — side to move loses");
            None
        }
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
        taikyokushogi::GameResult::BlackWins => {
            if a_was_white {
                "B"
            } else {
                "A"
            }
        }
        taikyokushogi::GameResult::WhiteWins => {
            if a_was_white {
                "A"
            } else {
                "B"
            }
        }
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
        side: if board.side_to_move() == Color::Black {
            "b".into()
        } else {
            "w".into()
        },
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

/// Keywords shared by the `selfplay`, `match` and `sprt` commands (from the
/// point where their positional arguments end):
/// `movetime <MS>`, `maxplies <N>`, `threadsA <N>`, `threadsB <N>` — all
/// case-insensitive, unknown keywords warn and are ignored.
///
/// `movetime` / `max_plies` stay `None` when absent so every command keeps
/// its own fallback. `threads*` default to 1 (plain single-threaded search).
struct MatchKeywords {
    movetime: Option<u64>,
    max_plies: Option<u32>,
    threads_a: usize,
    threads_b: usize,
}

fn parse_match_keywords(tokens: &[&str]) -> MatchKeywords {
    let mut kw = MatchKeywords {
        movetime: None,
        max_plies: None,
        threads_a: 1,
        threads_b: 1,
    };
    let mut it = tokens.iter();
    while let Some(k) = it.next() {
        match k.to_ascii_lowercase().as_str() {
            "movetime" => kw.movetime = it.next().and_then(|v| v.parse().ok()),
            "maxplies" => kw.max_plies = it.next().and_then(|v| v.parse().ok()),
            "threadsa" => match it.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n >= 1 => kw.threads_a = n,
                Some(_) => println!("error: threads must be >= 1 (keeping {})", kw.threads_a),
                None => println!("error: usage `threadsA <N>`"),
            },
            "threadsb" => match it.next().and_then(|v| v.parse::<usize>().ok()) {
                Some(n) if n >= 1 => kw.threads_b = n,
                Some(_) => println!("error: threads must be >= 1 (keeping {})", kw.threads_b),
                None => println!("error: usage `threadsB <N>`"),
            },
            other => println!("warning: ignored `{}`", other),
        }
    }
    kw
}

fn status(board: &Board, opt: &Options) {
    let stm = board.side_to_move();
    println!("side to move: {}", stm);
    println!("move number:  {}", board.move_number());
    println!("material:     {} (black-white)", board.material_score());
    println!("legal moves:  {}", board.legal_moves().len());
    println!("eval:         {}", board.evaluate());
    println!(
        "result:       {}",
        board
            .game_result()
            .map_or("ongoing".to_string(), |r| r.to_string())
    );
    println!(
        "engine:       depth {} movetime {}ms threads {} nnue {} hash {}MiB plays {:?}",
        opt.depth,
        opt.movetime,
        opt.threads,
        if opt.nnue { "on" } else { "off" },
        taikyokushogi::search::hash_mb(),
        opt.engine_side
    );
    println!("tsfen:        {}", board.to_tsfen());
}

fn execute(line: &str, board: &mut Board, opt: &mut Options, hist: &mut Vec<String>) -> bool {
    let line = line.trim();
    if line.is_empty() {
        return true;
    }
    let mut tokens = line.split_whitespace();
    let cmd = tokens.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<&str> = tokens.collect();
    match cmd.as_str() {
        "help" | "?" => {
            print_help();
        }
        "quit" | "exit" => {
            return false;
        }
        "new" | "startpos" => {
            *board = Board::initial();
            hist.clear();
            println!("new game");
        }
        "position" => {
            let tsfen = rest.join(" ");
            let tsfen = tsfen.strip_prefix("tsfen ").unwrap_or(&tsfen);
            match Board::from_tsfen(tsfen) {
                Ok(b) => {
                    *board = b;
                    println!("position loaded");
                }
                Err(e) => println!("error: {}", e),
            }
        }
        "board" => {
            println!("{}", board.display());
        }
        "status" => {
            status(board, opt);
        }
        "legal" => {
            let moves = board.legal_moves();
            let filtered: Vec<_> = match rest.first() {
                Some(sq) => match parse_square(sq) {
                    Some((row, col)) => moves
                        .iter()
                        .filter(|m| {
                            let r = m.raw();
                            (r.from_sq as usize / 36, r.from_sq as usize % 36) == (row, col)
                        })
                        .collect(),
                    None => {
                        println!("error: bad square `{}`", sq);
                        return true;
                    }
                },
                None => moves.iter().collect(),
            };
            if filtered.is_empty() {
                println!("no legal moves");
            }
            for (i, m) in filtered.iter().enumerate() {
                let r = m.raw();
                let mut extra = String::new();
                if r.captured_piece != 0 {
                    extra.push_str(&format!(" x{}", r.captured_piece));
                }
                if r.mid_piece != 0 {
                    extra.push_str(&format!(" +mid{}", r.mid_piece));
                }
                if r.range_cap && r.caps_value != 0 {
                    extra.push_str(&format!(" rc({})", r.caps_value));
                }
                if r.is_igui {
                    extra.push_str(" igui");
                }
                println!("{:>4}: {}{}", i, move_name(m), extra);
            }
        }
        "move" => {
            if rest.is_empty() {
                println!("error: usage `move f19f20`");
                return true;
            }
            match parse_move(board, &rest.join(" ")) {
                Ok(m) => {
                    let name = move_name(&m);
                    board.apply(&m);
                    hist.push(name);
                    println!("played {}", hist.last().unwrap());
                    check_terminal(board);
                    if board.game_result().is_none() {
                        if let Some(em) = engine_turn(board, opt) {
                            hist.push(em);
                        }
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
            } else {
                println!("nothing to undo");
            }
        }
        "fen" => {
            println!("{}", board.to_tsfen());
        }
        "hint" => {
            taikyokushogi::set_use_nnue(opt.nnue);
            cmd_go(board, opt);
        }
        "history" => {
            if hist.is_empty() {
                println!("no moves played yet");
            }
            for (i, mv) in hist.iter().enumerate() {
                if i % 2 == 0 {
                    print!("{}. {}", i / 2 + 1, mv);
                } else {
                    println!(" {}", mv);
                }
            }
            println!();
        }
        "save" => match rest.first() {
            Some(p) => match save_game(p, board, hist) {
                Ok(()) => println!("saved to {}", p),
                Err(e) => println!("error: {}", e),
            },
            None => println!("error: usage `save <file.json>`"),
        },
        "load" => match rest.first() {
            Some(p) => match load_game(p, board, hist) {
                Ok(()) => println!("loaded {} ({} moves)", p, hist.len()),
                Err(e) => println!("error: {}", e),
            },
            None => println!("error: usage `load <file.json>`"),
        },
        "selfplay" => {
            // selfplay [games] [depth] [maxplies] [movetime] [threadsA N] [threadsB N]
            let games: usize = rest.first().and_then(|s| s.parse().ok()).unwrap_or(2);
            let depth: u32 = rest
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(opt.depth);
            let kw = parse_match_keywords(&rest[4.min(rest.len())..]);
            let max_plies: u32 = kw
                .max_plies
                .or_else(|| rest.get(2).and_then(|s| s.parse().ok()))
                .unwrap_or(200);
            let movetime: u64 = kw
                .movetime
                .or_else(|| rest.get(3).and_then(|s| s.parse().ok()))
                .unwrap_or(opt.movetime);
            let cfg = MatchConfig {
                games,
                a: EngineConfig {
                    depth,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_a,
                },
                b: EngineConfig {
                    depth,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_b,
                },
                max_plies,
                on_game: Some(print_game_result),
            };
            let r = elo::run_match(&cfg);
            r.print();
        }
        "match" => {
            // match <games> <depthA> <depthB> [movetime MS] [maxplies N] [threadsA N] [threadsB N]
            if rest.len() < 3 {
                println!("error: usage `match <games> <depthA> <depthB> [movetime MS] [maxplies N] [threadsA N] [threadsB N]`");
                return true;
            }
            let games: usize = rest[0].parse().unwrap_or(10);
            let da: u32 = rest[1].parse().unwrap_or(2);
            let db: u32 = rest[2].parse().unwrap_or(3);
            let kw = parse_match_keywords(&rest[3..]);
            let movetime = kw.movetime.unwrap_or(0);
            let max_plies = kw.max_plies.unwrap_or(400);
            println!("match: {} games — A depth {} ({} threads) vs B depth {} ({} threads) (movetime {}ms, max plies {})",
                     games, da, kw.threads_a, db, kw.threads_b, movetime, max_plies);
            let cfg = MatchConfig {
                games,
                a: EngineConfig {
                    depth: da,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_a,
                },
                b: EngineConfig {
                    depth: db,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_b,
                },
                max_plies,
                on_game: Some(print_game_result),
            };
            let r = elo::run_match(&cfg);
            r.print();
        }
        "sprt" => {
            // sprt <elo0> <elo1> [games] [depthA] [depthB] [movetime MS] [threadsA N] [threadsB N]
            let elo0: f64 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let elo1: f64 = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(10.0);
            let games: usize = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
            let da: u32 = rest.get(3).and_then(|s| s.parse().ok()).unwrap_or(2);
            let db: u32 = rest.get(4).and_then(|s| s.parse().ok()).unwrap_or(3);
            let kw = parse_match_keywords(&rest[6.min(rest.len())..]);
            let movetime: u64 = kw
                .movetime
                .or_else(|| rest.get(5).and_then(|s| s.parse().ok()))
                .unwrap_or(0);
            let max_plies = kw.max_plies.unwrap_or(400);
            println!("SPRT: H0 ≤ {:+.0} vs H1 ≥ {:+.0}, max {} games (A depth {} {} threads, B depth {} {} threads)",
                     elo0, elo1, games, da, kw.threads_a, db, kw.threads_b);
            let cfg = MatchConfig {
                games,
                a: EngineConfig {
                    depth: da,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_a,
                },
                b: EngineConfig {
                    depth: db,
                    time_ms: movetime,
                    nnue: false,
                    threads: kw.threads_b,
                },
                max_plies,
                on_game: Some(print_game_result),
            };
            let sprt = Sprt {
                elo0,
                elo1,
                alpha: 0.05,
                beta: 0.05,
            };
            let r = elo::run_sprt(&cfg, &sprt);
            r.print();
        }
        "about" => {
            println!(
                "taikyokushogi v{} — Taikyoku Shogi engine",
                env!("CARGO_PKG_VERSION")
            );
            println!(
                "  board: 36x36 (1296 squares), 804 pieces, {} piece types",
                taikyokushogi::num_piece_types()
            );
            println!(
                "  evaluator: {} (setoption nnue on|off)",
                if taikyokushogi::using_nnue() {
                    "NNUE"
                } else {
                    "hand-crafted"
                }
            );
            println!("  search: threads {}..={} (setoption threads <N>), hash {}MiB (setoption hash <MiB>)",
                     taikyokushogi::search::DEFAULT_THREADS, taikyokushogi::search::MAX_THREADS,
                     taikyokushogi::search::hash_mb());
        }
        "go" => {
            // inline overrides: `go depth 5` / `go movetime 1000` / `go threads 4`
            let mut d = opt.depth;
            let mut t = opt.movetime;
            let mut th = opt.threads;
            let mut args = rest.iter();
            while let Some(k) = args.next() {
                match *k {
                    "depth" => d = args.next().and_then(|v| v.parse().ok()).unwrap_or(d),
                    "movetime" => t = args.next().and_then(|v| v.parse().ok()).unwrap_or(t),
                    "threads" => th = args.next().and_then(|v| v.parse().ok()).unwrap_or(th),
                    other => println!("warning: ignored `{}`", other),
                }
            }
            taikyokushogi::set_use_nnue(opt.nnue);
            cmd_go(
                board,
                &Options {
                    depth: d,
                    movetime: t,
                    threads: th,
                    ..*opt
                },
            );
        }
        "play" => {
            opt.engine_side = match rest.first().map(|s| s.to_ascii_lowercase()).as_deref() {
                Some("black") | Some("b") => EngineSide::Black,
                Some("white") | Some("w") => EngineSide::White,
                Some("both") => EngineSide::Both,
                _ => {
                    println!("error: `play black|white|both`");
                    return true;
                }
            };
            println!("engine takes: {:?}", opt.engine_side);
            if let Some(em) = engine_turn(board, opt) {
                hist.push(em);
            }
            check_terminal(board);
        }
        "stop" => {
            opt.engine_side = EngineSide::None;
            println!("engine stopped");
        }
        "eval" => {
            println!("eval = {}", board.evaluate());
        }
        "perft" => {
            let depth: u32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(2);
            let t = std::time::Instant::now();
            let n = perft(board, depth);
            let secs = t.elapsed().as_secs_f64().max(1e-9);
            println!(
                "perft({}) = {}  ({:.1}ms, {:.1} Mnps)",
                depth,
                n,
                secs * 1000.0,
                n as f64 / secs / 1e6
            );
        }
        "bench" => {
            const ITERS: usize = 50;
            let t = std::time::Instant::now();
            for _ in 0..ITERS {
                let _ = board.legal_moves();
            }
            let mg = t.elapsed().as_micros() as f64 / ITERS as f64;
            let t = std::time::Instant::now();
            for _ in 0..ITERS {
                let _ = board.evaluate();
            }
            let ev = t.elapsed().as_micros() as f64 / ITERS as f64;
            let t = std::time::Instant::now();
            for _ in 0..ITERS {
                let m = board.legal_moves().into_iter().next();
                if let Some(m) = m {
                    board.apply(&m);
                    board.undo();
                }
            }
            let au = t.elapsed().as_micros() as f64 / ITERS as f64;
            println!(
                "movegen {:.1}us  eval {:.1}us  apply+undo {:.1}us  ({} iters each)",
                mg, ev, au, ITERS
            );
        }
        "setoption" => {
            if rest.len() < 2 {
                println!("error: `setoption depth <N>` | `setoption movetime <MS>` | `setoption threads <N>` | `setoption hash <MiB>` | `setoption nnue <on|off>`");
                return true;
            }
            let key = rest[0].to_ascii_lowercase();
            let val = rest[1].to_ascii_lowercase();
            match key.as_str() {
                "depth" => match val.parse() {
                    Ok(d) => {
                        opt.depth = d;
                        println!("depth = {}", d);
                    }
                    Err(_) => println!("error: bad depth"),
                },
                "movetime" => match val.parse() {
                    Ok(t) => {
                        opt.movetime = t;
                        println!("movetime = {}ms", t);
                    }
                    Err(_) => println!("error: bad movetime"),
                },
                "threads" => match val.parse::<usize>() {
                    Ok(n) if n >= 1 => {
                        let capped = n.min(taikyokushogi::search::MAX_THREADS);
                        if capped != n {
                            println!("warning: threads capped at {}", capped);
                        }
                        opt.threads = capped;
                        println!("threads = {}", opt.threads);
                    }
                    Ok(_) => println!("error: threads must be >= 1"),
                    Err(_) => println!("error: bad threads"),
                },
                "hash" => match val.parse::<usize>() {
                    Ok(mb) => {
                        let actual = taikyokushogi::set_hash_mb(mb);
                        println!("hash = {}MiB", actual);
                    }
                    Err(_) => println!("error: bad hash size"),
                },
                "nnue" => {
                    opt.nnue = matches!(val.as_str(), "on" | "true" | "1");
                    taikyokushogi::set_use_nnue(opt.nnue);
                    println!("nnue = {}", if opt.nnue { "on" } else { "off" });
                    if opt.nnue && std::env::var_os("TAIKYOKU_NNUE_PATH").is_none() {
                        println!(
                            "warning: TAIKYOKU_NNUE_PATH not set — using random untrained weights"
                        );
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
  go [depth N | movetime MS | threads N]
                              buscar y mostrar la mejor jugada (no la aplica)
  hint                        igual que `go`
  play <black|white|both>     el motor juega ese lado tras tus movimientos
  stop                        el motor deja de jugar
  eval                        evaluación estática
  perft <N>                   conteo de nodos desde la posición actual
  bench                       micro-benchmark (movegen/eval/apply+undo)
  save <archivo.json>         guardar la partida (TSFEN + jugadas, JSON)
  load <archivo.json>         cargar una partida guardada
  selfplay [G D PLIES MS] [threadsA N] [threadsB N]
                              partidas motor contra motor (hilos SMP por lado)
  match <G> <dA> <dB> [movetime MS] [maxplies N] [threadsA N] [threadsB N]
                              match entre 2 configs + estimación de ELO
  sprt <elo0> <elo1> [G] [dA] [dB] [MS] [threadsA N] [threadsB N]
                              test SPRT entre 2 configs
  setoption depth <N>         profundidad del motor (por defecto 3)
  setoption movetime <MS>     presupuesto de tiempo por jugada (0 = sin límite)
  setoption threads <N>       hilos Lazy SMP (1..=64, por defecto 1)
  setoption hash <MiB>        tamaño de la tabla de transposiciones
  setoption nnue <on|off>     cambiar evaluador (NNUE requiere TAIKYOKU_NNUE_PATH)
  about                       versión e info del motor
  quit | exit                 salir

Coordenadas: archivos a-z (cols 1-26) y A-J (27-36); fila+1 desde ARRIBA (fila 0 = rank 1).
La notación coincide con el orden de filas del TSFEN. El sufijo '+' promociona."
    );
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}\n", e);
            eprintln!("{}", usage());
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{}", usage());
        return;
    }

    let mut board = Board::initial();

    // Build SearchConfig from CLI toggles: production default, then disable
    // any technique the user explicitly turned off.
    let mut cfg = SearchConfig::production();
    if args.no_razoring.unwrap_or(false) {
        cfg.razoring = false;
    }
    if args.no_rfp.unwrap_or(false) {
        cfg.reverse_futility = false;
    }
    if args.no_futility.unwrap_or(false) {
        cfg.futility = false;
    }
    if args.no_null_move.unwrap_or(false) {
        cfg.null_move_pruning = false;
    }
    if args.no_probcut.unwrap_or(false) {
        cfg.probcut = false;
    }
    if args.no_tt.unwrap_or(false) {
        cfg.transposition_table = false;
    }
    if args.no_killers.unwrap_or(false) {
        cfg.killer_moves = false;
    }
    if args.no_history.unwrap_or(false) {
        cfg.history_heuristic = false;
    }
    if args.no_counter.unwrap_or(false) {
        cfg.counter_moves = false;
    }
    if args.no_iid.unwrap_or(false) {
        cfg.internal_iterative_deepening = false;
    }
    if args.no_lmr.unwrap_or(false) {
        cfg.late_move_reductions = false;
    }
    if args.no_lmp.unwrap_or(false) {
        cfg.late_move_pruning = false;
    }
    if args.no_beam.unwrap_or(false) {
        cfg.capture_beam = false;
    }
    if args.no_hash_ext.unwrap_or(false) {
        cfg.hash_move_extension = false;
    }
    if args.no_quiescence.unwrap_or(false) {
        cfg.quiescence = false;
    }

    let mut opt = Options::with_config(cfg);
    let mut hist: Vec<String> = Vec::new();

    if let Some(mb) = args.hash_mb {
        println!("hash = {}MiB", taikyokushogi::set_hash_mb(mb));
    }
    if let Some(d) = args.depth {
        opt.depth = d;
    }
    if let Some(t) = args.movetime {
        opt.movetime = t;
    }
    if let Some(n) = args.threads {
        let capped = n.min(taikyokushogi::search::MAX_THREADS);
        if capped != n {
            eprintln!("warning: threads capped at {}", capped);
        }
        opt.threads = capped;
    }
    if let Some(pos) = &args.position {
        if pos == "startpos" {
            board = Board::initial();
        } else {
            match Board::from_tsfen(pos) {
                Ok(b) => board = b,
                Err(e) => {
                    eprintln!("error: bad position: {}", e);
                    std::process::exit(2);
                }
            }
        }
    }

    // Traditional command line: remaining tokens joined as one command line
    // (`&&` chains commands) — exactly the pre-flags behaviour.
    let mut ran_command = false;
    if !args.commands.is_empty() {
        ran_command = true;
        for line in args.commands.join(" ").split("&&") {
            if !execute(line, &mut board, &mut opt, &mut hist) {
                return;
            }
        }
    }

    if let Some(n) = args.perft {
        let t = std::time::Instant::now();
        let nodes = perft(&board, n);
        let secs = t.elapsed().as_secs_f64().max(1e-9);
        println!(
            "perft({}) = {}  ({:.1}ms, {:.1} Mnps)",
            n,
            nodes,
            secs * 1000.0,
            nodes as f64 / secs / 1e6
        );
        return;
    }
    if args.go {
        taikyokushogi::set_use_nnue(opt.nnue);
        cmd_go(&mut board, &opt);
        return;
    }
    if ran_command {
        return;
    }

    println!("Taikyoku Shogi CLI — `help` para comandos.");
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(l) => {
                if !execute(&l, &mut board, &mut opt, &mut hist) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = std::io::stdout().flush();
}
// ── Command-line arguments (scripting / non-interactive use) ────
/// Parsed argv. Kept out of `main` so the flag grammar is unit-testable.
#[derive(Debug, Default, PartialEq)]
struct CliArgs {
    depth: Option<u32>,
    movetime: Option<u64>,
    threads: Option<usize>,
    hash_mb: Option<usize>,
    position: Option<String>,
    go: bool,
    perft: Option<u32>,
    help: bool,
    /// Technique toggles — `--no-<technique>` disables that technique.
    /// All start as `None` meaning "use the default (production) config";
    /// `Some(false)` means explicitly disabled.
    no_razoring: Option<bool>,
    no_rfp: Option<bool>,
    no_futility: Option<bool>,
    no_null_move: Option<bool>,
    no_probcut: Option<bool>,
    no_tt: Option<bool>,
    no_killers: Option<bool>,
    no_history: Option<bool>,
    no_counter: Option<bool>,
    no_iid: Option<bool>,
    no_lmr: Option<bool>,
    no_lmp: Option<bool>,
    no_beam: Option<bool>,
    no_hash_ext: Option<bool>,
    no_quiescence: Option<bool>,
    /// Remaining tokens: interactive commands, joined with spaces and
    /// `&&`-chained exactly like the pre-flags argv handling.
    commands: Vec<String>,
}

fn usage() -> &'static str {
    "\
taikyokushogi-cli — Taikyoku Shogi engine

USAGE:
    taikyokushogi-cli [OPTIONS] [COMMANDS...]

OPTIONS:
    -d, --depth <N>        default search depth in plies (default 3)
    -t, --movetime <MS>    time budget per move in ms (0 = none)
    -j, --threads <N>      Lazy SMP workers, 1..=64 (default 1)
        --hash <MiB>       transposition table size in MiB
    -p, --position <TSFEN> start from this position (`startpos` = initial board;
                           quote a TSFEN so it arrives as one argument)
        --go               run one search, print `bestmove ...`, exit
        --perft <N>        print perft(N) for the position, exit
    -h, --help             show this help

TECHNIQUE TOGGLES (ablation / testing — disable one or more):
        --no-razoring      disable razoring pruning
        --no-rfp           disable reverse futility pruning
        --no-futility      disable futility pruning
        --no-null-move     disable null-move pruning
        --no-probcut       disable ProbCut pruning
        --no-tt            disable transposition table
        --no-killers       disable killer moves
        --no-history       disable butterfly history
        --no-counter       disable counter moves
        --no-iid           disable internal iterative deepening
        --no-lmr           disable late move reductions
        --no-lmp           disable late move pruning
        --no-beam          disable capture beam
        --no-hash-ext      disable hash-move extension
        --no-quiescence    disable quiescence search

    Example: taikyokushogi-cli --no-tt --no-lmr --depth 3 --go
             (plain alpha-beta with move ordering only, depth 3)

COMMANDS:
    Remaining arguments are interactive commands; `&&` chains them:
        taikyokushogi-cli --threads 4 \"position <tsfen>\" && go depth 6
    With no arguments the CLI reads commands from stdin (REPL)."
}

// ============================================================
// Tests — parsing helpers and command plumbing
// ============================================================

/// Parse the raw argv (without the program name). Unknown `-`/`--` tokens are
/// errors so typos do not silently fall through to the command interpreter;
/// bare tokens keep the old behaviour of being run as commands.
fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut out = CliArgs::default();
    let mut i = 0usize;
    while i < args.len() {
        let tok = args[i].clone();
        // Consume the value token after a flag.
        let take_value = |i: &mut usize, flag: &str| -> Result<String, String> {
            *i += 1;
            args.get(*i)
                .cloned()
                .ok_or_else(|| format!("`{}` needs a value", flag))
        };
        match tok.as_str() {
            "--depth" | "-d" => {
                let v = take_value(&mut i, &tok)?;
                out.depth = Some(v.parse().map_err(|_| format!("bad depth `{}`", v))?);
            }
            "--movetime" | "--time" | "-t" => {
                let v = take_value(&mut i, &tok)?;
                out.movetime = Some(v.parse().map_err(|_| format!("bad movetime `{}`", v))?);
            }
            "--threads" | "-j" => {
                let v = take_value(&mut i, &tok)?;
                let n: usize = v.parse().map_err(|_| format!("bad thread count `{}`", v))?;
                if n == 0 {
                    return Err("`--threads` must be at least 1".to_string());
                }
                out.threads = Some(n);
            }
            "--hash" => {
                let v = take_value(&mut i, &tok)?;
                let mb: usize = v.parse().map_err(|_| format!("bad hash size `{}`", v))?;
                if mb == 0 {
                    return Err("`--hash` must be at least 1 MiB".to_string());
                }
                out.hash_mb = Some(mb);
            }
            // ── Technique toggles ──
            // `--no-<tech>` disables a search technique for testing/ablation.
            "--no-razoring" => out.no_razoring = Some(true),
            "--no-rfp" => out.no_rfp = Some(true),
            "--no-futility" => out.no_futility = Some(true),
            "--no-null-move" => out.no_null_move = Some(true),
            "--no-probcut" => out.no_probcut = Some(true),
            "--no-tt" => out.no_tt = Some(true),
            "--no-killers" => out.no_killers = Some(true),
            "--no-history" => out.no_history = Some(true),
            "--no-counter" => out.no_counter = Some(true),
            "--no-iid" => out.no_iid = Some(true),
            "--no-lmr" => out.no_lmr = Some(true),
            "--no-lmp" => out.no_lmp = Some(true),
            "--no-beam" => out.no_beam = Some(true),
            "--no-hash-ext" => out.no_hash_ext = Some(true),
            "--no-quiescence" => out.no_quiescence = Some(true),
            "--position" | "-p" => {
                let v = take_value(&mut i, &tok)?;
                if v == "startpos" {
                    out.position = Some(v);
                } else {
                    // A shell-quoted TSFEN arrives as one token. Also accept
                    // an unquoted one: keep appending tokens until we hold a
                    // complete position (ranks + side + move number).
                    let mut pos = v;
                    while i + 1 < args.len()
                        && !args[i + 1].starts_with('-')
                        && !(pos.split_whitespace().count() >= 3 && Board::from_tsfen(&pos).is_ok())
                    {
                        i += 1;
                        pos.push(' ');
                        pos.push_str(&args[i]);
                    }
                    out.position = Some(pos);
                }
            }
            "--go" => out.go = true,
            "--perft" => {
                let v = take_value(&mut i, &tok)?;
                out.perft = Some(v.parse().map_err(|_| format!("bad perft depth `{}`", v))?);
            }
            "--help" | "-h" => out.help = true,
            "--" => {
                out.commands.extend_from_slice(&args[i + 1..]);
                break;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag `{}` — see `--help`", other));
            }
            other => out.commands.push(other.to_string()),
        }
        i += 1;
    }
    Ok(out)
}

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
        // files are a-z then A-J; anything else is invalid
        assert_eq!(parse_file('k'), Some(10));
        assert_eq!(parse_file('z'), Some(25));
        assert_eq!(parse_file('J'), Some(35));
        assert_eq!(parse_file('1'), None);
        assert_eq!(parse_file('K'), None); // no second uppercase range
        assert_eq!(parse_file('&'), None);
    }

    // ── move naming ────────────────────────────────────────────
    /// Name of the first legal move in the current position (always valid).
    fn first_move_name() -> String {
        let board = Board::initial();
        move_name(&board.legal_moves().into_iter().next().unwrap())
    }

    #[test]
    fn move_name_roundtrip() {
        let board = Board::initial();
        let m = board
            .legal_moves()
            .into_iter()
            .next()
            .expect("startpos has moves");
        let name = move_name(&m);
        // name = file + rank (1-36) + file + rank, optional '+'
        let body = name.trim_end_matches('+');
        let mut chars = body.chars();
        let f1 = parse_file(chars.next().unwrap());
        let mut digits = String::new();
        let mut f2 = ' ';
        for c in chars.by_ref() {
            if c.is_ascii_alphabetic() {
                f2 = c;
                break;
            }
            digits.push(c);
        }
        let digits2: String = chars.collect();
        let f2p = parse_file(f2);
        assert!(f1.is_some() && f2p.is_some());
        assert!(!digits.is_empty() && !digits2.is_empty());
        assert!(digits.parse::<usize>().unwrap() >= 1 && digits.parse::<usize>().unwrap() <= 36);
        // roundtrip: the name must parse back to the same move
        let parsed = parse_move(&board, &name).expect("move_name must be parseable");
        assert_eq!(parsed.raw().from_sq, m.raw().from_sq);
        assert_eq!(parsed.raw().to_sq, m.raw().to_sq);
    }

    #[test]
    fn parse_move_variants() {
        let board = Board::initial();
        let base = first_move_name();
        let base = base.trim_end_matches('+').to_string();
        // split into two squares (file letter + 1-2 digit rank, twice)
        let (sq1, sq2) = {
            let mut chars = base.chars();
            let f1 = chars.next().unwrap();
            let mut d1 = String::new();
            let mut f2 = ' ';
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    f2 = c;
                    break;
                }
                d1.push(c);
            }
            (
                format!("{}{}", f1, d1),
                format!("{}{}", f2, chars.by_ref().collect::<String>()),
            )
        };
        let m1 = parse_move(&board, &format!("{}{}", sq1, sq2)).expect("algebraic");
        let m2 = parse_move(&board, &format!("{}-{}", sq1, sq2)).expect("dashed");
        assert_eq!(m1.raw().from_sq, m2.raw().from_sq);
        assert_eq!(m1.raw().to_sq, m2.raw().to_sq);
        // numeric form row,col-row,col
        let (r1, c1) = (
            m1.raw().from_sq as usize / 36 + 1,
            m1.raw().from_sq as usize % 36 + 1,
        );
        let (r2, c2) = (
            m1.raw().to_sq as usize / 36 + 1,
            m1.raw().to_sq as usize % 36 + 1,
        );
        let m3 = parse_move(&board, &format!("{},{}-{},{}", r1, c1, r2, c2)).expect("numeric");
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
        assert!(
            err.contains("no legal moves from a1") || err.contains("not legal"),
            "got: {}",
            err
        );
        let err2 = parse_move(&board, "q99q99").unwrap_err();
        assert!(err2.contains("bad square"), "got: {}", err2);
    }

    // ── command execution ──────────────────────────────────────
    fn run_cmds(lines: &[&str]) -> (Board, Options, Vec<String>) {
        let mut board = Board::initial();
        let mut opt = Options::default();
        let mut hist = Vec::new();
        for l in lines {
            if !execute(l, &mut board, &mut opt, &mut hist) {
                break;
            }
        }
        (board, opt, hist)
    }

    #[test]
    fn execute_quit_stops_repl() {
        assert!(!execute(
            "quit",
            &mut Board::initial(),
            &mut Options::default(),
            &mut Vec::new()
        ));
        assert!(!execute(
            "exit",
            &mut Board::initial(),
            &mut Options::default(),
            &mut Vec::new()
        ));
        assert!(execute(
            "help",
            &mut Board::initial(),
            &mut Options::default(),
            &mut Vec::new()
        ));
    }

    #[test]
    fn execute_move_undo_and_history() {
        // derive two actually-legal moves from the position sequence
        let mut probe = Board::initial();
        let m1 = move_name(&probe.legal_moves().into_iter().next().unwrap());
        probe.apply(&probe.legal_moves().into_iter().next().unwrap());
        let m2 = move_name(&probe.legal_moves().into_iter().next().unwrap());
        drop(probe);

        let (board, _, hist) =
            run_cmds(&[&format!("move {}", m1), &format!("move {}", m2), "undo"]);
        assert_eq!(
            hist.len(),
            1,
            "one move remains after undo, hist = {:?}",
            hist
        );
        assert_eq!(hist[0], m1);
        // black + white played, white's move undone → white to move again
        assert_eq!(board.side_to_move(), Color::White);
    }

    #[test]
    fn execute_new_clears_history() {
        let m1 = first_move_name();
        let (board, _, hist) = run_cmds(&[&format!("move {}", m1), "new"]);
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
        let mut probe = Board::initial();
        let m1 = move_name(&probe.legal_moves().into_iter().next().unwrap());
        probe.apply(&probe.legal_moves().into_iter().next().unwrap());
        let m2 = move_name(&probe.legal_moves().into_iter().next().unwrap());
        drop(probe);

        let (mut board, mut opt, mut hist) =
            run_cmds(&[&format!("move {}", m1), &format!("move {}", m2)]);
        assert!(execute(
            &format!("save {}", path),
            &mut board,
            &mut opt,
            &mut hist
        ));
        // reset and load
        assert!(execute("new", &mut board, &mut opt, &mut hist));
        assert!(hist.is_empty());
        assert!(execute(
            &format!("load {}", path),
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0], m1);
        // board matches the saved TSFEN
        let gf: GameFile = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(board.to_tsfen(), gf.tsfen);
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

    // ── command-line flags ─────────────────────────────────────
    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_args_reads_engine_flags() {
        let a = parse_args(&argv(&[
            "--depth",
            "6",
            "--movetime",
            "1500",
            "--threads",
            "4",
            "--hash",
            "128",
            "--position",
            "startpos",
            "--go",
        ]))
        .expect("flags must parse");
        assert_eq!(a.depth, Some(6));
        assert_eq!(a.movetime, Some(1500));
        assert_eq!(a.threads, Some(4));
        assert_eq!(a.hash_mb, Some(128));
        assert_eq!(a.position.as_deref(), Some("startpos"));
        assert!(a.go);
        assert!(!a.help);
        assert!(a.commands.is_empty());
    }

    #[test]
    fn parse_args_keeps_traditional_commands() {
        // Flags first, then the pre-flags argv behaviour: bare tokens are
        // commands, joined with spaces and `&&`-chained.
        let a = parse_args(&argv(&["--threads", "2", "status"])).expect("must parse");
        assert_eq!(a.threads, Some(2));
        assert_eq!(a.commands, vec!["status"]);
        assert_eq!(a.commands.join(" "), "status");
        // short aliases + `--` terminator + bare perft command
        let a =
            parse_args(&argv(&["-d", "4", "-t", "500", "--", "perft", "2"])).expect("must parse");
        assert_eq!(a.depth, Some(4));
        assert_eq!(a.movetime, Some(500));
        assert_eq!(a.commands, vec!["perft", "2"]);
        // short help
        assert!(parse_args(&argv(&["-h"])).expect("-h parses").help);
    }

    #[test]
    fn parse_args_rejects_bad_values() {
        assert!(parse_args(&argv(&["--depth"])).is_err(), "missing value");
        assert!(parse_args(&argv(&["--depth", "x"])).is_err(), "bad number");
        assert!(
            parse_args(&argv(&["--threads", "0"])).is_err(),
            "zero workers"
        );
        assert!(
            parse_args(&argv(&["--threads", "lots"])).is_err(),
            "non-number"
        );
        assert!(parse_args(&argv(&["--hash", "0"])).is_err(), "zero hash");
        assert!(parse_args(&argv(&["--nope"])).is_err(), "unknown flag");
    }

    #[test]
    fn parse_args_position_accepts_quoted_and_split_tsfen() {
        // quoted: one token, parses immediately
        let tsfen = Board::initial().to_tsfen();
        let a = parse_args(&argv(&["--position", &tsfen, "status"])).expect("quoted parses");
        assert_eq!(a.position.as_deref(), Some(tsfen.as_str()));
        assert_eq!(
            a.commands,
            vec!["status"],
            "commands after a full TSFEN stay commands"
        );
        // unquoted: three tokens (ranks side number) reassembled
        let parts: Vec<&str> = tsfen.split_whitespace().collect();
        assert_eq!(parts.len(), 3, "TSFEN must be ranks + side + move number");
        let a =
            parse_args(&argv(&["--position", parts[0], parts[1], parts[2]])).expect("split parses");
        assert_eq!(a.position.as_deref(), Some(tsfen.as_str()));
    }

    #[test]
    fn setoption_threads_stores_and_caps() {
        let mut board = Board::initial();
        let mut opt = Options::default();
        let mut hist = Vec::new();
        assert_eq!(opt.threads, taikyokushogi::search::DEFAULT_THREADS);
        assert!(execute(
            "setoption threads 4",
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert_eq!(opt.threads, 4);
        // above the cap → clamped to MAX_THREADS
        let over = taikyokushogi::search::MAX_THREADS * 2;
        assert!(execute(
            &format!("setoption threads {}", over),
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert_eq!(opt.threads, taikyokushogi::search::MAX_THREADS);
        // 0 rejected, previous value kept
        assert!(execute(
            "setoption threads 0",
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert_eq!(opt.threads, taikyokushogi::search::MAX_THREADS);
        // garbage rejected, previous value kept
        assert!(execute(
            "setoption threads banana",
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert_eq!(opt.threads, taikyokushogi::search::MAX_THREADS);
    }

    #[test]
    fn setoption_hash_resizes_the_default_searcher() {
        let mut board = Board::initial();
        let mut opt = Options::default();
        let mut hist = Vec::new();
        assert!(execute("setoption hash 8", &mut board, &mut opt, &mut hist));
        let actual = taikyokushogi::search::hash_mb();
        assert!(actual >= 1, "the table must report a real size");
        // the reported size is the allocated one (round trip)
        assert!(execute(
            "setoption hash 16",
            &mut board,
            &mut opt,
            &mut hist
        ));
        assert!(taikyokushogi::search::hash_mb() >= 1);
        // garbage rejected
        assert!(execute(
            "setoption hash huge",
            &mut board,
            &mut opt,
            &mut hist
        ));
    }

    #[test]
    fn parse_match_keywords_handles_threads_and_fallbacks() {
        let kw = parse_match_keywords(&[
            "movetime", "250", "maxplies", "30", "threadsA", "4", "threadsB", "2",
        ]);
        assert_eq!(kw.movetime, Some(250));
        assert_eq!(kw.max_plies, Some(30));
        assert_eq!(kw.threads_a, 4);
        assert_eq!(kw.threads_b, 2);

        // nothing given: options stay None (each command keeps its default),
        // workers default to plain single-threaded search
        let kw = parse_match_keywords(&[]);
        assert_eq!(kw.movetime, None);
        assert_eq!(kw.max_plies, None);
        assert_eq!((kw.threads_a, kw.threads_b), (1, 1));

        // case-insensitive keys; bad values never override the previous one
        let kw = parse_match_keywords(&[
            "THREADSA", "3", "threadsA", "0", "threadsB", "x", "junk", "movetime", "soon",
        ]);
        assert_eq!(kw.threads_a, 3, "0 must not override the valid 3");
        assert_eq!(kw.threads_b, 1, "unparsable must not override");
        assert_eq!(kw.movetime, None, "unparsable movetime falls back");
    }
}
