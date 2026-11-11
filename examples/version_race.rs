//! Head-to-head strength testing between two ENGINE VERSIONS (binaries).
//!
//! Plays N games between two engine *executables* — for example the current
//! build and an older commit, built in a separate git worktree — and reports
//! W/L/D, the Elo difference with a 95% confidence interval, and (optional)
//! SPRT. This is the cross-version counterpart of `match_race`, which can
//! only compare two search configurations of the *same* binary.
//!
//! Usage:
//!   cargo run --release --example version_race -- [args]
//!
//! Positional args:  games [depth] [time_ms] [opening_plies] [maxplies]
//! Optional flags:   engineA=PATH    executable of side A (required)
//!                   engineB=PATH    executable of side B (required)
//!                   netA=PATH       side A NNUE network (`--net`)
//!                   netB=PATH       side B NNUE network (`--net`)
//!                   depthA=N        override the depth for side A only
//!                   depthB=N        override the depth for side B only
//!                   threadsA=N      side A Lazy SMP workers (`--threads`)
//!                   threadsB=N      side B Lazy SMP workers (`--threads`)
//!                   hash=N          transposition table MiB for BOTH
//!                   argA="..."      extra CLI args for side A (space-split)
//!                   argB="..."      extra CLI args for side B (space-split)
//!                   sprt=elo0:elo1  SPRT (H0: <= elo0, H1: >= elo1)
//!
//! Examples:
//!   # current build vs the previous commit (as built by `make version-race`)
//!   cargo run --release --example version_race -- 40 2 2000 6 \
//!     engineA=target/release/taikyokushogi-cli \
//!     engineB=target/version-race/target/release/taikyokushogi-cli
//!   # SPRT: is the new version at least +10 Elo?
//!   ... engineA=... engineB=... sprt=0:10
//!
//! Both engines are driven through their CLI (`position <tsfen>` then `go`,
//! reading the `bestmove` line); the harness itself never searches, so the
//! two sides can be genuinely different builds. Each engine runs as one
//! persistent process for the whole match: startup cost (hash allocation,
//! network load) is paid once and the transposition table stays warm
//! between games, as in real play.
//!
//! A `bestmove` line may carry a `mid <sq>` token (newer builds) that pins
//! down igui victim / lion mid-capture squares exactly. Older builds omit
//! it; the harness then falls back to the first legal move with the same
//! from/to/promotion — the same rule the engine CLI applies when a human
//! types that move.
//!
//! The game results are deterministic (seeded LCG openings), so a given
//! invocation is reproducible. For publishable claims use the `sprt` mode
//! with standard bounds and many games.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};

use taikyokushogi::elo::{Sprt, SprtDecision, Wdl};
use taikyokushogi::{Board, Color, GameResult, Move};

// ── Engine process ──────────────────────────────────────────────

struct Engine {
    label: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr_tail: Arc<Mutex<String>>,
}

