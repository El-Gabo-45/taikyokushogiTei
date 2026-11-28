#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
//! ATSI v0 protocol adapter.
//!
//! ATSI v0 is the protocol used by `taikyoku-shogi.github.io/play`. It is a
//! thin frontend over the same [`Board`] and [`Searcher`] used by USI: this
//! module only translates text commands and coordinate notation, it does not
//! change move generation, search or evaluation.
//!
//! Supported commands: `atsiinit v0`, `identify`, `time`, `player`,
//! `startgame`, `opmove`, `win`/`loss`/`draw`, `setparam`, `tsfen`, `quit`.
//! The engine answers with `atsiok`, `info "..."`, `move <pos> <pos>`,
//! `eval <cp>` and `log TSFEN: <tsfen>` lines.
//!
//! Run with: `cargo run --bin taikyokushogi-atsi`

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::Board;
use crate::search::Searcher;
use crate::usi::UsiSink;

const BOARD: usize = 36;
/// ATSI reports the starting clock in seconds; this mirrors the original
/// engine's `timeIncrement + startingTime / 900` budget.
const ESTIMATED_GAME_LENGTH: f32 = 900.0;

/// Convert an ATSI position (`19aa`) to a flat square index.
///
/// Files are `36 - column` (1..=36); ranks are `a`..`z` and then doubled
/// letters `aa`..`jj` for the upper half of the 36x36 board.
fn parse_pos(token: &str) -> Option<usize> {
    let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let file: usize = digits.parse().ok()?;
    if !(1..=BOARD).contains(&file) {
        return None;
    }
    let ranks = &token[digits.len()..];
    if ranks.is_empty() || !ranks.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    let first = ranks.as_bytes()[0] - b'a';
    let row = match ranks.len() {
        1 => first as usize,
        2 if ranks.as_bytes()[0] == ranks.as_bytes()[1] => 26 + first as usize,
        _ => return None,
    };
    if row >= BOARD {
        return None;
    }
    Some(row * BOARD + (BOARD - file))
}

/// Flat square index to ATSI position.
fn format_pos(sq: usize) -> String {
    let row = sq / BOARD;
    let col = sq % BOARD;
    let file = BOARD - col;
    let letter = (b'a' + (row % 26) as u8) as char;
    if row >= 26 {
        format!("{file}{letter}{letter}")
    } else {
        format!("{file}{letter}")
    }
}

/// ATSI move string. The optional middle step is emitted as a third position,
/// matching the original protocol's lion move form.
fn format_move(m: &crate::types::Move) -> String {
    let mut out = format!(
        "{} {}",
        format_pos(m.from_sq as usize),
        format_pos(m.to_sq as usize)
    );
    if m.mid_sq != crate::types::INVALID_SQ {
        out.push(' ');
        out.push_str(&format_pos(m.mid_sq as usize));
    }
    out
}

/// ATSI engine state. All protocol handling is isolated here.
pub struct AtsiEngine {
    sink: Arc<dyn UsiSink>,
    board: Board,
    searcher: Searcher,
    depth: u32,
    player: u8,
    starting_time: f32,
    increment: f32,
    initialised: bool,
    quit: bool,
}

