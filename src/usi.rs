//! USI (Universal Shogi Interface) front-end for the Taikyoku Shogi engine.
//!
//! Speaks the standard USI command set (`usi`, `isready`, `setoption`,
//! `usinewgame`, `position`, `go`, `stop`, `ponderhit`, `gameover`, `quit`)
//! over stdin/stdout, so the engine can be registered in USI GUIs
//! (ShogiGUI, ShogiHome, Tama, ...) and USI test harnesses. The protocol
//! engine lives in this library module so the whole command surface is
//! unit-testable; the `taikyokushogi-usi` binary is a thin wrapper around
//! [`run`].
//!
//! # Taikyoku Shogi profile
//!
//! USI was designed for 9x9 shogi, which cannot describe a 36x36 board with
//! 804 pieces, so this engine — like other large-variant engines — re-uses
//! the USI envelope with its own position format:
//!
//! * `position startpos` loads the initial position.
//! * `position sfen <TSFEN>` expects the engine's TSFEN notation
//!   (`tsfen::to_tsfen`: 36 `/`-separated ranks, uppercase = Black, lowercase
//!   = White, multi-letter abbreviations in parentheses, then `b|w` and the
//!   move number). A 9x9 SFEN is rejected with an `info string` message.
//! * Move tokens use the engine's coordinate notation: `from`+`to` with files
//!   `a`–`z` (columns 1–26) and `A`–`J` (columns 27–36), ranks `1`–`36`
//!   counted from the top, e.g. `a26a25`; a trailing `+` promotes. Igui
//!   (capture without moving) is `from`==`to`, e.g. `c5c5` (`c5=c5` is also
//!   accepted). `bestmove`, `info ... pv` and `position ... moves` all share
//!   this notation.
//! * Moves that share a from/to pair (lion mid-captures, igui victims, range
//!   captures) cannot be disambiguated in text; the engine always resolves
//!   them to the most valuable capture, consistently for input and output.
//! * `go` supports the standard time parameters (`wtime`, `btime`, `winc`,
//!   `binc`, `byoyomi`), `depth <N>`, `ponder`, `infinite`, `movetime <MS>`
//!   (convenience extension) and bare `go` (= infinite). `go nodes` has no
//!   node-limited search behind it and is answered with an `info string`
//!   plus an unlimited search; `go mate` runs a normal search — this variant
//!   has no checks, so "mating" is the forced capture of the last royal,
//!   which the normal search already finds (reported as `score mate <n>`).
//! * Pondering: this engine never announces a `ponder` move (reply
//!   prediction on a 500+ branching-factor board is a coin toss), so a USI
//!   GUI will normally not send `go ponder`. If one does, the search runs
//!   without a time budget and its result is only played after `ponderhit`
//!   (which re-bases the remaining time from the original `go` parameters);
//!   `stop` during pondering suppresses `bestmove` as the USI spec requires.
//! * Options: `Threads` (Lazy SMP), `Hash` (TT MiB), `DepthLimit` (0 = the
//!   time budget decides), `NetworkFile` (NNUE `.tkn`/`.tkn.zst`, or `none`;
//!   the default comes from `TAIKYOKU_NET`) and `Ponder` (accepted for GUI
//!   compatibility).

use crate::search::{IterInfo, Searcher};
use crate::{Board, Color, Network};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

const ENGINE_NAME: &str = "TaikyokuShogi";
const AUTHOR: &str = "El-Gabo-45 (taikyokushogiTei contributors)";
/// Default transposition-table size (MiB), matching the `Hash` option default.
const DEFAULT_HASH_MB: usize = 256;
/// Clock head-room (ms) a time-budgeted move never touches.
const RESERVE_MS: u64 = 200;
/// Search depth passed to the deepening loop when no explicit depth was
/// requested: the loop then only ends through the time budget or a `stop`.
const UNLIMITED_DEPTH: u32 = u32::MAX;
/// `|score|` beyond [`crate::MATE_SCORE`] minus this is reported as
/// `score mate` (plies-to-mate never gets anywhere near this bound).
const MATE_PLIES_BOUND: i32 = 4_096;
/// Fallback budget (ms) for `go mate` without any time parameters.
const MATE_FALLBACK_MS: u64 = 3_000;

// ============================================================
// Output sink
// ============================================================

/// Where USI reply lines go. [`run`] wires this to stdout; tests collect
/// lines in a `Vec` instead.
pub trait UsiSink: Send + Sync {
    fn send(&self, line: &str);
}

struct StdoutSink;

impl UsiSink for StdoutSink {
    fn send(&self, line: &str) {
        println!("{line}");
    }
}

// ============================================================
// `go` parameters and time budget
// ============================================================

/// Parsed `go` parameters (USI plus the `movetime` convenience extension).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GoParams {
    pub ponder: bool,
    pub infinite: bool,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
    pub byoyomi: Option<u64>,
    pub movetime: Option<u64>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub mate: bool,
}