impl Engine {
    /// Spawn one engine process configured entirely through argv. The harness
    /// then drives it with `position <tsfen>` / `go` lines for the whole
    /// match.
    #[allow(clippy::too_many_arguments)]
    fn spawn(
        label: &str,
        path: &str,
        depth: u32,
        time_ms: u64,
        threads: Option<usize>,
        hash: Option<usize>,
        net: Option<&str>,
        extra: &[String],
    ) -> Engine {
        let mut cmd = Command::new(path);
        cmd.arg("--depth")
            .arg(depth.to_string())
            .arg("--movetime")
            .arg(time_ms.to_string());
        if let Some(n) = threads {
            cmd.arg("--threads").arg(n.to_string());
        }
        if let Some(mb) = hash {
            cmd.arg("--hash").arg(mb.to_string());
        }
        if let Some(p) = net {
            cmd.arg("--net").arg(p);
        }
        cmd.args(extra);
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
        // stderr is drained into a buffer by a helper thread so a chatty
        // engine can never fill the pipe and deadlock the match.
        cmd.stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: cannot start side {label} `{path}`: {e}");
                std::process::exit(2);
            }
        };
        let stderr_tail = Arc::new(Mutex::new(String::new()));
        if let Some(err) = child.stderr.take() {
            let tail = Arc::clone(&stderr_tail);
            std::thread::spawn(move || {
                let mut r = BufReader::new(err);
                let mut line = String::new();
                loop {
                    line.clear();
                    match r.read_line(&mut line) {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {
                            let mut buf = tail.lock().unwrap();
                            buf.push_str(&line);
                            // keep at most the last 100 lines
                            let lines: Vec<&str> = buf.lines().collect();
                            if lines.len() > 100 {
                                *buf = lines[lines.len() - 100..].join("\n");
                                buf.push('\n');
                            }
                        }
                    }
                }
            });
        }
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = BufReader::new(child.stdout.take().expect("stdout was piped"));
        Engine {
            label: label.to_string(),
            child,
            stdin,
            stdout,
            stderr_tail,
        }
    }

    /// Send a position, ask for one search, and read stdout until the
    /// `bestmove` line. Returns `None` if the engine died or closed stdout.
    fn best_move(&mut self, tsfen: &str) -> Option<(String, Option<usize>)> {
        if writeln!(self.stdin, "position {tsfen}").is_err() {
            return None;
        }
        if writeln!(self.stdin, "go").is_err() {
            return None;
        }
        if self.stdin.flush().is_err() {
            return None;
        }
        loop {
            let mut line = String::new();
            match self.stdout.read_line(&mut line) {
                Ok(0) | Err(_) => return None,
                Ok(_) => {}
            }
            let line = line.trim_end();
            if let Some(rest) = line.strip_prefix("bestmove ") {
                let mut parts = rest.split_whitespace();
                let name = parts.next()?.to_string();
                let mut mid = None;
                if parts.next() == Some("mid") {
                    mid = parts.next().and_then(parse_square_token);
                }
                return Some((name, mid));
            }
            // everything else ("position loaded", warnings) is ignored
        }
    }

    fn print_stderr_tail(&self) {
        let tail = self.stderr_tail.lock().unwrap();
        let shown: Vec<&str> = tail.lines().rev().take(10).collect();
        if !shown.is_empty() {
            eprintln!("last stderr from side {}:", self.label);
            for l in shown.into_iter().rev() {
                eprintln!("  {l}");
            }
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // "quit" is best effort: closing stdin (when this struct is dropped)
        // also ends the engine's REPL at EOF. kill() guarantees the match
        // never hangs on a wedged engine at shutdown.
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ── Move parsing (mirrors the CLI's `move_name` / `parse_move`) ──

#[cfg(test)]
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
        (v < 26).then_some(v)
    } else if c.is_ascii_uppercase() {
        let v = c as usize - 'A' as usize;
        (v < 10).then_some(v + 26)
    } else {
        None
    }
}

/// `f19`-style square token -> flat square index (row * 36 + col).
fn parse_square_token(tok: &str) -> Option<usize> {
    let mut chars = tok.chars();
    let col = parse_file(chars.next()?)?;
    let row: usize = chars.as_str().parse().ok()?;
    if (1..=36).contains(&row) {
        Some((row - 1) * 36 + col)
    } else {
        None
    }
}

/// A `bestmove` move token (`f19f20`, `f19f20+`) -> `(from, to, promotion)`.
fn parse_move_token(tok: &str) -> Option<(usize, usize, bool)> {
    let promo = tok.ends_with('+');
    let body: String = tok
        .trim_end_matches('+')
        .chars()
        .filter(|c| *c != '-')
        .collect();
    let mut segments = Vec::new();
    let mut cur = String::new();
    for ch in body.chars() {
        if !cur.is_empty()
            && parse_file(ch).is_some()
            && cur.chars().last().is_some_and(|c| c.is_ascii_digit())
        {
            segments.push(std::mem::take(&mut cur));
        }
        cur.push(ch);
    }
    segments.push(cur);
    if segments.len() != 2 {
        return None;
    }
    let from = parse_square_token(&segments[0])?;
    let to = parse_square_token(&segments[1])?;
    Some((from, to, promo))
}

/// Resolve a `bestmove` token against the harness's authoritative board.
///
/// `mid` (from the optional `mid <sq>` output token) pins igui victims and
/// lion mid-capture squares. Without it, if several legal moves share
/// from/to/promotion, the first one is used — exactly what the engine CLI
/// does for a human-typed move.
fn resolve_move(board: &Board, token: &str, mid: Option<usize>) -> Option<Move> {
    let (from, to, promo) = parse_move_token(token)?;
    let mut candidates: Vec<Move> = Vec::new();
    for m in board.legal_moves() {
        let r = m.raw();
        if r.from_sq as usize == from && r.to_sq as usize == to {
            candidates.push(m);
        }
    }
    if candidates.is_empty() {
        return None;
    }
    if let Some(sq) = mid {
        candidates.retain(|m| m.mid_sq() == Some(sq));
        if candidates.is_empty() {
            return None;
        }
    }
    if candidates.len() > 1 {
        // promotion suffix decides when both variants exist
        let exact: Vec<Move> = candidates
            .iter()
            .copied()
            .filter(|m| m.raw().promotion == promo)
            .collect();
        if !exact.is_empty() {
            candidates = exact;
        }
    }
    candidates.first().copied()
}

// ── Match driving ───────────────────────────────────────────────

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() as usize) % n
        }
    }
}

