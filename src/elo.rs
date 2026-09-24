//! Elo measurement and statistical testing tools.
//!
//! Provides everything needed to measure engine strength changes:
//!
//! * [`expected_score`] / [`elo_from_score`] — the logistic Elo model.
//! * [`EngineConfig`] — one engine side of a match (depth / movetime / NNUE).
//! * [`play_game`] — play a single engine-vs-engine game.
//! * [`run_match`] — play `N` games (colors alternate) and report W/D/L plus
//!   the Elo difference of A over B with a 95% confidence interval.
//! * [`Sprt`] — sequential probability ratio test (the standard tool for
//!   accepting/rejecting Elo improvements, as used by fishtest).
//! * [`run_sprt`] — run a match under SPRT, stopping as soon as the test
//!   accepts H0 or H1.

use crate::board::Board;
use crate::search;
use crate::types::Move;
use crate::GameResult;

// ============================================================
// Elo math
// ============================================================

/// Logistic model: expected score of A against B for an Elo difference `d`.
///
/// ```rust
/// use taikyokushogi::elo::expected_score;
/// assert!((expected_score(0.0) - 0.5).abs() < 1e-9);
/// assert!((expected_score(400.0) - 10.0f64 / 11.0).abs() < 1e-9);
/// ```
pub fn expected_score(d: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-d / 400.0))
}

/// Inverse of [`expected_score`]: the Elo difference implied by an average
/// score `s` in `[0, 1]`. Clamped so `s = 0` or `s = 1` saturate to ±1200
/// instead of producing infinities.
pub fn elo_from_score(s: f64) -> f64 {
    let s = s.clamp(1e-6, 1.0 - 1e-6);
    -400.0 * (1.0 / s - 1.0).log10()
}

/// A win/draw/loss tally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wdl {
    pub wins: u64,
    pub draws: u64,
    pub losses: u64,
}

impl Wdl {
    pub fn total(&self) -> u64 {
        self.wins + self.draws + self.losses
    }

    /// Average score in points (win = 1, draw = 0.5, loss = 0).
    pub fn score(&self) -> f64 {
        let n = self.total();
        if n == 0 { return 0.5; }
        (self.wins as f64 + 0.5 * self.draws as f64) / n as f64
    }

    /// Variance of the per-game score (trinomial).
    pub fn variance(&self) -> f64 {
        let n = self.total();
        if n == 0 { return 0.0; }
        let s = self.score();
        (self.wins as f64 + 0.25 * self.draws as f64) / n as f64 - s * s
    }

    /// Elo difference of A over B.
    pub fn elo(&self) -> f64 {
        elo_from_score(self.score())
    }

    /// 95% half-width confidence interval of [`Wdl::elo`], in Elo points.
    /// Uses the delta method on the logistic transform; degenerate score
    /// distributions (all wins / all losses) fall back to a conservative
    /// estimate so the interval stays finite.
    pub fn elo_margin95(&self) -> f64 {
        let n = self.total();
        if n == 0 { return 0.0; }
        let s = self.score();
        let var = self.variance();
        if var <= 0.0 {
            // Deterministic outcome: pretend the score had a small spread.
            let fake = 0.25 / (n as f64 + 1.0);
            return (1.96 * fake.sqrt() * 400.0 / (std::f64::consts::LN_10 * s * (1.0 - s))).min(2000.0);
        }
        1.96 * (var / n as f64).sqrt() * (400.0 / (std::f64::consts::LN_10 * s * (1.0 - s)))
    }

    pub fn elo_interval95(&self) -> (f64, f64) {
        let e = self.elo();
        let m = self.elo_margin95();
        (e - m, e + m)
    }
}

// ============================================================
// Engine configuration & game playing
// ============================================================

/// One engine side of a match.
#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    /// Search depth in plies.
    pub depth: u32,
    /// Wall-clock time budget per move in ms (0 = unlimited).
    pub time_ms: u64,
    /// Use the NNUE evaluator instead of the hand-crafted one.
    pub nnue: bool,
}

impl EngineConfig {
    pub fn depth(d: u32) -> Self {
        EngineConfig { depth: d, time_ms: 0, nnue: false }
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig { depth: 2, time_ms: 0, nnue: false }
    }
}

