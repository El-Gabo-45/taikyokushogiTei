//! # Taikyoku Shogi Engine
//!
//! A complete engine for [Taikyoku Shogi](https://en.wikipedia.org/wiki/Taikyoku_shogi),
//! the largest known variant of shogi (Japanese chess).
//!
//! - **36 x 36** board (1,296 squares)
//! - **804 pieces** (402 per side)
//! - **209 initial piece types** / **92 promoted forms** (301 piece types)
//!
//! ## Quick Start
//!
//! ```rust
//! use taikyokushogi::Board;
//!
//! let mut board = Board::initial();
//! assert_eq!(board.piece_count(taikyokushogi::Color::Black), 402);
//!
//! let moves = board.legal_moves();
//! println!("{} legal moves from starting position", moves.len()); // 512
//!
//! // Apply the first move
//! board.apply(&moves[0]);
//! println!("Score: {}", board.material_score());
//!
//! // Undo
//! board.undo();
//! ```
//!
//! ## Search
//!
//! Single-threaded by default (depth + optional time limit in ms,
//! `0 = no limit`):
//!
//! ```rust,no_run
//! use taikyokushogi::Board;
//!
//! let mut board = Board::initial();
//! let result = board.search(2, 5000); // depth 2, 5s time limit
//! if let Some(mv) = result.best_move {
//!     println!("Best move: {}, score: {}", mv, result.score);
//! }
//! ```
//!
//! Lazy SMP workers are **opt-in** per call (`1..=`
//! [`search::MAX_THREADS`], default [`search::DEFAULT_THREADS`] = 1) and
//! share one transposition table; results with `threads > 1` are not
//! bit-reproducible:
//!
//! ```rust,no_run
//! use taikyokushogi::Board;
//!
//! let mut board = Board::initial();
//! let result = board.search_threads(3, 5000, 4); // depth 3, 5s, 4 workers
//! println!("nodes = {}", result.nodes);
//!
//! // Transposition-table size in MiB (returns what was actually allocated)
//! let mb = taikyokushogi::set_hash_mb(64);
//! ```

mod attack;
mod board;
mod eval;
mod movegen;
mod pieces;
pub mod search;
mod types;

/// Centralized correctness suite (runs with `cargo test`). Lives inside the
/// crate because the release profile uses `panic = "abort"`, which external
/// integration tests in `tests/` cannot link against.
#[cfg(test)]
mod correctness_tests;

/// Serialises the tests that touch the engine's process-wide state.
///
/// `cargo test` runs the test cases in parallel threads of ONE process, but the
/// engine deliberately shares state across calls: the evaluation backend
/// (hand-crafted vs NNUE), the transposition table and the killer/history
/// tables. Two tests that search at the same time therefore influence each
/// other's scores, node counts and move choices, which surfaces as seemingly
/// random failures in tests that assert on a result. Every such test holds this
/// guard for its whole duration.
#[cfg(test)]
pub(crate) mod test_lock {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
mod bitboard;
pub mod debugging;
pub mod elo;
pub mod selfplay;
#[cfg(feature = "server")]
pub mod server;
mod tsfen;

/// USI (Universal Shogi Interface) front-end: the [`usi`] module speaks the
/// standard USI command set over stdin/stdout so the engine can be loaded by
/// USI GUIs and test harnesses (bin `taikyokushogi-usi`).
pub mod usi;

#[cfg(feature = "python")]
mod python;

// ============================================================
// Public re-exports — the user-facing API
// ============================================================

/// Board size (36).
pub const BOARD_SIZE: usize = types::BOARD_SIZE;

/// Number of squares (1,296).
pub const NUM_SQUARES: usize = types::NUM_SQUARES;

/// Crate version, exposed through the REST API (`/api/health`, `/api/engine-info`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Side / player color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color {
    Black = 0,
    White = 1,
}

impl Color {
    /// The other color.
    pub fn opponent(self) -> Self {
        match self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }

    pub(crate) fn raw(self) -> u8 {
        self as u8
    }