/// Settings shared by both sides unless overridden per side.
struct Settings {
    depth: u32,
    depth_a: Option<u32>,
    depth_b: Option<u32>,
    time_ms: u64,
    opening_plies: usize,
    max_plies: u32,
}

/// Play one game. `black` and `white` are the engine processes for each
/// colour (colour alternation is handled by the caller). Returns the game
/// result and the plies played, or an error message naming the failing side.
fn play_game(
    game: usize,
    black: &mut Engine,
    white: &mut Engine,
    settings: &Settings,
) -> Result<(GameResult, u32), String> {
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15 ^ (game as u64).wrapping_mul(0x85EB_CA6B));
    let mut board = Board::initial();

    for _ in 0..settings.opening_plies {
        if board.game_result().is_some() {
            break;
        }
        let moves = board.legal_moves();
        if moves.is_empty() {
            break;
        }
        board.apply(&moves[rng.pick(moves.len())]);
    }

    let mut plies: u32 = 0;
    loop {
        if let Some(result) = board.game_result() {
            return Ok((result, plies));
        }
        if settings.max_plies > 0 && plies >= settings.max_plies {
            return Ok((GameResult::Draw, plies)); // adjudication
        }
        let mover = if board.side_to_move() == Color::Black {
            &mut *black
        } else {
            &mut *white
        };
        let side = mover.label.clone();
        let tsfen = board.to_tsfen();
        let Some((token, mid)) = mover.best_move(&tsfen) else {
            return Err(format!("side {side} closed stdout or died mid-game"));
        };
        if token == "none" {
            // No legal move: the side to move loses (same convention as the
            // in-process match_race harness).
            return Ok((
                if board.side_to_move() == Color::Black {
                    GameResult::WhiteWins
                } else {
                    GameResult::BlackWins
                },
                plies,
            ));
        }
        match resolve_move(&board, &token, mid) {
            Some(mv) => board.apply(&mv),
            None => {
                return Err(format!(
                    "side {side} returned move `{token}`, which is not legal in `{tsfen}`"
                ))
            }
        }
        plies += 1;
    }
}

/// Win/draw/loss points from A's perspective.
fn points_from_a(result: GameResult, a_is_black: bool) -> (u64, u64, u64) {
    let a_won = match result {
        GameResult::BlackWins => a_is_black,
        GameResult::WhiteWins => !a_is_black,
        GameResult::Draw => false,
    };
    match (a_won, result == GameResult::Draw) {
        (true, _) => (1, 0, 0),
        (false, true) => (0, 1, 0),
        (false, false) => (0, 0, 1),
    }
}

fn batch_report(sprt: &Sprt, wdl: &Wdl) {
    let n = wdl.total();
    let (lo, hi) = wdl.elo_interval95();
    let mut line = format!(
        "n={n:>4}  W-L-D={}-{}-{}  score={:.3}  Elo {:+.1} [{:+.1}, {:+.1}]",
        wdl.wins,
        wdl.losses,
        wdl.draws,
        wdl.score(),
        wdl.elo(),
        lo,
        hi
    );
    if n > 0 {
        line.push_str(&format!("  LLR={:+.2}", sprt.llr(wdl)));
    }
    println!("{line}");
}