impl GoParams {
    fn parse(tokens: &[&str]) -> Self {
        let mut p = GoParams::default();
        let mut i = 0usize;
        while i < tokens.len() {
            let key = tokens[i].to_ascii_lowercase();
            // Numbers follow most keys; missing/invalid values leave None.
            let num = |i: &mut usize| -> Option<u64> {
                *i += 1;
                tokens.get(*i).and_then(|v| v.parse().ok())
            };
            match key.as_str() {
                "ponder" => p.ponder = true,
                "infinite" => p.infinite = true,
                "wtime" => p.wtime = num(&mut i),
                "btime" => p.btime = num(&mut i),
                "winc" => p.winc = num(&mut i),
                "binc" => p.binc = num(&mut i),
                "byoyomi" => p.byoyomi = num(&mut i),
                "movetime" => p.movetime = num(&mut i),
                "depth" => p.depth = num(&mut i).map(|v| v as u32),
                "nodes" => p.nodes = num(&mut i),
                // `go mate [N | in N | all]`: the keyword alone means mate.
                "mate" => {
                    p.mate = true;
                    if let Some(v) = tokens.get(i + 1) {
                        if v.parse::<u64>().is_ok() {
                            i += 1;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
        p
    }
}

/// Wall-clock budget for a `go`: returns `(search depth, budget_ms)` with
/// `budget_ms == 0` meaning "until `stop`". `elapsed_ms` re-bases the clocks
/// for a `ponderhit` that arrives after the pondering began.
///
/// Policy: an explicit `depth` or `movetime` wins; otherwise spend
/// `remaining/30 + increment*0.9` (or 90% of the clock when under a second),
/// never touching the [`RESERVE_MS`] head-room; `byoyomi` may stretch the
/// spend within what the main clock can still afford.
fn budget_from_params(
    params: &GoParams,
    side: Color,
    depth_limit: u32,
    elapsed_ms: Option<u64>,
) -> (u32, u64) {
    if let Some(d) = params.depth {
        return (d.max(1), 0);
    }
    let depth = if depth_limit == 0 {
        UNLIMITED_DEPTH
    } else {
        depth_limit
    };
    if let Some(t) = params.movetime {
        return (depth, t.max(1));
    }
    // `go infinite` (or a bare `go`): run until `stop`, no matter what other
    // time parameters tagged along.
    if params.infinite {
        return (depth, 0);
    }
    let (left, inc) = match side {
        Color::Black => (params.btime, params.binc.unwrap_or(0)),
        Color::White => (params.wtime, params.winc.unwrap_or(0)),
    };
    let left = left.map(|l| l.saturating_sub(elapsed_ms.unwrap_or(0)));
    if let Some(left) = left {
        let reserve = RESERVE_MS.min(left);
        let usable = left.saturating_sub(reserve);
        if usable == 0 {
            // Desperate: play instantly, leaving the reserve untouched.
            return (depth, 50);
        }
        let mut spend = if usable < 1_000 {
            usable * 9 / 10
        } else {
            usable / 30 + inc * 9 / 10
        };
        // Byoyomi time is available on top of the main clock; only use it
        // within what the main clock can still afford (never flag because of
        // it).
        if let Some(byo) = params.byoyomi {
            spend = spend.max((byo * 9 / 10).min(usable));
        }
        (depth, spend.clamp(50.min(usable), usable))
    } else if let Some(byo) = params.byoyomi {
        (depth, byo * 9 / 10)
    } else {
        // No time information at all: search until `stop` (USI `go infinite`,
        // also the bare `go`).
        (depth, 0)
    }
}

// ============================================================
// Notation
// ============================================================

/// File letter for a column (0-based): `a`–`z`, then `A`–`J`.
fn file_char(col: usize) -> char {
    if col < 26 {
        (b'a' + col as u8) as char
    } else {
        (b'A' + (col - 26) as u8) as char
    }
}

/// `file + rank` name of a flat square index (rank = row + 1 from the top).
fn square_name(sq: u16) -> String {
    format!("{}{}", file_char((sq % 36) as usize), sq / 36 + 1)
}

/// USI move token for an internal move: `from`+`to`, `+` when it promotes.
/// (Igui moves are `from`==`to` by construction.)
fn usi_move_str(m: &crate::types::Move) -> String {
    let mut s = format!("{}{}", square_name(m.from_sq), square_name(m.to_sq));
    if m.promotion {
        s.push('+');
    }
    s
}

/// Parse a `file + rank` square into a flat index.
fn parse_square(s: &str) -> Option<u16> {
    let mut chars = s.chars();
    let col = match chars.next()? {
        c @ 'a'..='z' => c as usize - 'a' as usize,
        c @ 'A'..='J' => 26 + c as usize - 'A' as usize,
        _ => return None,
    };
    let rank: usize = chars.as_str().parse().ok()?;
    if !(1..=36).contains(&rank) || col >= 36 {
        return None;
    }
    Some(((rank - 1) * 36 + col) as u16)
}

/// Split `e34A12` into `("e34", "A12")`: a from/to pair breaks where a file
/// letter (`a`–`z`, `A`–`J`) immediately follows a digit.
fn split_two_squares(s: &str) -> Option<(&str, &str)> {
    let is_file = |c: char| matches!(c, 'a'..='z' | 'A'..='J');
    let mut last_was_digit = false;
    for (i, c) in s.char_indices() {
        if last_was_digit && is_file(c) {
            return Some((&s[..i], &s[i..]));
        }
        last_was_digit = c.is_ascii_digit();
    }
    None
}

/// Parse one move token in the engine's coordinate notation and match it
/// against the current position's legal moves.
///
/// Accepted forms: `a26a25`, `a26a25+`, `a26-a25` and the igui spellings
/// `c5c5` / `c5=c5`. When several legal moves share the from/to pair (lion
/// mid-captures, igui victims, range captures) the text cannot say which one
/// was meant, so the most valuable capture is chosen — the same rule the
/// engine uses when emitting such a move, which keeps `bestmove` ↔ `position
/// moves ...` round-trips consistent.
fn parse_move(board: &Board, token: &str) -> Result<crate::Move, String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("empty move".into());
    }
    let promotion = token.ends_with('+');
    let core = token.trim_end_matches('+');
    // Squares are `file + 1..2 digits`, so a from/to pair is NOT a fixed
    // 4-character slice: `e34A12` is `e34` + `A12`. Split where a file letter
    // follows a digit (the `=` igui spelling is an explicit separator).
    let squares: (Option<u16>, Option<u16>) = match core.split_once('=') {
        Some((a, b)) => (parse_square(a.trim()), parse_square(b.trim())),
        None => {
            let cleaned: String = core.chars().filter(|c| *c != '-').collect();
            match split_two_squares(&cleaned) {
                Some((a, b)) => (parse_square(a), parse_square(b)),
                None => return Err(format!("cannot parse `{token}` (expected e.g. `a26a25`)")),
            }
        }
    };
    let (from, to) = match squares {
        (Some(f), Some(t)) => (f, t),
        _ => return Err(format!("bad square in `{token}`")),
    };
    let mut matching: Vec<crate::Move> = board
        .legal_moves()
        .into_iter()
        .filter(|m| {
            let r = m.raw();
            r.from_sq == from && r.to_sq == to
        })
        .collect();
    if matching.is_empty() {
        return Err(format!("`{token}` is not legal here"));
    }
    // from == to is an igui (capture without moving); elsewhere a plain move
    // always beats an igui variant should both somehow match.
    if from == to {
        matching.retain(|m| m.raw().is_igui);
    } else {
        let plain: Vec<crate::Move> = matching
            .iter()
            .copied()
            .filter(|m| !m.raw().is_igui)
            .collect();
        if !plain.is_empty() {
            matching = plain;
        }
    }
    if matching.is_empty() {
        return Err(format!("no igui move matches `{token}`"));
    }
    // When both promoting and non-promoting variants exist the `+` decides.
    let promo_variants = matching.iter().filter(|m| m.raw().promotion).count();
    if promo_variants > 0 && promo_variants < matching.len() {
        matching.retain(|m| m.raw().promotion == promotion);
    }
    // Still ambiguous (lion mid square, igui victim, range capture): prefer
    // the most valuable combined capture; ties keep generation order.
    if matching.len() > 1 {
        matching.sort_by_key(|m| {
            let r = m.raw();
            -(i64::from(r.caps_value)
                + i64::from(crate::pieces::value(r.captured_piece))
                + i64::from(crate::pieces::value(r.mid_piece)))
        });
    }
    Ok(matching[0])
}

// ============================================================
// Info line
// ============================================================

/// One completed iteration as a USI `info` line:
/// `info depth D nodes N time T score cp|mate S nps P pv <move>`.
fn format_info(info: &IterInfo) -> String {
    let mut line = format!(
        "info depth {} nodes {} time {}",
        info.depth, info.nodes, info.time_ms
    );
    if info.score.abs() >= crate::MATE_SCORE - MATE_PLIES_BOUND {
        // `score mate N` counts full moves: a 1-ply royal capture is mate in 1.
        let plies = (crate::MATE_SCORE - info.score.abs()).max(1);
        let moves = (plies + 1) / 2;
        if info.score > 0 {
            line.push_str(&format!(" score mate {moves}"));
        } else {
            line.push_str(&format!(" score mate -{moves}"));
        }
    } else {
        line.push_str(&format!(" score cp {}", info.score));
    }
    if let Some(nps) = info
        .nodes
        .checked_mul(1000)
        .and_then(|n| n.checked_div(info.time_ms))
    {
        line.push_str(&format!(" nps {nps}"));
    }
    if let Some(m) = &info.best_move {
        line.push_str(&format!(" pv {}", usi_move_str(&m.inner)));
    }
    line
}

// ============================================================
// Engine
// ============================================================

/// A running (or just-finished) search job owned by its worker thread. The
/// thread returns the [`Searcher`] on completion so the engine keeps the
/// transposition table and heuristics warm between moves.
struct Job {
    handle: std::thread::JoinHandle<Searcher>,
    /// Cooperative stop flag (`stop`, pondering teardown, new `go`).
    stop: Arc<AtomicBool>,
    /// When set, the thread does not report a `bestmove` (pondering teardown,
    /// `gameover`, a superseded search).
    silent: Arc<AtomicBool>,
    params: GoParams,
    side_at_go: Color,
    started: Instant,
    /// Started by `go ponder` — only then is `ponderhit` meaningful.
    ponder: bool,
}

/// The USI engine: one game position, one persistent [`Searcher`], the
/// loaded options and (at most) one background search.
pub struct UsiEngine {
    sink: Arc<dyn UsiSink>,
    board: Board,
    searcher: Option<Searcher>,
    net: Option<Arc<Network>>,
    threads: usize,
    /// `0` = the time budget decides the depth.
    depth_limit: u32,
    /// `Ponder` option echo (GUIs toggle it; see the module docs).
    ponder: bool,
    job: Option<Job>,
    quit: bool,
}

impl UsiEngine {
    /// New engine on the given sink, with the initial position and the
    /// network from `TAIKYOKU_NET` (if any).
    pub fn new(sink: Arc<dyn UsiSink>) -> Self {
        let net = load_env_net();
        let mut board = Board::initial();
        board.set_network(net.clone());
        let searcher = Searcher::new();
        searcher.set_hash_mb(DEFAULT_HASH_MB);
        UsiEngine {
            sink,
            board,
            searcher: Some(searcher),
            net,
            threads: crate::search::DEFAULT_THREADS,
            depth_limit: 0,
            ponder: false,
            job: None,
            quit: false,
        }
    }

    /// Handle one USI command line. Replies go through the sink; unknown
    /// commands are ignored, as the USI spec allows.
    pub fn handle(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let mut tokens = line.split_whitespace();
        let cmd = tokens.next().unwrap_or("").to_ascii_lowercase();
        let rest: Vec<&str> = tokens.collect();
        match cmd.as_str() {
            "usi" => self.cmd_usi(),
            "isready" => self.send("readyok"),
            "setoption" => self.cmd_setoption(&rest),
            "usinewgame" => {
                self.join_search(true);
                if let Some(s) = self.searcher.as_mut() {
                    s.clear();
                }
            }
            "position" => self.cmd_position(&rest),
            "go" => self.cmd_go(&rest),
            "stop" => self.join_search(false),
            "ponderhit" => self.cmd_ponderhit(),
            "gameover" => self.join_search(true),
            "quit" => {
                self.quit = true;
                self.join_search(true);
            }
            _ => {}
        }
    }

    /// Whether a `quit` was received.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    fn send(&self, line: &str) {
        self.sink.send(line);
    }

    // ── commands ────────────────────────────────────────────────

    fn cmd_usi(&mut self) {
        self.send(&format!("id name {ENGINE_NAME} {}", crate::VERSION));
        self.send(&format!("id author {AUTHOR}"));
        let net_default = std::env::var("TAIKYOKU_NET").unwrap_or_default();
        for line in [
            format!(
                "option name Threads type spin default {} min 1 max {}",
                crate::search::DEFAULT_THREADS,
                crate::search::MAX_THREADS
            ),
            format!("option name Hash type spin default {DEFAULT_HASH_MB} min 16 max 4096"),
            "option name DepthLimit type spin default 0 min 0 max 32".to_string(),
            format!("option name NetworkFile type string default {net_default}"),
            "option name Ponder type check default false".to_string(),
        ] {
            self.send(&line);
        }
        self.send("usiok");
    }

    fn cmd_setoption(&mut self, rest: &[&str]) {
        // `setoption name <id> [value <x>]` — the value may contain spaces
        // (network file paths).
        let idx = usize::from(
            rest.first()
                .map(|s| s.eq_ignore_ascii_case("name"))
                .unwrap_or(false),
        );
        let Some(name) = rest.get(idx) else {
            self.send("info string setoption: expected `name <id> [value <x>]`");
            return;
        };
        let after: &[&str] = &rest[(idx + 1).min(rest.len())..];
        let value: Option<String> = if after.is_empty() {
            None
        } else if after[0].eq_ignore_ascii_case("value") {
            Some(after[1..].join(" "))
        } else {
            Some(after.join(" ")) // tolerate a missing `value` keyword
        };
        let val = value.unwrap_or_default();
        match name.to_ascii_lowercase().as_str() {
            "threads" => match val.trim().parse::<usize>() {
                Ok(n) if n >= 1 => self.threads = n.min(crate::search::MAX_THREADS),
                _ => self.send("info string setoption Threads: expected a number >= 1"),
            },
            "hash" => match val.trim().parse::<usize>() {
                Ok(mb) => {
                    let mb = mb.max(16);
                    if self.job.is_some() {
                        self.send("info string setoption Hash ignored while searching");
                    } else {
                        let actual = self
                            .searcher
                            .get_or_insert_with(Searcher::new)
                            .set_hash_mb(mb);
                        self.send(&format!("info string hash = {actual} MiB"));
                    }
                }
                Err(_) => self.send("info string setoption Hash: expected a MiB number"),
            },
            "depthlimit" => match val.trim().parse::<u32>() {
                Ok(d) => self.depth_limit = d.min(32),
                Err(_) => self.send("info string setoption DepthLimit: expected a number >= 0"),
            },
            "networkfile" => {
                let path = val.trim();
                if path.is_empty() || path.eq_ignore_ascii_case("none") {
                    self.net = None;
                    self.board.set_network(None);
                } else {
                    match Network::load(path) {
                        Ok(n) => {
                            let net = Arc::new(n);
                            self.net = Some(net.clone());
                            self.board.set_network(Some(net));
                        }
                        Err(e) => {
                            self.send(&format!(
                                "info string cannot load network `{path}`: {e} (keeping the previous evaluator)"
                            ));
                        }
                    }
                }
            }
            "ponder" => {
                self.ponder = val.trim().eq_ignore_ascii_case("true");
                if self.ponder {
                    self.send(
                        "info string Ponder on: this engine never announces a ponder move, but `go ponder`/`ponderhit` are honored",
                    );
                }
            }
            other => {
                self.send(&format!("info string unknown option `{other}`"));
            }
        }
    }

    fn cmd_position(&mut self, rest: &[&str]) {
        self.join_search(true);
        if rest.is_empty() {
            self.send("info string position: expected `startpos` or `sfen <TSFEN>`");
            return;
        }
        let moves_from = match rest[0].to_ascii_lowercase().as_str() {
            "startpos" => {
                let mut board = Board::initial();
                board.set_network(self.net.clone());
                self.board = board;
                1
            }
            "sfen" | "tsfen" => {
                // TSFEN is three whitespace-free fields: ranks side move_number.
                if rest.len() < 4 {
                    self.send("info string position sfen: expected ranks, side and move number");
                    return;
                }
                let tsfen = format!("{} {} {}", rest[1], rest[2], rest[3]);
                match Board::from_tsfen(&tsfen) {
                    Ok(mut board) => {
                        board.set_network(self.net.clone());
                        self.board = board;
                        4
                    }
                    Err(e) => {
                        self.send(&format!(
                            "info string position rejected: {e} (this engine expects a Taikyoku Shogi TSFEN with 36 ranks)"
                        ));
                        return;
                    }
                }
            }
            other => {
                self.send(&format!(
                    "info string position: unknown keyword `{other}` (expected startpos or sfen)"
                ));
                return;
            }
        };
        // `moves <m1> <m2> ...`
        let mut moves: &[&str] = &rest[moves_from.min(rest.len())..];
        if moves.first().map(|s| s.eq_ignore_ascii_case("moves")) == Some(true) {
            moves = &moves[1..];
        }
        for token in moves {
            match parse_move(&self.board, token) {
                Ok(m) => self.board.apply(&m),
                Err(e) => {
                    self.send(&format!("info string illegal move `{token}` ignored: {e}"));
                    break;
                }
            }
        }
    }

    fn cmd_go(&mut self, rest: &[&str]) {
        // A well-behaved GUI never sends `go` while a search is running;
        // abort any such search silently anyway.
        self.join_search(true);
        let params = GoParams::parse(rest);
        let side = self.board.side_to_move();
        if params.mate {
            self.send(
                "info string no mate search in Taikyoku Shogi (no checks); searching normally for the royal capture",
            );
        }
        if params.nodes.is_some() {
            self.send("info string go nodes is not supported; searching without the node limit");
        }
        if params.ponder {
            // Pondering: no budget — the result is only played after
            // `ponderhit` (which restarts with the remaining time) and always
            // suppressed on `stop`.
            self.start_search(UNLIMITED_DEPTH, 0, true, params);
        } else {
            let (mut depth, mut budget) = budget_from_params(&params, side, self.depth_limit, None);
            if params.mate && budget == 0 {
                // Mate requests must always answer: cap the search hard.
                budget = MATE_FALLBACK_MS;
                depth = depth.min(12);
            }
            self.start_search(depth, budget, false, params);
        }
    }

    fn cmd_ponderhit(&mut self) {
        let Some(job) = self.job.as_ref() else {
            return;
        };
        if !job.ponder {
            return; // `ponderhit` outside pondering: ignore
        }
        // Abandon the ponder search and think normally with the remaining
        // time. The transposition table carries over, so the restart is
        // cheap compared to the elapsed ponder time.
        let params = job.params.clone();
        let side = job.side_at_go;
        let started = job.started;
        self.join_search(true);
        let elapsed = started.elapsed().as_millis() as u64;
        let (depth, budget) = budget_from_params(&params, side, self.depth_limit, Some(elapsed));
        self.start_search(depth, budget, false, params);
    }

    // ── search lifecycle ────────────────────────────────────────

    /// Spawn a background search. The thread owns the searcher (and a clone
    /// of the position) until it finishes; [`UsiEngine::join_search`] waits
    /// for it and parks the searcher back on the engine.
    fn start_search(&mut self, depth: u32, budget_ms: u64, ponder: bool, params: GoParams) {
        let mut searcher = self.searcher.take().unwrap_or_default();
        let mut board = self.board.clone();
        board.set_network(self.net.clone());
        let threads = self.threads;
        let sink_main = self.sink.clone();
        let sink_iter = self.sink.clone();
        let stop = Arc::new(AtomicBool::new(false));
        // A ponder search never reports a move: it is either discarded on
        // `stop` or superseded by the `ponderhit` restart (which runs its own
        // reportable search).
        let silent = Arc::new(AtomicBool::new(ponder));
        let (stop_thread, silent_thread) = (stop.clone(), silent.clone());
        let handle = std::thread::Builder::new()
            .name("usi-search".into())
            .spawn(move || {
                let result = searcher.search_threads_with_stop(
                    &mut board.inner,
                    depth,
                    budget_ms,
                    threads,
                    &stop_thread,
                    Some(&mut move |info: &IterInfo| {
                        sink_iter.send(&format_info(info));
                    }),
                );
                if !silent_thread.load(Ordering::Relaxed) {
                    match &result.best_move {
                        Some(m) => sink_main.send(&format!("bestmove {}", usi_move_str(m))),
                        None => sink_main.send("bestmove resign"),
                    }
                }
                searcher
            })
            .expect("spawn the USI search thread");
        self.job = Some(Job {
            handle,
            stop,
            silent,
            params,
            side_at_go: self.board.side_to_move(),
            started: Instant::now(),
            ponder,
        });
    }

    /// Wait for a running search to finish on its own (no stop signal) and
    /// park the searcher back on the engine. Does nothing when idle.
    fn wait_search(&mut self) {
        if let Some(job) = self.job.take() {
            self.recover(job);
        }
    }

    /// Stop a running search (if any), wait for its thread and park the
    /// searcher back on the engine. With `silent` the thread does not report
    /// a `bestmove`.
    fn join_search(&mut self, silent: bool) {
        if let Some(job) = self.job.take() {
            if silent {
                job.silent.store(true, Ordering::Relaxed);
            }
            job.stop.store(true, Ordering::Relaxed);
            self.recover(job);
        }
    }

    /// Join a taken job's thread and put its searcher back into service.
    fn recover(&mut self, job: Job) {
        match job.handle.join() {
            Ok(searcher) => self.searcher = Some(searcher),
            Err(_) => {
                // The search thread panicked (debug builds only — the
                // release profile aborts): rebuild a fresh searcher.
                eprintln!("taikyokushogi-usi: search thread panicked; state reset");
                self.searcher = Some(Searcher::new());
            }
        }
    }
}

/// NNUE network from the `TAIKYOKU_NET` environment variable (same variable
/// and semantics as the HTTP server: `none` disables, `.tkn` or `.tkn.zst`).
fn load_env_net() -> Option<Arc<Network>> {
    let Ok(path) = std::env::var("TAIKYOKU_NET") else {
        return None;
    };
    if path.is_empty() || path.eq_ignore_ascii_case("none") {
        return None;
    }
    match Network::load(&path) {
        Ok(n) => Some(Arc::new(n)),
        Err(e) => {
            eprintln!("taikyokushogi-usi: cannot load TAIKYOKU_NET `{path}`: {e}");
            None
        }
    }
}

// ============================================================
// Entry point
// ============================================================

/// Run the USI loop on stdin/stdout until `quit` or EOF. Diagnostics go to
/// stderr so stdout stays a clean USI channel.
pub fn run() {
    let sink: Arc<dyn UsiSink> = Arc::new(StdoutSink);
    let mut engine = UsiEngine::new(sink);
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                engine.handle(&line);
                if engine.quit_requested() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    // EOF (the GUI closed the pipe) behaves like `quit`.
    engine.join_search(true);
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Sink that records every line (shared with search threads).
    #[derive(Clone, Default)]
    struct VecSink(Arc<Mutex<Vec<String>>>);

    impl UsiSink for VecSink {
        fn send(&self, line: &str) {
            self.0.lock().unwrap().push(line.to_string());
        }
    }

    impl VecSink {
        fn lines(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
        fn last(&self) -> String {
            self.0.lock().unwrap().last().cloned().unwrap_or_default()
        }
    }

    fn engine() -> (UsiEngine, VecSink) {
        let sink = VecSink::default();
        (UsiEngine::new(Arc::new(sink.clone())), sink)
    }

    // ── notation ────────────────────────────────────────────────

    #[test]
    fn square_name_and_parse_round_trip() {
        for idx in 0..1_296u16 {
            let name = square_name(idx);
            assert_eq!(parse_square(&name), Some(idx), "square {idx} ({name})");
        }
        assert_eq!(parse_square("a0"), None);
        assert_eq!(parse_square("a37"), None);
        assert_eq!(parse_square("K36"), None); // files stop at J
        assert_eq!(parse_square(""), None);
    }

    #[test]
    fn file_char_matches_cli_convention() {
        assert_eq!(file_char(0), 'a');
        assert_eq!(file_char(25), 'z');
        assert_eq!(file_char(26), 'A');
        assert_eq!(file_char(35), 'J');
    }

    #[test]
    fn move_format_round_trip_through_parse_move() {
        let board = Board::initial();
        for m in board.legal_moves().into_iter().take(64) {
            let token = usi_move_str(&m.inner);
            let parsed = parse_move(&board, &token)
                .unwrap_or_else(|e| panic!("`{token}` must re-parse: {e}"));
            let r1 = m.raw();
            let r2 = parsed.raw();
            assert_eq!(r1.from_sq, r2.from_sq, "token {token}");
            assert_eq!(r1.to_sq, r2.to_sq, "token {token}");
            assert_eq!(r1.promotion, r2.promotion, "token {token}");
        }
    }

    #[test]
    fn parse_move_accepts_dashes_and_igui_equals() {
        let board = Board::initial();
        let m = board.legal_moves()[0]; // a quiet pawn push at the start
        let plain = usi_move_str(&m.inner);
        assert!(plain.len() >= 4);
        let from = plain[..plain.len() / 2].to_string();
        let to = plain[plain.len() / 2..].to_string();
        assert_eq!(plain, format!("{from}{to}"));
        // `from-to` and the `from=to` spelling both resolve to the same move.
        for variant in [
            format!("{from}{to}"),
            format!("{from}-{to}"),
            format!("{from}={to}"),
        ] {
            let parsed = parse_move(&board, &variant)
                .unwrap_or_else(|e| panic!("`{variant}` must parse: {e}"));
            assert_eq!(parsed.raw().from_sq, m.raw().from_sq, "variant {variant}");
            assert_eq!(parsed.raw().to_sq, m.raw().to_sq, "variant {variant}");
        }
        // A garbage suffix makes it unparsable.
        assert!(parse_move(&board, &format!("{from}{to}9x")).is_err());
    }

    #[test]
    fn parse_move_rejects_unknown_moves() {
        let board = Board::initial();
        assert!(parse_move(&board, "f19f20").is_err()); // empty centre square
        assert!(parse_move(&board, "a99a98").is_err()); // off board
        assert!(parse_move(&board, "").is_err());
    }

    // ── `go` parameters and budget ──────────────────────────────

    #[test]
    fn go_params_parse() {
        let p = GoParams::parse(&["wtime", "60000", "btime", "30000", "binc", "5000", "ponder"]);
        assert!(p.ponder);
        assert_eq!(p.wtime, Some(60_000));
        assert_eq!(p.btime, Some(30_000));
        assert_eq!(p.binc, Some(5_000));
        assert_eq!(p.winc, None);
        assert!(GoParams::parse(&["mate"]).mate);
        assert!(GoParams::parse(&["mate", "in", "5"]).mate);
        assert!(GoParams::parse(&["mate", "7"]).mate);
        assert_eq!(GoParams::parse(&["depth", "4"]).depth, Some(4));
        assert_eq!(GoParams::parse(&["bogus", "1"]), GoParams::default());
    }

    #[test]
    fn budget_explicit_depth_and_movetime_win() {
        let side = Color::Black;
        let p = GoParams {
            depth: Some(3),
            btime: Some(1_000),
            ..Default::default()
        };
        assert_eq!(budget_from_params(&p, side, 0, None), (3, 0));
        let p = GoParams {
            movetime: Some(1_500),
            btime: Some(1_000),
            ..Default::default()
        };
        assert_eq!(budget_from_params(&p, side, 0, None).1, 1_500);
    }

    #[test]
    fn budget_never_overspends_the_clock() {
        let side = Color::Black;
        for left in [0, 100, 250, 800, 5_000, 60_000, 600_000] {
            let p = GoParams {
                btime: Some(left),
                ..Default::default()
            };
            let (_, budget) = budget_from_params(&p, side, 0, None);
            assert!(
                budget <= left.saturating_sub(RESERVE_MS.min(left)).max(50),
                "left={left}"
            );
        }
    }

    #[test]
    fn budget_scales_with_time_and_increment() {
        let side = Color::White;
        let p = GoParams {
            wtime: Some(60_000),
            winc: Some(5_000),
            ..Default::default()
        };
        let (_, budget) = budget_from_params(&p, side, 0, None);
        assert_eq!(
            budget,
            (60_000 - RESERVE_MS) / 30 + 5_000 * 9 / 10,
            "usable = remaining minus the reserve"
        );
    }

    #[test]
    fn budget_byoyomi_only_stretches_within_the_main_clock() {
        let side = Color::Black;
        let p = GoParams {
            btime: Some(1_000),
            byoyomi: Some(30_000),
            ..Default::default()
        };
        let (_, budget) = budget_from_params(&p, side, 0, None);
        assert!(budget <= 800, "must stay inside the main clock: {budget}");
        // Without a main clock the byoyomi is the whole budget (minus margin).
        let p = GoParams {
            byoyomi: Some(30_000),
            ..Default::default()
        };
        assert_eq!(budget_from_params(&p, side, 0, None).1, 27_000);
    }

    #[test]
    fn budget_no_time_info_is_unlimited() {
        let p = GoParams::default();
        let (depth, budget) = budget_from_params(&p, Color::Black, 0, None);
        assert_eq!(budget, 0);
        assert_eq!(depth, UNLIMITED_DEPTH);
    }

    #[test]
    fn budget_elapsed_rebases_the_clock() {
        let side = Color::White;
        let p = GoParams {
            wtime: Some(30_000),
            ..Default::default()
        };
        let full = budget_from_params(&p, side, 0, None).1;
        let after = budget_from_params(&p, side, 0, Some(20_000)).1;
        assert!(
            after < full,
            "ponderhit must see a smaller clock: {after} vs {full}"
        );
    }

    // ── info line ───────────────────────────────────────────────

    #[test]
    fn format_info_cp_and_mate() {
        let info = IterInfo {
            depth: 4,
            score: 1_234,
            nodes: 9_876,
            time_ms: 500,
            best_move: Some(crate::Move {
                inner: crate::types::Move::simple(0, 37),
            }),
        };
        let line = format_info(&info);
        assert!(
            line.starts_with("info depth 4 nodes 9876 time 500"),
            "{line}"
        );
        assert!(line.contains(" score cp 1234 "), "{line}");
        assert!(line.contains(" nps 19752 "), "{line}");
        assert!(line.ends_with(" pv a1b2"), "{line}");

        let mate = IterInfo {
            depth: 8,
            score: crate::MATE_SCORE - 3, // 3 plies -> mate in 2 moves
            nodes: 1,
            time_ms: 10,
            best_move: None,
        };
        let line = format_info(&mate);
        assert!(line.contains(" score mate 2 "), "{line}");
        let mated = IterInfo {
            depth: 8,
            score: -(crate::MATE_SCORE - 1),
            nodes: 1,
            time_ms: 10,
            best_move: None,
        };
        assert!(
            format_info(&mated).contains(" score mate -1 "),
            "{}",
            format_info(&mated)
        );
    }

    // ── end-to-end protocol ─────────────────────────────────────

    #[test]
    fn usi_handshake_and_options() {
        let (mut e, sink) = engine();
        e.handle("usi");
        let lines = sink.lines();
        assert!(lines
            .iter()
            .any(|l| l.starts_with("id name TaikyokuShogi ")));
        assert!(lines.iter().any(|l| l.starts_with("id author ")));
        assert!(lines.iter().any(|l| l.starts_with("option name Threads ")));
        assert!(lines.iter().any(|l| l.starts_with("option name Hash ")));
        assert!(lines
            .iter()
            .any(|l| l.starts_with("option name NetworkFile ")));
        assert_eq!(lines.last().map(|s| s.as_str()), Some("usiok"));
        e.handle("isready");
        assert_eq!(sink.last(), "readyok");
    }

    #[test]
    fn position_startpos_then_go_depth1_answers_bestmove() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        e.handle("go depth 1");
        e.wait_search();
        let lines = sink.lines();
        assert!(
            lines.iter().any(|l| l.starts_with("info depth 1 ")),
            "per-iteration info expected: {lines:?}"
        );
        let best = lines.iter().find(|l| l.starts_with("bestmove ")).unwrap();
        let token = best.strip_prefix("bestmove ").unwrap().trim();
        let board = Board::initial();
        assert!(parse_move(&board, token).is_ok(), "`{token}` must be legal");
    }

    #[test]
    fn position_sfen_round_trip_and_moves() {
        let (mut e, _sink) = engine();
        let tsfen = Board::initial().to_tsfen();
        // A quiet (non-capturing) move so the piece counts are unchanged.
        let mv = Board::initial()
            .legal_moves()
            .into_iter()
            .find(|m| {
                let r = m.raw();
                r.captured_piece == 0 && r.caps_value == 0 && r.mid_piece == 0
            })
            .expect("the initial position must have a quiet move");
        let token = usi_move_str(mv.raw());
        e.handle(&format!("position sfen {tsfen} moves {token}"));
        // One applied move: the side to move flipped (Black started), the
        // piece counts are intact and the move number is still 1.
        assert_eq!(e.board.side_to_move(), Color::White);
        assert_eq!(e.board.piece_count(Color::Black), 402);
        assert_eq!(e.board.piece_count(Color::White), 402);
        assert_eq!(e.board.move_number(), 1);
    }

    #[test]
    fn position_rejects_9x9_sfen() {
        let (mut e, sink) = engine();
        e.handle("position sfen lnsgkgsnl/1r5b1/ppppppppp/9/9/9/PPPPPPPPP/1B5R1/LNSGKGSNL b - 1");
        assert!(
            sink.lines()
                .iter()
                .any(|l| l.starts_with("info string position rejected")),
            "9x9 SFEN must be rejected: {:?}",
            sink.lines()
        );
    }

    #[test]
    fn stop_ends_an_infinite_search_with_bestmove() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        e.handle("go infinite");
        std::thread::sleep(std::time::Duration::from_millis(50));
        e.handle("stop");
        assert!(
            sink.lines().iter().any(|l| l.starts_with("bestmove ")),
            "stop must produce a bestmove: {:?}",
            sink.lines()
        );
        assert!(e.searcher.is_some(), "the searcher must be recovered");
    }

    #[test]
    fn stop_during_pondering_suppresses_bestmove() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        e.handle("go ponder");
        std::thread::sleep(std::time::Duration::from_millis(50));
        e.handle("stop");
        assert!(
            !sink.lines().iter().any(|l| l.starts_with("bestmove")),
            "a stopped ponder search must not report a move: {:?}",
            sink.lines()
        );
    }