    /// Convert a raw `u8` (0 = Black, 1 = White) to a `Color`.
    /// Returns `None` for any other value.
    pub fn from_raw(v: u8) -> Option<Self> {
        match v {
            0 => Some(Color::Black),
            1 => Some(Color::White),
            _ => None,
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Color::Black => write!(f, "Black"),
            Color::White => write!(f, "White"),
        }
    }
}

/// Result of a finished game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameResult {
    BlackWins,
    WhiteWins,
    Draw,
}

impl std::fmt::Display for GameResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            GameResult::BlackWins => write!(f, "Black wins"),
            GameResult::WhiteWins => write!(f, "White wins"),
            GameResult::Draw => write!(f, "Draw"),
        }
    }
}

/// A square on the 36x36 board, represented as `(row, col)`.
///
/// - Row 0 = top of the board (White's back rank)
/// - Row 35 = bottom (Black's back rank)
/// - Col 0 = left, Col 35 = right
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square {
    pub row: usize,
    pub col: usize,
}

impl Square {
    pub fn new(row: usize, col: usize) -> Self {
        Square { row, col }
    }

    pub(crate) fn from_index(idx: usize) -> Self {
        Square {
            row: idx / BOARD_SIZE,
            col: idx % BOARD_SIZE,
        }
    }

    /// Convert to flat index (row * 36 + col).
    pub fn index(&self) -> usize {
        self.row * BOARD_SIZE + self.col
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "({},{})", self.row, self.col)
    }
}

/// A piece on the board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Internal piece type ID.
    pub(crate) type_id: u16,
    /// Color of this piece.
    pub color: Color,
}

impl Piece {
    /// Abbreviation (e.g., `"K"`, `"CP"`, `"GG"`).
    pub fn abbrev(&self) -> &'static str {
        pieces::abbrev(self.type_id)
    }

    /// Full English name.
    pub fn name(&self) -> &'static str {
        pieces::name(self.type_id)
    }

    /// Material value.
    pub fn value(&self) -> i32 {
        pieces::value(self.type_id)
    }

    /// Whether this is a royal piece (King or Crown Prince).
    pub fn is_royal(&self) -> bool {
        pieces::is_royal(self.type_id)
    }

    /// What this piece promotes to, if anything.
    pub fn promotes_to(&self) -> Option<&'static str> {
        pieces::promotes_to(self.type_id).map(pieces::abbrev)
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let prefix = if self.color == Color::White { 'v' } else { '^' };
        write!(f, "{}{}", prefix, self.abbrev())
    }
}

/// A move in the game.
///
/// `types::Move` is `Copy` (all fields are primitives), so this wrapper is
/// `Copy` too — no spurious clones when moving `Move` values around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    inner: types::Move,
}

impl Move {
    /// Source square.
    pub fn from(&self) -> Square {
        Square::from_index(self.inner.from_sq as usize)
    }

    /// Destination square.
    pub fn to(&self) -> Square {
        Square::from_index(self.inner.to_sq as usize)
    }

    /// Whether this move promotes the piece.
    pub fn is_promotion(&self) -> bool {
        self.inner.promotion
    }

    /// Whether this is an igui (capture without moving).
    pub fn is_igui(&self) -> bool {
        self.inner.is_igui
    }

    /// The captured piece's abbreviation, if any.
    pub fn captured(&self) -> Option<&'static str> {
        if self.inner.captured_piece != 0 {
            Some(pieces::abbrev(self.inner.captured_piece))
        } else {
            None
        }
    }

    /// Access the internal move representation.
    pub fn raw(&self) -> &types::Move {
        &self.inner
    }

    /// The mid-capture square for igui/range moves, if any.
    pub fn mid_sq(&self) -> Option<usize> {
        (self.inner.mid_sq != types::INVALID_SQ).then_some(self.inner.mid_sq as usize)
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}->{}", self.from(), self.to())?;
        if self.is_promotion() {
            write!(f, "+")?;
        }
        Ok(())
    }
}