impl AtsiEngine {
    #[must_use]
    pub fn new(sink: Arc<dyn UsiSink>) -> Self {
        let depth = std::env::var("TAIKYOKU_ATSI_DEPTH")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3)
            .max(1);
        Self::with_depth(sink, depth)
    }

    #[must_use]
    pub fn with_depth(sink: Arc<dyn UsiSink>, depth: u32) -> Self {
        Self {
            sink,
            board: Board::initial(),
            searcher: Searcher::new(),
            depth,
            player: 0,
            starting_time: 0.0,
            increment: 0.0,
            initialised: false,
            quit: false,
        }
    }

    fn send(&self, line: &str) {
        self.sink.send(line);
    }

    #[must_use]
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// Handle one input line.
    pub fn handle(&mut self, line: &str) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let Some(cmd) = tokens.first().map(|s| s.to_ascii_lowercase()) else {
            return;
        };
        match cmd.as_str() {
            "atsiinit" => {
                if tokens.get(1).is_some_and(|v| *v == "v0") {
                    self.initialised = true;
                    self.send("atsiok");
                } else {
                    self.send("log Unknown ATSI version");
                }
            }
            "identify" => {
                self.send(&format!(
                    "info \"TaikyokuShogi\" \"Taikyoku Shogi engine\" \"El-Gabo-45\" \"{}\"",
                    crate::VERSION
                ));
            }
            "quit" => self.quit = true,
            "tsfen" => self.send(&format!("log TSFEN: {}", self.board.to_tsfen())),
            "time" => {
                self.starting_time = tokens.get(1).and_then(|v| v.parse().ok()).unwrap_or(0.0);
                self.increment = tokens.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.0);
            }
            "player" => {
                if let Some(p) = tokens.get(1).and_then(|v| v.parse::<u8>().ok()) {
                    self.player = p & 1;
                }
            }
            "startgame" => self.cmd_startgame(tokens.get(1).copied()),
            "opmove" => self.cmd_opmove(&tokens),
            "win" | "loss" | "draw" => self.send(&format!("log Game over: {cmd}")),
            // `setparam` is reserved by the original protocol: no parameters
            // are implemented, so it shares the ignore branch with unknowns.
            _ => {}
        }
    }

    fn cmd_startgame(&mut self, position: Option<&str>) {
        let position = position.unwrap_or("initial");
        if position == "initial" {
            self.board = Board::initial();
        } else if let Ok(board) = Board::from_tsfen(position) {
            self.board = board;
        } else {
            self.send("log Invalid TSFEN in startgame");
            return;
        }
        if self.player == 0 {
            self.think();
        }
    }

    fn cmd_opmove(&mut self, tokens: &[&str]) {
        if tokens.len() < 3 {
            return;
        }
        let Some(from) = parse_pos(tokens[1]) else {
            self.send("log Invalid source square");
            return;
        };
        let Some(to) = parse_pos(tokens[2]) else {
            self.send("log Invalid destination square");
            return;
        };
        let mid = tokens.get(5).and_then(|t| parse_pos(t));
        let Some(mv) = find_move(&self.board, from, to, mid) else {
            self.send("log Illegal opponent move");
            return;
        };
        self.board.apply(&mv);
        if self.board.game_result().is_none() {
            self.think();
        }
    }

    fn think(&mut self) {
        let seconds = self.increment + self.starting_time / ESTIMATED_GAME_LENGTH;
        let time_ms = if seconds <= 0.0 {
            1_000
        } else {
            (seconds * 1000.0).clamp(10.0, 60_000.0) as u64
        };
        let result: crate::search::SearchResult =
            self.searcher
                .search(&mut self.board.inner, self.depth, time_ms);
        if let Some(mv) = result.best_move {
            let text = format_move(&mv);
            self.send(&format!("move {text}"));
            self.board.inner.apply_move(&mv);
            self.send(&format!("eval {}", self.board.material_score()));
        } else {
            self.send("log No legal move");
        }
    }
}

/// Find a legal move by endpoints, preferring the highest-value capture when
/// several moves share them (lion/range moves cannot be disambiguated by the
/// ATSI text form alone).
fn find_move(board: &Board, from: usize, to: usize, mid: Option<usize>) -> Option<crate::Move> {
    board
        .legal_moves()
        .into_iter()
        .filter(|m| {
            let raw = m.raw();
            raw.from_sq as usize == from
                && raw.to_sq as usize == to
                && mid.is_none_or(|want| m.mid_sq() == Some(want))
        })
        .max_by_key(|m| m.raw().caps_value)
}

/// Run the ATSI v0 frontend over stdin/stdout.
pub fn run() {
    let sink: Arc<dyn UsiSink> = Arc::new(StdoutSink);
    let mut engine = AtsiEngine::new(sink);
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        engine.handle(&line);
        io::stdout().flush().ok();
        if engine.quit_requested() {
            break;
        }
    }
}

struct StdoutSink;

impl UsiSink for StdoutSink {
    fn send(&self, line: &str) {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Collector(Mutex<Vec<String>>);
    impl UsiSink for Collector {
        fn send(&self, line: &str) {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(line.to_owned());
        }
    }

    fn lines(c: &Collector) -> Vec<String> {
        c.0.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    #[test]
    fn position_roundtrip() {
        for sq in [0, 35, 36, 900, 1295] {
            assert_eq!(parse_pos(&format_pos(sq)), Some(sq));
        }
    }

    #[test]
    fn handshake_identify_and_quit() {
        let sink = Arc::new(Collector::default());
        let mut engine = AtsiEngine::with_depth(sink.clone(), 1);
        engine.handle("atsiinit v0");
        engine.handle("identify");
        engine.handle("quit");
        let out = lines(&sink);
        assert_eq!(out[0], "atsiok");
        assert!(out[1].starts_with("info \""));
        assert!(engine.quit_requested());
    }

    #[test]
    fn startgame_answers_a_move() {
        let sink = Arc::new(Collector::default());
        let mut engine = AtsiEngine::with_depth(sink.clone(), 1);
        engine.handle("atsiinit v0");
        engine.handle("time 60 1");
        engine.handle("player 0");
        engine.handle("startgame initial");
        let out = lines(&sink);
        assert!(out.iter().any(|l| l.starts_with("move ")), "{out:?}");
    }
}