/// Outcome of one played game. `result` is the actual game result (which
/// SIDE won); callers map it to "engine A won / lost" using the colors.
#[derive(Debug, Clone, Copy)]
pub struct GameOutcome {
    pub result: GameResult,
    /// Plies actually played.
    pub plies: u32,
}

fn apply_best(board: &mut Board, cfg: &EngineConfig) -> Option<Move> {
    crate::set_use_nnue(cfg.nnue);
    let r = search::search(board, cfg.depth, cfg.time_ms);
    r.best_move
}

/// Play a single engine-vs-engine game from the initial position.
///
/// `white` and `black` describe the engine settings for each side. The game
/// ends when the rules end it, or after `max_plies` plies (`0` = unlimited),
/// in which case it is scored as a draw.
pub fn play_game(white: &EngineConfig, black: &EngineConfig, max_plies: u32) -> GameOutcome {
    let mut board = Board::new();
    board.setup_initial();
    let mut plies: u32 = 0;
    loop {
        if let Some(r) = board.game_result() {
            return GameOutcome { result: to_public(r), plies };
        }
        if max_plies > 0 && plies >= max_plies {
            return GameOutcome { result: GameResult::Draw, plies };
        }
        let cfg = if board.side_to_move == 0 { white } else { black };
        match apply_best(&mut board, cfg) {
            Some(m) => board.apply_move(&m),
            None => {
                let r = board.game_result().map_or(GameResult::Draw, to_public);
                return GameOutcome { result: r, plies };
            }
        }
        plies += 1;
    }
}

/// Convert the internal game result to the public enum.
fn to_public(r: crate::types::GameResult) -> GameResult {
    match r {
        crate::types::GameResult::BlackWins => GameResult::BlackWins,
        crate::types::GameResult::WhiteWins => GameResult::WhiteWins,
        crate::types::GameResult::Draw => GameResult::Draw,
    }
}

/// Map a game result to a win/draw/loss point from A's perspective, given
/// whether A played White in that game.
fn points_from_a(game: GameResult, a_was_white: bool) -> (u64, u64, u64) {
    let a_won = match game {
        GameResult::BlackWins => !a_was_white,
        GameResult::WhiteWins => a_was_white,
        GameResult::Draw => false,
    };
    let draw = game == GameResult::Draw;
    match (a_won, draw) {
        (true, _) => (1, 0, 0),
        (false, true) => (0, 1, 0),
        (false, false) => (0, 0, 1),
    }
}

// ============================================================
// Match
// ============================================================

/// Callback invoked after every finished game:
/// `(game_index, game_result, a_was_white)`.
pub type GameCallback = fn(usize, GameResult, bool);

/// Configuration for a full match.
#[derive(Clone)]
pub struct MatchConfig {
    /// Number of games (colors alternate: A is White on even games).
    pub games: usize,
    pub a: EngineConfig,
    pub b: EngineConfig,
    /// Hard ply cap per game (0 = unlimited).
    pub max_plies: u32,
    /// Optional progress callback (e.g. for streaming reports from the CLI).
    pub on_game: Option<GameCallback>,
}

impl Default for MatchConfig {
    fn default() -> Self {
        MatchConfig {
            games: 10,
            a: EngineConfig::depth(2),
            b: EngineConfig::depth(3),
            max_plies: 400,
            on_game: None,
        }
    }
}

/// Aggregate result of a match, from A's perspective.
#[derive(Debug, Clone)]
pub struct MatchReport {
    pub games: usize,
    pub wdl: Wdl,
    /// Elo of A over B.
    pub elo: f64,
    /// 95% confidence half-width.
    pub margin95: f64,
    /// Plies per game (for diagnostics).
    pub plies: Vec<u32>,
}

impl MatchReport {
    pub fn print(&self) {
        let w = self.wdl;
        println!("=== Match Report ===");
        println!("Games: {}   A: {} - {} - {} B", self.games, w.wins, w.draws, w.losses);
        println!("Score: {:.3} / {:.1}", w.score(), 0.5 * self.games as f64);
        let (lo, hi) = w.elo_interval95();
        println!("Elo:   {:+.1} ± {:.1} (95%)  [{:+.1}, {:+.1}]", self.elo, self.margin95, lo, hi);
        if !self.plies.is_empty() {
            let avg: f64 = self.plies.iter().map(|&p| p as f64).sum::<f64>() / self.plies.len() as f64;
            println!("Avg plies/game: {:.1}", avg);
        }
    }
}