/// Error returned when applying a move fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    /// The coordinates are outside the 36x36 board.
    OutOfBounds,
    /// No legal move matches the given coordinates and options.
    Illegal,
    /// More than one legal move matches (ambiguous — e.g. an igui and a
    /// normal move to the same square). Disambiguate with `is_igui`.
    Ambiguous,
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MoveError::OutOfBounds => write!(f, "coordinates out of bounds"),
            MoveError::Illegal => write!(f, "illegal move"),
            MoveError::Ambiguous => write!(f, "ambiguous move — multiple legal moves match"),
        }
    }
}

impl std::error::Error for MoveError {}

impl std::str::FromStr for Move {
    type Err = MoveError;

    /// Parse a move from a string like `"f19f20"`, `"f19f20+"`, or
    /// `"c5c5"` (igui). Returns [`MoveError::Illegal`] on failure.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() < 4 {
            return Err(MoveError::Illegal);
        }
        let promotion = s.ends_with('+');
        let core = if promotion { &s[..s.len() - 1] } else { s };
        let is_igui = core.contains('=');
        let parts: Vec<&str> = if is_igui {
            core.split('=').collect()
        } else if core.len() >= 4 {
            vec![&core[..2], &core[2..4]]
        } else {
            return Err(MoveError::Illegal);
        };
        if parts.len() != 2 {
            return Err(MoveError::Illegal);
        }
        let from_sq = parse_square(parts[0]).ok_or(MoveError::Illegal)?;
        let to_sq = parse_square(parts[1]).ok_or(MoveError::Illegal)?;
        Ok(Move {
            inner: types::Move {
                from_sq: from_sq.index() as u16,
                to_sq: to_sq.index() as u16,
                promotion,
                captured_piece: 0,
                captured_color: 0,
                is_igui,
                mid_sq: types::INVALID_SQ,
                mid_piece: 0,
                mid_color: 0,
                range_cap: false,
                caps_value: 0,
            },
        })
    }
}

/// Parse a two-character square like `"f19"` into a [`Square`].
fn parse_square(s: &str) -> Option<Square> {
    if s.len() < 2 {
        return None;
    }
    let col_char = s.chars().next()?;
    let row_str = &s[1..];
    let row: usize = row_str.parse().ok()?;
    let col = match col_char {
        'a'..='z' => col_char as usize - 'a' as usize,
        'A'..='J' => 26 + (col_char as usize - 'A' as usize),
        _ => return None,
    };
    if row >= BOARD_SIZE || col >= BOARD_SIZE {
        return None;
    }
    Some(Square::new(row, col))
}

/// Result of a search.
pub struct SearchResult {
    /// Best move found, if any.
    pub best_move: Option<Move>,
    /// Evaluation score (positive = good for side to move).
    pub score: i32,
    /// Number of nodes searched.
    pub nodes: u64,
    /// Wall-clock time in milliseconds.
    pub time_ms: u64,
}

/// Information about a piece type.
pub struct PieceInfo {
    pub abbrev: &'static str,
    pub name: &'static str,
    pub value: i32,
    pub promotes_to: Option<&'static str>,
    pub slide_directions: usize,
    pub jump_destinations: usize,
    pub has_hook: bool,
    pub area_steps: u8,
    pub has_range_capture: bool,
    pub has_igui: bool,
    /// Whether capturing this piece ends the game (King, Crown Prince, …).
    pub is_royal: bool,
    /// Betza movement notation, when the piece database defines one.
    pub betza: Option<&'static str>,
}

/// Look up information about a piece type by abbreviation.
///
/// ```rust
/// let info = taikyokushogi::piece_info("K").unwrap();
/// assert_eq!(info.name, "King");
/// assert!(info.value > 0);
/// ```
pub fn piece_info(abbrev: &str) -> Option<PieceInfo> {
    let pt = pieces::find_by_abbrev(abbrev)?;
    Some(piece_info_by_id(pt))
}