    #[test]
    fn ponderhit_turns_pondering_into_a_normal_search() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        // Small clock so the restarted search answers quickly.
        e.handle("go ponder btime 700");
        std::thread::sleep(std::time::Duration::from_millis(50));
        e.handle("ponderhit");
        e.wait_search();
        assert!(
            sink.lines().iter().any(|l| l.starts_with("bestmove ")),
            "ponderhit must eventually produce a bestmove: {:?}",
            sink.lines()
        );
    }

    #[test]
    fn gameover_suppresses_bestmove() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        e.handle("go infinite");
        std::thread::sleep(std::time::Duration::from_millis(50));
        e.handle("gameover");
        assert!(
            !sink.lines().iter().any(|l| l.starts_with("bestmove")),
            "gameover must not produce a move: {:?}",
            sink.lines()
        );
    }

    #[test]
    fn setoption_changes_threads_and_depth_limit() {
        let (mut e, _sink) = engine();
        e.handle("setoption name Threads value 4");
        assert_eq!(e.threads, 4);
        e.handle("setoption name Threads value 0");
        assert_eq!(e.threads, 4, "invalid value must be ignored");
        e.handle("setoption name DepthLimit value 5");
        assert_eq!(e.depth_limit, 5);
        e.handle("setoption name Ponder value true");
        assert!(e.ponder);
        // An explicit depth beats any time budget, regardless of DepthLimit.
        let p = GoParams {
            depth: Some(2),
            ..Default::default()
        };
        assert_eq!(
            budget_from_params(&p, Color::Black, e.depth_limit, None),
            (2, 0)
        );
    }

    #[test]
    fn mate_go_always_answers() {
        let (mut e, sink) = engine();
        e.handle("position startpos");
        e.handle("go mate");
        e.wait_search();
        assert!(
            sink.lines().iter().any(|l| l.starts_with("bestmove ")),
            "`go mate` must answer: {:?}",
            sink.lines()
        );
    }

    #[test]
    fn usinewgame_clears_and_recovers() {
        let (mut e, _sink) = engine();
        e.handle("position startpos");
        e.handle("go depth 1");
        e.handle("usinewgame");
        assert!(e.job.is_none());
        assert!(e.searcher.is_some());
    }

    #[test]
    fn unknown_commands_are_ignored() {
        let (mut e, sink) = engine();
        e.handle("nonsense command here");
        assert!(sink.lines().is_empty());
    }
}