// ── Argument parsing ────────────────────────────────────────────

fn bad_arg(arg: &str) -> ! {
    eprintln!("error: bad numeric value in `{arg}`");
    std::process::exit(2);
}

struct Args {
    games: usize,
    settings: Settings,
    engine_a: Option<String>,
    engine_b: Option<String>,
    net_a: Option<String>,
    net_b: Option<String>,
    threads_a: Option<usize>,
    threads_b: Option<usize>,
    hash: Option<usize>,
    arg_a: Vec<String>,
    arg_b: Vec<String>,
    sprt: Option<(f64, f64)>,
}

fn parse_args() -> Args {
    let mut args = Args {
        games: 40,
        settings: Settings {
            depth: 2,
            depth_a: None,
            depth_b: None,
            time_ms: 2000,
            opening_plies: 6,
            max_plies: 300,
        },
        engine_a: None,
        engine_b: None,
        net_a: None,
        net_b: None,
        threads_a: None,
        threads_b: None,
        hash: None,
        arg_a: Vec::new(),
        arg_b: Vec::new(),
        sprt: None,
    };
    let mut pos: Vec<u64> = Vec::new();
    for a in std::env::args().skip(1) {
        macro_rules! num_opt {
            ($prefix:literal, $target:expr) => {
                if let Some(rest) = a.strip_prefix($prefix) {
                    match rest.parse() {
                        Ok(v) => $target = Some(v),
                        Err(_) => bad_arg(&a),
                    }
                    continue;
                }
            };
        }
        if let Some(rest) = a.strip_prefix("engineA=") {
            args.engine_a = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = a.strip_prefix("engineB=") {
            args.engine_b = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = a.strip_prefix("netA=") {
            args.net_a = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = a.strip_prefix("netB=") {
            args.net_b = Some(rest.to_string());
            continue;
        }
        num_opt!("threadsA=", args.threads_a);
        num_opt!("threadsB=", args.threads_b);
        num_opt!("hash=", args.hash);
        num_opt!("depthA=", args.settings.depth_a);
        num_opt!("depthB=", args.settings.depth_b);
        if let Some(rest) = a.strip_prefix("argA=") {
            args.arg_a = rest.split_whitespace().map(str::to_string).collect();
            continue;
        }
        if let Some(rest) = a.strip_prefix("argB=") {
            args.arg_b = rest.split_whitespace().map(str::to_string).collect();
            continue;
        }
        if let Some(rest) = a.strip_prefix("sprt=") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                args.sprt = Some((
                    parts[0].parse().unwrap_or(0.0),
                    parts[1].parse().unwrap_or(10.0),
                ));
            } else {
                eprintln!("error: sprt expects elo0:elo1");
                std::process::exit(2);
            }
            continue;
        }
        if let Ok(v) = a.parse::<u64>() {
            pos.push(v);
            continue;
        }
        eprintln!("warning: ignored arg `{a}`");
    }
    args.games = pos.first().copied().unwrap_or(args.games as u64) as usize;
    if let Some(v) = pos.get(1) {
        args.settings.depth = *v as u32;
    }
    if let Some(v) = pos.get(2) {
        args.settings.time_ms = *v;
    }
    if let Some(v) = pos.get(3) {
        args.settings.opening_plies = *v as usize;
    }
    if let Some(v) = pos.get(4) {
        args.settings.max_plies = *v as u32;
    }
    args
}

// ── Main ────────────────────────────────────────────────────────

fn main() {
    // run() owns the engine processes; returning from it drops them (quit +
    // kill + wait), and only then does main exit with the match's status.
    std::process::exit(run());
}

fn run() -> i32 {
    let args = parse_args();
    let engine_a = args.engine_a.clone().unwrap_or_else(|| {
        eprintln!("error: engineA=PATH is required (engine executable for side A)");
        std::process::exit(2);
    });
    let engine_b = args.engine_b.clone().unwrap_or_else(|| {
        eprintln!("error: engineB=PATH is required (engine executable for side B)");
        std::process::exit(2);
    });
    let (elo0, elo1) = args.sprt.unwrap_or((0.0, 10.0));
    let sprt = Sprt {
        elo0,
        elo1,
        alpha: 0.05,
        beta: 0.05,
    };

    println!(
        "version race: {} games | A: `{}` d{} {}ms{} | B: `{}` d{} {}ms{} | {} opening plies{}",
        args.games,
        engine_a,
        args.settings.depth_a.unwrap_or(args.settings.depth),
        args.settings.time_ms,
        if args.net_a.is_some() { " +nnue" } else { "" },
        engine_b,
        args.settings.depth_b.unwrap_or(args.settings.depth),
        args.settings.time_ms,
        if args.net_b.is_some() { " +nnue" } else { "" },
        args.settings.opening_plies,
        if args.sprt.is_some() {
            format!(" | SPRT [{elo0},{elo1}] a=0.05 b=0.05")
        } else {
            String::new()
        },
    );

    let mut a = Engine::spawn(
        "A",
        &engine_a,
        args.settings.depth_a.unwrap_or(args.settings.depth),
        args.settings.time_ms,
        args.threads_a,
        args.hash,
        args.net_a.as_deref(),
        &args.arg_a,
    );
    let mut b = Engine::spawn(
        "B",
        &engine_b,
        args.settings.depth_b.unwrap_or(args.settings.depth),
        args.settings.time_ms,
        args.threads_b,
        args.hash,
        args.net_b.as_deref(),
        &args.arg_b,
    );

    let mut wdl = Wdl::default();
    let mut plies_total: u64 = 0;
    const BATCH: usize = 10;

    for game in 0..args.games {
        let a_is_black = game % 2 == 0;
        let (black, white) = if a_is_black {
            (&mut a, &mut b)
        } else {
            (&mut b, &mut a)
        };
        let (result, plies) = match play_game(game, black, white, &args.settings) {
            Ok(x) => x,
            Err(msg) => {
                eprintln!("error: {msg}");
                a.print_stderr_tail();
                b.print_stderr_tail();
                return 1; // engines are reaped by Drop
            }
        };
        let (w, d, l) = points_from_a(result, a_is_black);
        wdl.wins += w;
        wdl.draws += d;
        wdl.losses += l;
        plies_total += plies as u64;
        println!(
            "game {:>3}: {}{} ({} plies)",
            game + 1,
            result,
            if a_is_black { ", A black" } else { ", A white" },
            plies
        );

        if (game + 1) % BATCH == 0 || game + 1 == args.games {
            batch_report(&sprt, &wdl);
            if args.sprt.is_some() {
                match sprt.decision(&wdl) {
                    SprtDecision::AcceptH1 => {
                        println!("=== SPRT: H1 ACCEPTED — A is stronger (>= +{elo1} Elo) ===");
                        report_final(&wdl, plies_total);
                        return 0;
                    }
                    SprtDecision::AcceptH0 => {
                        println!(
                            "=== SPRT: H0 ACCEPTED — no evidence that A is >= +{elo1} Elo ==="
                        );
                        report_final(&wdl, plies_total);
                        return 0;
                    }
                    SprtDecision::Continue => {}
                }
            }
        }
    }

    report_final(&wdl, plies_total);
    0
}

fn report_final(wdl: &Wdl, plies_total: u64) {
    let n = wdl.total();
    let (lo, hi) = wdl.elo_interval95();
    println!();
    println!("=== RESULT ===");
    println!(
        "A: {} wins | B: {} wins | draws {}",
        wdl.wins, wdl.losses, wdl.draws
    );
    println!(
        "Elo(A vs B) = {:+.1}  [95% CI: {:+.1}, {:+.1}]",
        wdl.elo(),
        lo,
        hi
    );
    if n > 0 {
        println!("average plies/game: {:.1}", plies_total as f64 / n as f64);
    }
    println!();
    println!("For a publishable claim: use the sprt=elo0:elo1 mode with many games,");
    println!("or run >= 1000 games and report the confidence interval.");
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_move_token_roundtrip() {
        // `f19f20`: file f (col 5), rank 19 -> row index 18; flat = 18*36+5.
        assert_eq!(
            parse_move_token("f19f20"),
            Some((18 * 36 + 5, 19 * 36 + 5, false))
        );
        assert_eq!(
            parse_move_token("f19f20+"),
            Some((18 * 36 + 5, 19 * 36 + 5, true))
        );
        assert_eq!(parse_move_token("a1a1"), Some((0, 0, false)));
        assert_eq!(
            parse_move_token("J36J36"),
            Some((35 * 36 + 35, 35 * 36 + 35, false))
        );
        assert_eq!(parse_move_token("f19"), None);
        assert_eq!(parse_move_token("f19f20f21"), None);
        assert_eq!(parse_move_token("garbage"), None);
        assert_eq!(parse_move_token("none"), None); // caller special-cases "none" before parsing
    }

    #[test]
    fn square_token_roundtrip() {
        for idx in [0usize, 35, 36, 719, 720, 1295] {
            let row = idx / 36;
            let col = idx % 36;
            let tok = format!("{}{}", file_char(col), row + 1);
            assert_eq!(parse_square_token(&tok), Some(idx));
        }
        assert_eq!(parse_square_token("a1"), Some(0));
        assert_eq!(parse_square_token("J36"), Some(1295));
        assert_eq!(parse_square_token("K36"), None); // only A..J above 'z'
        assert_eq!(parse_square_token("f0"), None);
        assert_eq!(parse_square_token("f37"), None);
    }

    #[test]
    fn resolve_move_matches_the_engine_token() {
        // Every legal move whose (from, to, promotion) triple is unique must
        // round-trip through its rendered token.
        let board = Board::initial();
        let moves = board.legal_moves();
        assert!(!moves.is_empty());
        for m in moves.iter().take(50) {
            let r = m.raw();
            let same: Vec<&Move> = moves
                .iter()
                .filter(|o| {
                    let q = o.raw();
                    q.from_sq == r.from_sq && q.to_sq == r.to_sq && q.promotion == r.promotion
                })
                .collect();
            if same.len() != 1 {
                continue; // ambiguous igui/mid-capture cluster: needs the `mid` token
            }
            let name = format!(
                "{}{}{}{}{}",
                file_char((r.from_sq % 36) as usize),
                (r.from_sq as usize / 36) + 1,
                file_char((r.to_sq % 36) as usize),
                (r.to_sq as usize / 36) + 1,
                if r.promotion { "+" } else { "" }
            );
            let resolved = resolve_move(&board, &name, None).expect("unique move resolves");
            assert_eq!(resolved.raw().from_sq, r.from_sq);
            assert_eq!(resolved.raw().to_sq, r.to_sq);
            assert_eq!(resolved.raw().promotion, r.promotion);
        }
        // A token that matches no legal move does not resolve.
        assert_eq!(resolve_move(&board, "zz", None), None);
    }

    #[test]
    fn resolve_move_uses_the_mid_token_for_igui() {
        // igui moves (stationary captures) share from == to; the `mid` token
        // must pick the exact victim. The initial position is dense enough
        // that igui moves exist; if a rules change removes them, the test
        // still passes without exercising the branch.
        let board = Board::initial();
        let moves = board.legal_moves();
        if let Some(ig) = moves.iter().find(|m| m.raw().is_igui) {
            let r = ig.raw();
            let victim = r.mid_sq as usize;
            let name = format!(
                "{}{}{}{}",
                file_char((r.from_sq % 36) as usize),
                (r.from_sq as usize / 36) + 1,
                file_char((r.from_sq % 36) as usize),
                (r.from_sq as usize / 36) + 1
            );
            let resolved =
                resolve_move(&board, &name, Some(victim)).expect("mid token resolves exactly");
            assert_eq!(resolved.raw().mid_sq as usize, victim);
            assert!(resolved.raw().is_igui);
        }
    }

    #[test]
    fn points_from_a_mapping() {
        assert_eq!(points_from_a(GameResult::BlackWins, true), (1, 0, 0));
        assert_eq!(points_from_a(GameResult::BlackWins, false), (0, 0, 1));
        assert_eq!(points_from_a(GameResult::WhiteWins, false), (1, 0, 0));
        assert_eq!(points_from_a(GameResult::WhiteWins, true), (0, 0, 1));
        assert_eq!(points_from_a(GameResult::Draw, true), (0, 1, 0));
        assert_eq!(points_from_a(GameResult::Draw, false), (0, 1, 0));
    }
}