/// Run a full match and return the aggregate report.
pub fn run_match(cfg: &MatchConfig) -> MatchReport {
    let mut wdl = Wdl::default();
    let mut plies = Vec::with_capacity(cfg.games);
    for g in 0..cfg.games {
        let a_was_white = g % 2 == 0;
        let (white, black) = if a_was_white { (cfg.a, cfg.b) } else { (cfg.b, cfg.a) };
        let outcome = play_game(&white, &black, cfg.max_plies);
        let (w, d, l) = points_from_a(outcome.result, a_was_white);
        wdl.wins += w;
        wdl.draws += d;
        wdl.losses += l;
        plies.push(outcome.plies);
        if let Some(cb) = cfg.on_game {
            cb(g, outcome.result, a_was_white);
        }
    }
    MatchReport {
        games: cfg.games,
        elo: wdl.elo(),
        margin95: wdl.elo_margin95(),
        wdl,
        plies,
    }
}

// ============================================================
// SPRT
// ============================================================

/// SPRT hypothesis bounds: H0 = "elo ≤ elo0", H1 = "elo ≥ elo1".
#[derive(Debug, Clone, Copy)]
pub struct Sprt {
    pub elo0: f64,
    pub elo1: f64,
    /// Type-I error probability (default 0.05).
    pub alpha: f64,
    /// Type-II error probability (default 0.05).
    pub beta: f64,
}

impl Default for Sprt {
    fn default() -> Self {
        Sprt { elo0: 0.0, elo1: 10.0, alpha: 0.05, beta: 0.05 }
    }
}

impl Sprt {
    fn bounds(&self) -> (f64, f64) {
        (self.beta.ln() - (1.0 - self.alpha).ln(),
         (1.0 - self.beta).ln() - self.alpha.ln())
    }

    /// Log-likelihood ratio after the given WDL, under the Gaussian
    /// approximation used by fishtest / bayeselo.
    pub fn llr(&self, wdl: &Wdl) -> f64 {
        let n = wdl.total();
        if n == 0 { return 0.0; }
        let e0 = expected_score(self.elo0);
        let e1 = expected_score(self.elo1);
        let s = wdl.score();
        let var = wdl.variance().max(0.25 / n as f64);
        (e1 - e0) * (2.0 * s - e0 - e1) * n as f64 / (2.0 * var)
    }

    /// Current decision.
    pub fn decision(&self, wdl: &Wdl) -> SprtDecision {
        let (lb, ub) = self.bounds();
        let llr = self.llr(wdl);
        if llr >= ub { SprtDecision::AcceptH1 }
        else if llr <= lb { SprtDecision::AcceptH0 }
        else { SprtDecision::Continue }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SprtDecision {
    /// The engine is NOT better than elo0 — reject the patch.
    AcceptH0,
    /// The engine IS at least elo1 — accept the patch.
    AcceptH1,
    /// Keep playing.
    Continue,
}

impl std::fmt::Display for SprtDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SprtDecision::AcceptH0 => write!(f, "H0 accepted (no improvement)"),
            SprtDecision::AcceptH1 => write!(f, "H1 accepted (improvement confirmed)"),
            SprtDecision::Continue => write!(f, "continue"),
        }
    }
}

/// Report of a completed SPRT run.
#[derive(Debug, Clone)]
pub struct SprtReport {
    pub games: usize,
    pub wdl: Wdl,
    pub llr: f64,
    pub decision: SprtDecision,
    pub elo: f64,
}

impl SprtReport {
    pub fn print(&self) {
        println!("=== SPRT Report ===");
        println!("Games: {}   WDL: {} - {} - {}", self.games, self.wdl.wins, self.wdl.draws, self.wdl.losses);
        println!("Score: {:.3}   Elo: {:+.1}", self.wdl.score(), self.elo);
        println!("LLR:   {:+.2}  ->  {}", self.llr, self.decision);
    }
}

