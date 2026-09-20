//! Head-to-head strength testing with **Elo estimation and SPRT**.
//!
//! Plays N games between two search configs (alternating colors, seeded
//! opening randomization) and reports, every batch:
//!   - W/L/D score and the Elo difference with a 95% confidence interval
//!   - (optional) GSPRT likelihood ratio for early accept/stop
//!
//! Usage:
//!   cargo run --release --example match_race -- [args]
//!
//! Positional args:  games [depthA] [depthB] [time_ms] [opening_plies]
//! Optional flags:   nnueA        side A uses the NNUE evaluator
//!                   nnueB        side B uses the NNUE evaluator
//!                   sprt=elo0:elo1   run a GSPRT (alpha=0.05, beta=0.10)
//!
//! Examples:
//!   # depth 3 vs depth 2, 100 games, Elo + CI every 10 games
//!   cargo run --release --example match_race -- 100 3 2 2000 6
//!   # SPRT test: is depth 3 >= +10 Elo over depth 2?
//!   cargo run --release --example match_race -- 400 3 2 2000 6 sprt=0:10
//!   # handcrafted vs NNUE (needs TAIKYOKU_NNUE_PATH)
//!   TAIKYOKU_NNUE_PATH=net.nnue cargo run --release --example match_race -- 100 3 3 2000 6 nnueB
//!
//! The game results are deterministic (seeded LCG), so a given invocation
//! is reproducible. For publishable claims use the SPRT mode with standard
//! bounds (elo0=0, elo1=10, alpha=0.05, beta=0.10) and many games.

use taikyokushogi::{Board, GameResult};

// ── Elo / statistics ────────────────────────────────────────────
/// Logistic Elo model: score = 1 / (1 + 10^(-elo/400)).
fn logistic(elo: f64) -> f64 {
    1.0 / (1.0 + 10.0f64.powf(-elo / 400.0))
}

/// Inverted logistic: Elo difference implied by a score in [0,1].
fn elo_from_score(score: f64) -> f64 {
    let s = score.clamp(0.001, 0.999);
    -400.0 * (1.0 / s - 1.0).log10()
}

struct Stats { w: u64, l: u64, d: u64 }

impl Stats {
    fn n(&self) -> u64 { self.w + self.l + self.d }
    fn score(&self) -> f64 {
        let n = self.n() as f64;
        if n == 0.0 { 0.5 } else { (self.w as f64 + 0.5 * self.d as f64) / n }
    }
    /// Unbiased sample variance of per-game results (win=1, draw=0.5, loss=0).
    fn variance(&self) -> f64 {
        let n = self.n() as f64;
        if n < 2.0 { return 0.25; }
        let s = self.score();
        let mut acc = 0.0;
        for (count, value) in [(self.w as f64, 1.0), (self.l as f64, 0.0), (self.d as f64, 0.5)] {
            acc += count * (value - s) * (value - s);
        }
        acc / (n - 1.0)
    }
    /// Elo difference (A minus B) with a 95% CI from the sample variance.
    fn elo_with_ci(&self) -> (f64, f64, f64) {
        let s = self.score();
        let elo = elo_from_score(s);
        let sem_score = (self.variance() / self.n() as f64).sqrt();
        // d(elo)/d(score) = 400 / (ln 10 * s * (1-s))
        let sem_elo = 400.0 / (std::f64::consts::LN_10 * s * (1.0 - s)) * sem_score;
        (elo, elo - 1.96 * sem_elo, elo + 1.96 * sem_elo)
    }
}

/// Generalized SPRT (Fisler/… "Parameterized SPRT" from the gSPRT paper):
/// H0: elo = elo0, H1: elo = elo1. Returns the log-likelihood ratio.
fn gsprt_llr(stats: &Stats, elo0: f64, elo1: f64) -> f64 {
    let n = stats.n() as f64;
    if n == 0.0 { return 0.0; }
    let s = stats.score();
    let var = (stats.w as f64 + 0.25 * stats.d as f64) / n - s * s;
    let s0 = logistic(elo0);
    let s1 = logistic(elo1);
    if var <= 1e-9 || (s1 - s0).abs() < 1e-9 { return 0.0; }
    n * (s1 - s0) * (2.0 * s - s0 - s1) / (2.0 * var)
}

// ── Game playing ────────────────────────────────────────────────
struct Config {
    name: &'static str,
    depth: u32,
    time_ms: u64,
    nnue: bool,
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn pick(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() as usize) % n }
    }
}

const MAX_GAME_PLIES: u32 = 300; // adjudicate longer games as draws

/// Returns 1 if black wins, -1 if white wins, 0 for a draw.
fn play_game(game: usize, cfg_black: &Config, cfg_white: &Config, opening_plies: usize) -> i8 {
    let mut rng = Lcg(0x9E37_79B9_7F4A_7C15 ^ (game as u64).wrapping_mul(0x85EB_CA6B));
    let mut board = Board::initial();

    for _ in 0..opening_plies {
        let moves = board.legal_moves();
        if moves.is_empty() { break; }
        board.apply(&moves[rng.pick(moves.len())]);
    }

    for _ply in 0..MAX_GAME_PLIES {
        if let Some(result) = board.game_result() {
            return match result {
                GameResult::BlackWins => 1,
                GameResult::WhiteWins => -1,
                GameResult::Draw => 0,
            };
        }
        let cfg = if board.side_to_move() == taikyokushogi::Color::Black { cfg_black } else { cfg_white };
        taikyokushogi::set_use_nnue(cfg.nnue);
        let r = board.search(cfg.depth, cfg.time_ms);
        match r.best_move {
            Some(m) => board.apply(&m),
            None => {
                return if board.side_to_move() == taikyokushogi::Color::Black { -1 } else { 1 };
            }
        }
    }
    0
}