/// Look up information about a piece type by its numeric ID (1-based).
pub fn piece_info_by_id(pt: u16) -> PieceInfo {
    let mv = pieces::movement(pt);
    let promo = pieces::promotes_to(pt).map(pieces::abbrev);
    PieceInfo {
        abbrev: pieces::abbrev(pt),
        name: pieces::name(pt),
        value: pieces::value(pt),
        promotes_to: promo,
        slide_directions: mv.slides.len(),
        jump_destinations: mv.jumps.len(),
        has_hook: mv.hook.is_some(),
        area_steps: mv.area,
        has_range_capture: !mv.range_capture.is_empty(),
        has_igui: mv.igui,
        is_royal: pieces::is_royal(pt),
        betza: pieces::betza(pt),
    }
}

/// Every defined piece type, in ID order (used by `GET /api/pieces`).
///
/// ```rust
/// let all = taikyokushogi::all_pieces();
/// assert_eq!(all.len(), taikyokushogi::num_piece_types());
/// assert!(all.iter().any(|p| p.abbrev == "K" && p.is_royal));
/// ```
pub fn all_pieces() -> Vec<PieceInfo> {
    (1..=pieces::num_piece_types() as u16)
        .map(piece_info_by_id)
        .collect()
}

/// Total number of defined piece types (base + promoted).
pub fn num_piece_types() -> usize {
    pieces::num_piece_types()
}

pub use eval::nnue;
pub use eval::nnue::{Network, NnueError};

/// Score bound the search reserves for a forced win: a score `s` with
/// `|s| >= MATE_SCORE - plies` means the side to move (positive `s`) forces
/// the capture of the opponent's last royal piece `plies` plies away
/// (negative: it is getting captured). Protocol front-ends map this to
/// `score mate <moves>`; this variant has no checks, so "mate" here is always
/// the direct royal-capture line.
pub use eval::MATE_SCORE;

/// Resize the transposition table to `mb` MiB (see `search::set_hash_mb`).
/// Returns the size actually allocated, which is rounded down to a
/// power-of-two bucket count.
pub fn set_hash_mb(mb: usize) -> usize {
    search::set_hash_mb(mb)
}

/// Current transposition-table size in MiB.
pub fn hash_mb() -> usize {
    search::hash_mb()
}

/// Whether the given piece_type ID is a royal piece (King, Crown Prince,
/// or any other piece type whose capture ends the game). Exposed for
/// external tooling that has to tell royals apart from other pieces.
pub fn is_royal_piece_type(piece_type: u16) -> bool {
    pieces::is_royal(piece_type)
}

// ============================================================
// Board — the main game interface
// ============================================================

/// The Taikyoku Shogi board.
///
/// This is the primary type for interacting with the engine.
/// Create one with [`Board::initial()`] for the starting position,
/// or [`Board::empty()`] for an empty board.
pub struct Board {
    inner: board::Board,
}

impl Board {
    /// Create an empty board.
    pub fn empty() -> Self {
        Board {
            inner: board::Board::new(),
        }
    }

    /// Create a board with the standard initial position (804 pieces).
    ///
    /// ```rust
    /// let board = taikyokushogi::Board::initial();
    /// assert_eq!(board.piece_count(taikyokushogi::Color::Black), 402);
    /// assert_eq!(board.piece_count(taikyokushogi::Color::White), 402);
    /// ```
    pub fn initial() -> Self {
        let mut b = board::Board::new();
        b.setup_initial();
        Board { inner: b }
    }

    /// Whose turn is it?
    pub fn side_to_move(&self) -> Color {
        Color::from_raw(self.inner.side_to_move)
            .expect("invalid side_to_move value (must be 0 or 1)")
    }

    /// Current full-move number (starts at 1, increments after White moves).
    pub fn move_number(&self) -> u32 {
        self.inner.move_number
    }

    /// Number of consecutive plies without a capture or promotion.
    /// The game is automatically drawn when this reaches 1,000
    /// (500 full moves without progress).
    pub fn no_progress_plies(&self) -> u32 {
        self.inner.no_progress_plies
    }