/// Run games until SPRT decides or `cfg.games` are exhausted.
pub fn run_sprt(cfg: &MatchConfig, sprt: &Sprt) -> SprtReport {
    let mut wdl = Wdl::default();
    let mut games = 0usize;
    for g in 0..cfg.games {
        let a_was_white = g % 2 == 0;
        let (white, black) = if a_was_white { (cfg.a, cfg.b) } else { (cfg.b, cfg.a) };
        let outcome = play_game(&white, &black, cfg.max_plies);
        let (w, d, l) = points_from_a(outcome.result, a_was_white);
        wdl.wins += w;
        wdl.draws += d;
        wdl.losses += l;
        games += 1;
        if let Some(cb) = cfg.on_game { cb(g, outcome.result, a_was_white); }
        if sprt.decision(&wdl) != SprtDecision::Continue { break; }
    }
    SprtReport {
        games,
        elo: wdl.elo(),
        llr: sprt.llr(&wdl),
        decision: sprt.decision(&wdl),
        wdl,
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn expected_score_basics() {
        assert!(close(expected_score(0.0), 0.5, 1e-12));
        // +400 Elo → 10:1 odds
        assert!(close(expected_score(400.0), 10.0 / 11.0, 1e-9));
        // -400 Elo → 1:10 odds
        assert!(close(expected_score(-400.0), 1.0 / 11.0, 1e-9));
        // monotone increasing and antisymmetric
        for d in [50.0, 100.0, 250.0] {
            assert!(expected_score(d) > expected_score(0.0));
            assert!(close(expected_score(d) + expected_score(-d), 1.0, 1e-9));
        }
    }

    #[test]
    fn elo_from_score_is_inverse() {
        assert!(close(elo_from_score(0.5), 0.0, 1e-9));
        for d in [-300.0, -100.0, 0.0, 100.0, 300.0] {
            assert!(close(elo_from_score(expected_score(d)), d, 1e-6));
        }
        // saturates instead of panicking on degenerate scores
        assert!(elo_from_score(1.0) > 800.0);
        assert!(elo_from_score(0.0) < -800.0);
    }

    #[test]
    fn wdl_score_and_elo() {
        let w = Wdl { wins: 10, draws: 5, losses: 5 };
        assert_eq!(w.total(), 20);
        assert!(close(w.score(), 0.625, 1e-12));
        assert!(w.elo() > 0.0);
        assert!(close(w.variance(), 10.0 / 20.0 + 0.25 * 5.0 / 20.0 - 0.625 * 0.625, 1e-12));
        let balanced = Wdl { wins: 5, draws: 10, losses: 5 };
        assert!(close(balanced.elo(), 0.0, 1e-9));
        // empty tally is well-defined
        let empty = Wdl::default();
        assert!(close(empty.score(), 0.5, 1e-12));
        assert_eq!(empty.elo_margin95(), 0.0);
    }

    #[test]
    fn confidence_interval_shrinks_and_centers() {
        let few = Wdl { wins: 6, draws: 2, losses: 2 };
        let many = Wdl { wins: 600, draws: 200, losses: 200 };
        assert!(many.elo_margin95() < few.elo_margin95());
        let (lo, hi) = many.elo_interval95();
        assert!(lo < many.elo() && many.elo() < hi);
        // degenerate outcomes don't produce NaN / infinity
        let all_wins = Wdl { wins: 100, draws: 0, losses: 0 };
        assert!(all_wins.elo_margin95().is_finite());
        let (lo, hi) = all_wins.elo_interval95();
        assert!(lo.is_finite() && hi.is_finite());
        assert!(all_wins.elo() > 0.0);
    }

    #[test]
    fn sprt_decisions() {
        let s = Sprt::default();
        // crushing loss → H0; crushing win → H1; too little data → continue.
        let lost_all = Wdl { wins: 0, draws: 0, losses: 30 };
        assert_eq!(s.decision(&lost_all), SprtDecision::AcceptH0);
        let won_all = Wdl { wins: 30, draws: 0, losses: 0 };
        assert_eq!(s.decision(&won_all), SprtDecision::AcceptH1);
        let even = Wdl { wins: 1, draws: 0, losses: 1 };
        assert_eq!(s.decision(&even), SprtDecision::Continue);
        // LLR is monotone in the score
        let weak = Wdl { wins: 4, draws: 2, losses: 4 };
        let strong = Wdl { wins: 6, draws: 2, losses: 2 };
        assert!(s.llr(&strong) > s.llr(&weak));
        // alpha = beta = 0.05 → symmetric bounds ±ln(19)
        let (lb, ub) = s.bounds();
        assert!(close(ub, 19f64.ln(), 1e-9));
        assert!(close(lb, -19f64.ln(), 1e-9));
        // no data → continue
        assert_eq!(s.decision(&Wdl::default()), SprtDecision::Continue);
    }

    #[test]
    fn points_from_a_mapping() {
        use GameResult::*;
        // A is White: WhiteWins = win for A; BlackWins = loss.
        assert_eq!(points_from_a(WhiteWins, true), (1, 0, 0));
        assert_eq!(points_from_a(BlackWins, true), (0, 0, 1));
        assert_eq!(points_from_a(Draw, true), (0, 1, 0));
        // A is Black: mirrored.
        assert_eq!(points_from_a(BlackWins, false), (1, 0, 0));
        assert_eq!(points_from_a(WhiteWins, false), (0, 0, 1));
        assert_eq!(points_from_a(Draw, false), (0, 1, 0));
    }

    #[test]
    fn play_game_returns_valid_outcome() {
        // `play_game` sets the process-wide evaluation backend from the config
        // (elo::apply_best), and this test compares two games move-for-move: run
        // alone (see crate::test_lock).
        let _serial = crate::test_lock::lock();
        let a = EngineConfig::depth(1);
        let b = EngineConfig::depth(1);
        let o = play_game(&a, &b, 6); // ply-capped at 6 → draw
        assert_eq!(o.result, GameResult::Draw);
        assert_eq!(o.plies, 6);
        // unlimited cap must terminate on its own (same deterministic game)
        let o2 = play_game(&a, &b, 0);
        assert!(o2.plies > 6, "uncapped game should play past the cap");
    }

    #[test]
    fn match_smoke_and_alternation() {
        // See play_game_returns_valid_outcome: run alone (crate::test_lock).
        let _serial = crate::test_lock::lock();
        let cfg = MatchConfig {
            games: 2,
            a: EngineConfig::depth(1),
            b: EngineConfig::depth(1),
            max_plies: 4,
            on_game: None,
        };
        let r = run_match(&cfg);
        assert_eq!(r.games, 2);
        assert_eq!(r.wdl.total(), 2);
        // every game ply-capped → all draws → 0 Elo exactly
        assert_eq!(r.wdl.draws, 2);
        assert!(r.elo.abs() < 1e-9);
        assert!(r.plies.iter().all(|&p| p == 4));
        assert!(r.margin95.is_finite());
    }

    #[test]
    fn match_callback_receives_every_game() {
        // See play_game_returns_valid_outcome: run alone (crate::test_lock).
        let _serial = crate::test_lock::lock();
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        fn cb(_g: usize, _r: GameResult, _a_white: bool) {
            COUNT.fetch_add(1, Ordering::SeqCst);
        }
        let cfg = MatchConfig {
            games: 3,
            a: EngineConfig::depth(1),
            b: EngineConfig::depth(1),
            max_plies: 2,
            on_game: Some(cb),
        };
        let before = COUNT.load(Ordering::SeqCst);
        let r = run_match(&cfg);
        assert_eq!(r.games, 3);
        assert_eq!(COUNT.load(Ordering::SeqCst) - before, 3);
    }

    #[test]
    fn sprt_stops_early_on_trivial_results() {
        // See play_game_returns_valid_outcome: run alone (crate::test_lock).
        let _serial = crate::test_lock::lock();
        let cfg = MatchConfig {
            games: 200,
            a: EngineConfig::depth(1),
            b: EngineConfig::depth(1),
            max_plies: 2, // every game is a 2-ply draw → SPRT converges to H0 fast
            on_game: None,
        };
        let r = run_sprt(&cfg, &Sprt::default());
        assert!(r.games < 200, "SPRT should stop early on all-draws, took {} games", r.games);
        assert_eq!(r.decision, SprtDecision::AcceptH0);
        assert_eq!(r.wdl.total(), r.games as u64);
    }
}