// __APPEND__

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut pos: Vec<u64> = Vec::new();
    let mut nnue_a = false;
    let mut nnue_b = false;
    let mut sprt: Option<(f64, f64)> = None;
    for a in &args {
        if let Some(rest) = a.strip_prefix("sprt=") {
            let parts: Vec<&str> = rest.split(':').collect();
            if parts.len() == 2 {
                sprt = Some((parts[0].parse().unwrap_or(0.0), parts[1].parse().unwrap_or(10.0)));
            }
        } else if a.eq_ignore_ascii_case("nnueA") { nnue_a = true; }
        else if a.eq_ignore_ascii_case("nnueB") { nnue_b = true; }
        else if let Ok(v) = a.parse::<u64>() { pos.push(v); }
        else { eprintln!("warning: ignored arg `{}`", a); }
    }
    let games = pos.first().copied().unwrap_or(100) as usize;
    let cfg_a = Config {
        name: "A",
        depth: pos.get(1).copied().unwrap_or(3) as u32,
        time_ms: pos.get(3).copied().unwrap_or(2000),
        nnue: nnue_a,
    };
    let cfg_b = Config {
        name: "B",
        depth: pos.get(2).copied().unwrap_or(2) as u32,
        time_ms: pos.get(3).copied().unwrap_or(2000),
        nnue: nnue_b,
    };
    let opening_plies = pos.get(4).copied().unwrap_or(6) as usize;
    let (elo0, elo1) = sprt.unwrap_or((0.0, 10.0));

    println!(
        "match: {} games | A: d{}{} {}ms | B: d{}{} {}ms | {} opening plies{}",
        games, cfg_a.depth, if cfg_a.nnue { "+nnue" } else { "" }, cfg_a.time_ms,
        cfg_b.depth, if cfg_b.nnue { "+nnue" } else { "" }, cfg_b.time_ms,
        opening_plies,
        sprt.map(|(e0, e1)| format!(" | SPRT [{},{}] a=0.05 b=0.10", e0, e1)).unwrap_or_default(),
    );

    let mut stats = Stats { w: 0, l: 0, d: 0 };
    const BATCH: usize = 10;
    const ALPHA: f64 = 0.05;
    const BETA: f64 = 0.10;
    let upper = ((1.0 - BETA) / ALPHA).ln();
    let lower = (BETA / (1.0 - ALPHA)).ln();

    for game in 0..games {
        let score = if game % 2 == 0 {
            play_game(game, &cfg_a, &cfg_b, opening_plies)
        } else {
            -play_game(game, &cfg_b, &cfg_a, opening_plies)
        };
        match score {
            1 => stats.w += 1,
            -1 => stats.l += 1,
            _ => stats.d += 1,
        }

        if (game + 1) % BATCH == 0 || game + 1 == games {
            let (elo, lo, hi) = stats.elo_with_ci();
            let line = format!(
                "n={:>4}  W-L-D={}-{}-{}  score={:.3}  Elo {:+.1} [{:+.1}, {:+.1}]",
                stats.n(), stats.w, stats.l, stats.d, stats.score(), elo, lo, hi,
            );
            match sprt {
                Some(_) => {
                    let llr = gsprt_llr(&stats, elo0, elo1);
                    println!("{}  LLR={:+.2}", line, llr);
                    if llr >= upper {
                        println!("
=== SPRT: H1 ACCEPTED — {} is stronger (>= +{} Elo) ===", cfg_a.name, elo1);
                        println!("final: {}", line);
                        return;
                    }
                    if llr <= lower {
                        println!("
=== SPRT: H0 ACCEPTED — no evidence that {} is >= +{} Elo ===", cfg_a.name, elo1);
                        println!("final: {}", line);
                        return;
                    }
                }
                None => println!("{}", line),
            }
        }
    }

    let (elo, lo, hi) = stats.elo_with_ci();
    println!("
=== RESULT ===");
    println!("{} (d{}{}): {} wins | {} (d{}{}): {} wins | draws {}",
             cfg_a.name, cfg_a.depth, if cfg_a.nnue { "+nnue" } else { "" }, stats.w,
             cfg_b.name, cfg_b.depth, if cfg_b.nnue { "+nnue" } else { "" }, stats.l, stats.d);
    println!("Elo({} vs {}) = {:+.1}  [95% CI: {:+.1}, {:+.1}]",
             cfg_a.name, cfg_b.name, elo, lo, hi);
    println!("
For a publishable claim: use the sprt=elo0:elo1 mode with many games,");
    println!("or run >= 1000 games and report the confidence interval.");
}