    /// Get the piece at `(row, col)`, or `None` if empty or out of bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<Piece> {
        if row >= types::BOARD_SIZE || col >= types::BOARD_SIZE {
            return None;
        }
        let sq = types::sq_index(row, col);
        let cell = self.inner.cells[sq];
        if cell == types::EMPTY_CELL {
            None
        } else {
            Some(Piece {
                type_id: types::cell_piece(cell),
                color: Color::from_raw(types::cell_color(cell))
                    .expect("invalid cell color value (must be 0 or 1)"),
            })
        }
    }

    /// Number of pieces for the given color.
    pub fn piece_count(&self, color: Color) -> usize {
        self.inner.piece_count[color.raw() as usize]
    }

    /// Material score from Black's perspective.
    /// Positive = Black has more material.
    pub fn material_score(&self) -> i32 {
        eval::material_score(&self.inner)
    }

    /// Evaluate the position from the side to move's perspective.
    ///
    /// Uses the attached NNUE network if there is one
    /// ([`Board::set_network`]), the hand-crafted evaluator otherwise.
    pub fn evaluate(&self) -> i32 {
        eval::evaluate(&self.inner)
    }

    /// Attach an NNUE network to this board, or detach it with `None`.
    ///
    /// The network is shared (`Arc`), so one loaded [`Network`] can serve any
    /// number of boards and threads, and two boards in the same process can
    /// use different networks. `apply`/`undo` keep the accumulator in sync.
    ///
    /// ```no_run
    /// use std::sync::Arc;
    /// use taikyokushogi::{Board, Network};
    ///
    /// let net = Arc::new(Network::load("nets/tk-net1.tkn").expect("valid network"));
    /// let mut board = Board::initial();
    /// board.set_network(Some(net));
    /// let r = board.search(3, 0);
    /// ```
    pub fn set_network(&mut self, net: Option<std::sync::Arc<Network>>) {
        self.inner.set_network(net);
    }

    /// The attached NNUE network, if any.
    pub fn network(&self) -> Option<&std::sync::Arc<Network>> {
        self.inner.network()
    }

    /// Check if the game is over.
    pub fn game_result(&self) -> Option<GameResult> {
        self.inner.game_result().map(|r| match r {
            types::GameResult::BlackWins => GameResult::BlackWins,
            types::GameResult::WhiteWins => GameResult::WhiteWins,
            types::GameResult::Draw => GameResult::Draw,
        })
    }

    /// Generate all legal moves for the side to move.
    pub fn legal_moves(&self) -> Vec<Move> {
        movegen::generate_legal_moves(&self.inner)
            .into_iter()
            .map(|m| Move { inner: m })
            .collect()
    }

    /// Apply a move to the board.
    ///
    /// # Safety
    /// The caller must ensure `mv` is a legal move in the current position.
    /// Applying a move from a different position or an already-applied move
    /// can corrupt board state. Use [`Board::apply_legal`] for checked
    /// application.
    pub fn apply(&mut self, mv: &Move) {
        self.inner.apply_move(&mv.inner);
    }

    /// Apply a move only if it is legal in the current position.
    ///
    /// Returns `Ok(())` if the move was applied, or [`MoveError::Illegal`]
    /// if no matching legal move exists. This is safer than [`Board::apply`]
    /// because it validates the move against the current position before
    /// applying it.
    pub fn apply_legal(&mut self, mv: &Move) -> Result<(), MoveError> {
        let moves = movegen::generate_legal_moves(&self.inner);
        for legal in &moves {
            if legal.from_sq == mv.inner.from_sq
                && legal.to_sq == mv.inner.to_sq
                && legal.promotion == mv.inner.promotion
                && legal.is_igui == mv.inner.is_igui
            {
                self.inner.apply_move(legal);
                return Ok(());
            }
        }
        Err(MoveError::Illegal)
    }

    /// Apply a move specified by coordinates.
    ///
    /// Returns `Ok(())` if a matching legal move was found and applied,
    /// or a [`MoveError`] if the coordinates are out of bounds, no legal
    /// move matches, or the match is ambiguous (e.g. an igui and a normal
    /// move to the same square).
    ///
    /// ```rust,no_run
    /// let mut board = taikyokushogi::Board::initial();
    /// board.apply_by_coord(25, 0, 24, 0, false, None).unwrap(); // move pawn forward
    /// ```
    pub fn apply_by_coord(
        &mut self,
        from_row: usize,
        from_col: usize,
        to_row: usize,
        to_col: usize,
        promotion: bool,
        mid_sq: Option<usize>,
    ) -> Result<(), MoveError> {
        // Validate coordinates before any indexing — out-of-range coordinates
        // would otherwise panic on array access.
        if from_row >= types::BOARD_SIZE
            || from_col >= types::BOARD_SIZE
            || to_row >= types::BOARD_SIZE
            || to_col >= types::BOARD_SIZE
        {
            return Err(MoveError::OutOfBounds);
        }

        let from_sq = types::sq_index(from_row, from_col) as u16;
        let to_sq = types::sq_index(to_row, to_col) as u16;
        let moves = movegen::generate_legal_moves(&self.inner);
        let mut found: Option<&types::Move> = None;
        for m in &moves {
            let mid_match = mid_sq.map_or(true, |ms| m.mid_sq as usize == ms);
            if m.from_sq == from_sq && m.to_sq == to_sq && m.promotion == promotion && mid_match {
                if found.is_some() {
                    return Err(MoveError::Ambiguous);
                }
                found = Some(m);
            }
        }
        match found {
            Some(m) => {
                self.inner.apply_move(m);
                Ok(())
            }
            None => Err(MoveError::Illegal),
        }
    }

    /// Undo the last move. Returns `false` if there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        self.inner.undo_move()
    }

    /// Pick a uniformly random legal move, or `None` if no moves exist.
    pub fn random_move(&self) -> Option<Move> {
        let moves = movegen::generate_legal_moves(&self.inner);
        if moves.is_empty() {
            return None;
        }
        use rand::Rng;
        let idx = rand::thread_rng().gen_range(0..moves.len());
        Some(Move {
            inner: moves.into_iter().nth(idx).unwrap(),
        })
    }

    /// Run an alpha-beta search.
    ///
    /// - `depth`: search depth in plies
    /// - `time_limit_ms`: wall-clock time limit in milliseconds (0 = unlimited)
    pub fn search(&mut self, depth: u32, time_limit_ms: u64) -> SearchResult {
        let r = search::search(&mut self.inner, depth, time_limit_ms);
        SearchResult {
            best_move: r.best_move.map(|m| Move { inner: m }),
            score: r.score,
            nodes: r.nodes,
            time_ms: r.time_ms,
        }
    }

    /// [`Board::search`] with Lazy SMP: `threads` workers (1 = plain search)
    /// share the default searcher's transposition table. See
    /// [`search::search_threads`].
    pub fn search_threads(
        &mut self,
        depth: u32,
        time_limit_ms: u64,
        threads: usize,
    ) -> SearchResult {
        let r = search::search_threads(&mut self.inner, depth, time_limit_ms, threads);
        SearchResult {
            best_move: r.best_move.map(|m| Move { inner: m }),
            score: r.score,
            nodes: r.nodes,
            time_ms: r.time_ms,
        }
    }

    /// Like [`Board::search`], but with a searcher you own.
    ///
    /// Keep one [`Searcher`] around to carry the transposition table and the
    /// ordering heuristics from move to move, to size the table
    /// ([`Searcher::set_hash_mb`]) or to share it between workers
    /// ([`Searcher::share_tt`]). Call [`Searcher::clear`] for a new game.
    pub fn search_with(
        &mut self,
        searcher: &mut search::Searcher,
        depth: u32,
        time_limit_ms: u64,
    ) -> SearchResult {
        self.search_with_threads(searcher, depth, time_limit_ms, 1)
    }

    /// [`Board::search_with_threads`] with an optional per-iteration callback:
    /// `on_iter` receives one [`search::IterInfo`] after every completed
    /// deepening iteration (debug tooling and protocol front-ends use this to
    /// trace the search while it runs). Pass `None` for no callback.
    pub fn search_with_info(
        &mut self,
        searcher: &mut search::Searcher,
        depth: u32,
        time_limit_ms: u64,
        threads: usize,
        on_iter: Option<&mut dyn FnMut(&search::IterInfo)>,
    ) -> SearchResult {
        let stop = std::sync::atomic::AtomicBool::new(false);
        let r = searcher.search_threads_with_stop(
            &mut self.inner,
            depth,
            time_limit_ms,
            threads,
            &stop,
            on_iter,
        );
        SearchResult {
            best_move: r.best_move.map(|m| Move { inner: m }),
            score: r.score,
            nodes: r.nodes,
            time_ms: r.time_ms,
        }
    }

    /// [`Board::search_with`] with Lazy SMP: `threads` workers (1 = plain
    /// single-threaded search) share `searcher`'s transposition table.
    ///
    /// With more than one thread the result is no longer bit-reproducible —
    /// workers fill the table in a timing-dependent order — so use
    /// `threads = 1` for tests and for A/B comparisons.
    pub fn search_with_threads(
        &mut self,
        searcher: &mut search::Searcher,
        depth: u32,
        time_limit_ms: u64,
        threads: usize,
    ) -> SearchResult {
        let r = searcher.search_threads(&mut self.inner, depth, time_limit_ms, threads);
        SearchResult {
            best_move: r.best_move.map(|m| Move { inner: m }),
            score: r.score,
            nodes: r.nodes,
            time_ms: r.time_ms,
        }
    }

    /// Render the board as a text string.
    pub fn display(&self) -> String {
        self.inner.display()
    }

    /// Encode the board position as a TSFEN string.
    ///
    /// Format: `rank/rank/.../rank side move_number`
    ///
    /// ```rust
    /// let board = taikyokushogi::Board::initial();
    /// let tsfen = board.to_tsfen();
    /// assert!(tsfen.ends_with(" b 1"));
    ///
    /// // Round-trip
    /// let board2 = taikyokushogi::Board::from_tsfen(&tsfen).unwrap();
    /// assert_eq!(board2.to_tsfen(), tsfen);
    /// ```
    pub fn to_tsfen(&self) -> String {
        tsfen::to_tsfen(&self.inner)
    }

    /// Parse a TSFEN string into a board position.
    ///
    /// Returns `Err` with a description if the TSFEN is malformed.
    ///
    /// ```rust
    /// let board = taikyokushogi::Board::initial();
    /// let tsfen = board.to_tsfen();
    /// let board2 = taikyokushogi::Board::from_tsfen(&tsfen).unwrap();
    /// assert_eq!(board2.piece_count(taikyokushogi::Color::Black), 402);
    /// ```
    pub fn from_tsfen(tsfen: &str) -> Result<Self, String> {
        tsfen::from_tsfen(tsfen).map(|inner| Board { inner })
    }

    /// Iterate over all pieces of a given color.
    pub fn pieces(&self, color: Color) -> Vec<(Square, Piece)> {
        let c = color.raw() as usize;
        let mut result = Vec::new();
        for i in 0..self.inner.piece_list_len[c] {
            let sq = self.inner.piece_list[c][i];
            if sq == types::INVALID_SQ {
                continue;
            }
            let cell = self.inner.cells[sq as usize];
            if cell == types::EMPTY_CELL {
                continue;
            }
            result.push((
                Square::from_index(sq as usize),
                Piece {
                    type_id: types::cell_piece(cell),
                    color,
                },
            ));
        }
        result
    }
}

impl Clone for Board {
    fn clone(&self) -> Self {
        Board {
            inner: self.inner.clone(),
        }
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Board")
            .field("side_to_move", &self.side_to_move())
            .field("move_number", &self.move_number())
            .field("black_pieces", &self.piece_count(Color::Black))
            .field("white_pieces", &self.piece_count(Color::White))
            .finish()
    }
}

// ============================================================
// PyO3 module entry point (only with "python" feature)
// ============================================================

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn taikyokushogi(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register(m)
}
pub fn eval_only(b: &Board) -> i32 {
    crate::eval::evaluate(&b.inner)
}
