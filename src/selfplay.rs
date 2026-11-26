#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_lossless
)]
// The integer/float casts above are intentionally allowed: every cast in this
// module operates on provably-bounded values (square indices < 1296, direction
// indices 0..7, small enums, eval scores), so truncation/sign/wrap/precision
// warnings would only produce noise.

//! Parallel self-play for Taikyoku Shogi.
use crate::board::Board;
use crate::search::search;
use crate::types::{BLACK, EMPTY_CELL, GameResult, cell_color, cell_piece};

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crossbeam::channel::{Receiver, Sender, bounded};

#[derive(Debug, Clone)]
pub struct SelfPlayConfig {
    pub num_games: usize,
    pub num_workers: usize,
    pub depth: u32,
    pub time_limit_ms: u64,
    pub max_moves: u32,
    pub output_dir: String,
    pub save_interval: usize,
    pub sqlite_path: Option<String>,
    /// Random legal moves played at the start of every game (seeded by
    /// `seed` and the game id) so that the otherwise deterministic engine
    /// produces different games. Opening positions are not recorded.
    pub opening_plies: u32,
    /// Seed of the opening randomisation.
    pub seed: u64,
    /// NNUE network used by the searches (`None` = hand-crafted evaluator).
    pub net: Option<Arc<crate::eval::nnue::Network>>,
}

impl Default for SelfPlayConfig {
    fn default() -> Self {
        Self {
            num_games: 4,
            num_workers: 2,
            depth: 2,
            time_limit_ms: 500,
            max_moves: 0,
            output_dir: "training_data".to_string(),
            save_interval: 100,
            sqlite_path: Some("training_data/games.db".to_string()),
            opening_plies: 8,
            seed: 1,
            net: None,
        }
    }
}

fn should_stop_game(move_count: u32, max_moves: u32, terminal: Option<GameResult>) -> bool {
    if terminal.is_some() {
        return true;
    }
    max_moves > 0 && move_count >= max_moves
}

/// One training position: the board, the side to move, the search score and
/// the final outcome of the game.
#[derive(Clone, Debug)]
pub struct TrainingSample {
    /// Identifier of the game this sample came from. Unique across selfplay
    /// runs writing into the same directory (the coordinator reserves id
    /// ranges from `selfplay_next_game_id.txt` and/or the SQLite max id), so
    /// the trainer can split train/validation by game safely.
    pub game_id: u32,
    /// `(piece_type & 0x1FF) | (color << 9)` per square; 0 = empty.
    pub board: [u16; 1296],
    pub side_to_move: u8,
    /// Game outcome from the side to move's point of view: -1, 0 or +1.
    pub result: i8,
    /// Score of the search that chose the move, side to move's point of view,
    /// in the engine's internal units (pawn = 500).
    pub score: i32,
    /// Hand-crafted evaluation of the same position (side to move, internal
    /// units). Used by `training/check_label_units.py` to catch label-scale
    /// mistakes before training.
    pub classic_eval: i32,
    /// Ply number within the game (0 = initial position).
    pub ply: u16,
}

/// File header of the training-sample format "TKD1": magic, u16 version, u16 reserved.
pub const SAMPLE_FILE_HEADER: [u8; 8] = *b"TKD1\x01\x00\x00\x00";

/// Per-directory state file that reserves the next free game id. Without it,
/// every selfplay run would restart ids at 0: the sample writer avoids file
/// collisions, but SQLite (`games.id` PRIMARY KEY) would reject the duplicates
/// and the trainer would group samples from *different runs* as one game.
const GAME_ID_STATE_FILE: &str = "selfplay_next_game_id.txt";

fn next_game_id_state_path(output_dir: &str) -> String {
    format!("{output_dir}/{GAME_ID_STATE_FILE}")
}

/// Next free game id recorded in `output_dir` (0 when absent/corrupt).
fn read_next_game_id(output_dir: &str) -> u64 {
    std::fs::read_to_string(next_game_id_state_path(output_dir))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

/// Reserve `[0, next)` as used. Called *before* the run starts, so even a
/// crashed run never hands out its range again.
fn write_next_game_id(output_dir: &str, next: u64) -> std::io::Result<()> {
    std::fs::create_dir_all(output_dir)?;
    std::fs::write(next_game_id_state_path(output_dir), format!("{next}\n"))
}

/// Highest game id persisted in the SQLite log, plus one (0 when the DB is
/// missing, empty or unreadable). Read-only open: never creates the file.
fn max_db_game_id(db_path: &str) -> u64 {
    use rusqlite::OpenFlags;
    rusqlite::Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .and_then(|conn| {
            conn.query_row("SELECT COALESCE(MAX(id), -1) FROM games", [], |r| {
                r.get::<_, i64>(0)
            })
        })
        .map_or(0, |v| if v < 0 { 0 } else { v as u64 + 1 })
}

impl TrainingSample {
    /// Append the sparse little-endian record to `out`:
    /// `u8 stm | i8 result | i32 score | i32 classic_eval | u16 ply | u32 game_id | u16 n | n * (u16 sq, u16 piece|color<<9)`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        let n = self.board.iter().filter(|&&c| c != 0).count() as u16;
        out.push(self.side_to_move);
        out.push(self.result as u8);
        out.extend_from_slice(&self.score.to_le_bytes());
        out.extend_from_slice(&self.classic_eval.to_le_bytes());
        out.extend_from_slice(&self.ply.to_le_bytes());
        out.extend_from_slice(&self.game_id.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        for (sq, &c) in self.board.iter().enumerate() {
            if c != 0 {
                out.extend_from_slice(&(sq as u16).to_le_bytes());
                out.extend_from_slice(&c.to_le_bytes());
            }
        }
    }
}

/// Tiny deterministic generator for the opening randomisation.
struct SplitMix(u64);
impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

#[derive(Clone, Debug)]
pub struct GameMove {
    pub move_number: u16,
    pub from: u16,
    pub to: u16,
    pub promo: u8,
    pub captured_piece: u16,
    pub side: u8,
    pub black_pieces: usize,
    pub white_pieces: usize,
}

#[derive(Clone, Debug)]
pub struct GameRecord {
    pub game_id: u64,
    pub result: i8,
    pub moves: Vec<GameMove>,
    pub total_moves: u32,
    pub depth: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Default)]
pub struct SelfPlayStats {
    pub games_completed: AtomicUsize,
    pub total_moves: AtomicU64,
    pub total_nodes: AtomicU64,
    pub total_time_ms: AtomicU64,
}

impl SelfPlayStats {
    pub fn print(&self) {
        let games = self.games_completed.load(Ordering::Relaxed);
        let moves = self.total_moves.load(Ordering::Relaxed);
        let nodes = self.total_nodes.load(Ordering::Relaxed);
        let time = self.total_time_ms.load(Ordering::Relaxed);
        println!("=== Self-Play Stats ===");
        println!("Games: {games}");
        if games > 0 {
            println!(
                "Moves: {} ({:.1} moves/game)",
                moves,
                moves as f64 / games as f64
            );
        }
        if time > 0 {
            println!(
                "Nodes: {} ({:.0} nps)",
                nodes,
                nodes as f64 / (time as f64 / 1000.0)
            );
        }
        println!("Time: {:.1}s", time as f64 / 1000.0);
    }
}

/// Hand-crafted evaluation of `board` even when a network is attached.
fn classic_eval(board: &mut Board) -> i32 {
    let saved = board.nnue.take();
    let e = crate::eval::evaluate(board);
    board.nnue = saved;
    e
}

struct SelfPlayWorker {
    id: usize,
    config: SelfPlayConfig,
    /// First game id this run may use (reserved by the coordinator so ids
    /// never collide with previous runs in the same output directory).
    game_id_offset: u64,
    stats: Arc<SelfPlayStats>,
    sample_tx: Sender<TrainingSample>,
    game_tx: Sender<GameRecord>,
}

impl SelfPlayWorker {
    fn play_one_game(&mut self, game_id: u64) -> (Vec<TrainingSample>, GameRecord) {
        let mut board = Board::new();
        board.setup_initial();
        board.set_network(self.config.net.clone());
        let mut rng = SplitMix(self.config.seed ^ game_id.wrapping_mul(0xD1B5_4A32_D192_ED03));
        let start = std::time::Instant::now();

        let mut move_count = 0u32;
        let mut game_samples: Vec<TrainingSample> = Vec::new();
        let mut game_moves: Vec<GameMove> = Vec::new();
        let result_val: i8;

        loop {
            let terminal = board.game_result();
            if should_stop_game(move_count, self.config.max_moves, terminal) {
                result_val = match terminal {
                    Some(GameResult::BlackWins) => 1i8,
                    Some(GameResult::WhiteWins) => -1i8,
                    Some(GameResult::Draw) => 0i8,
                    None => 0i8,
                };
                break;
            }

            // Random opening: not searched, not recorded as training data.
            let in_opening = move_count < self.config.opening_plies;
            let (best_move, score, nodes, time_ms) = if in_opening {
                let legal = crate::movegen::generate_legal_moves(&board);
                if legal.is_empty() {
                    result_val = if board.side_to_move == BLACK {
                        -1i8
                    } else {
                        1i8
                    };
                    break;
                }
                (legal[(rng.next() % legal.len() as u64) as usize], 0, 0, 0)
            } else {
                let r = search(&mut board, self.config.depth, self.config.time_limit_ms);
                if let Some(m) = r.best_move {
                    (m, r.score, r.nodes, r.time_ms)
                } else {
                    result_val = if board.side_to_move == BLACK {
                        -1i8
                    } else {
                        1i8
                    };
                    break;
                }
            };

            let side = board.side_to_move;
            let black_pieces = board.piece_list_len[0];
            let white_pieces = board.piece_list_len[1];

            game_moves.push(GameMove {
                move_number: (move_count + 1) as u16,
                from: best_move.from_sq,
                to: best_move.to_sq,
                promo: u8::from(best_move.promotion),
                captured_piece: best_move.captured_piece,
                side,
                black_pieces,
                white_pieces,
            });

            if !in_opening {
                let mut sample = TrainingSample {
                    game_id: game_id as u32,
                    board: [0; 1296],
                    side_to_move: side,
                    result: 0,
                    score,
                    classic_eval: classic_eval(&mut board),
                    ply: move_count as u16,
                };
                for sq in 0..1296 {
                    let cell = board.cells[sq];
                    if cell != EMPTY_CELL {
                        // Color goes in bit 9: piece types go up to 301 (9 bits).
                        sample.board[sq] =
                            (cell_piece(cell) & 0x1FF) | (u16::from(cell_color(cell)) << 9);
                    }
                }
                game_samples.push(sample);
            }

            board.apply_move(&best_move);
            move_count += 1;
            self.stats.total_moves.fetch_add(1, Ordering::Relaxed);
            self.stats.total_nodes.fetch_add(nodes, Ordering::Relaxed);
            self.stats
                .total_time_ms
                .fetch_add(time_ms, Ordering::Relaxed);
        }

        for s in &mut game_samples {
            let from_perspective = if s.side_to_move == BLACK {
                f32::from(result_val)
            } else {
                -f32::from(result_val)
            };
            s.result = from_perspective as i8;
        }

        let record = GameRecord {
            game_id,
            result: result_val,
            moves: game_moves,
            total_moves: move_count,
            depth: self.config.depth,
            duration_ms: start.elapsed().as_millis() as u64,
        };

        (game_samples, record)
    }

    fn run(&mut self) -> Result<(), String> {
        let total_games = self.config.num_games;
        let num_workers = self.config.num_workers.max(1);
        let games_per_worker = total_games.div_ceil(num_workers);
        let my_start = self.id * games_per_worker;
        let my_end = std::cmp::min(my_start + games_per_worker, total_games);
        if my_start >= total_games {
            eprintln!("[worker-{}] no games assigned", self.id);
            return Ok(());
        }
        let my_count = my_end - my_start;
        eprintln!(
            "[worker-{}] will play {} games (ids {}..{})",
            self.id,
            my_count,
            self.game_id_offset + my_start as u64,
            self.game_id_offset + my_end as u64
        );

        for (i, game_id) in (my_start..my_end).enumerate() {
            let global_id = self.game_id_offset + game_id as u64;
            let (samples, record) = self.play_one_game(global_id);
            let n = samples.len();
            let result_str = match record.result {
                1 => "BlackWins",
                -1 => "WhiteWins",
                _ => "Draw",
            };
            // A failed send means the receiving thread is gone: keep going
            // and the run would silently lose every following sample.
            for s in samples {
                if self.sample_tx.send(s).is_err() {
                    return Err(format!(
                        "sample writer stopped (channel closed) — samples of game {global_id} and later are LOST"
                    ));
                }
            }
            if self.game_tx.send(record).is_err() {
                return Err(format!(
                    "database writer stopped (channel closed) — record of game {global_id} LOST"
                ));
            }
            self.stats.games_completed.fetch_add(1, Ordering::Relaxed);
            eprintln!(
                "[worker-{}] game {}/{} (id={}) finished: {} moves, result={}",
                self.id,
                i + 1,
                my_count,
                global_id,
                n,
                result_str
            );
        }
        Ok(())
    }
}

pub struct SelfPlayCoordinator {
    config: SelfPlayConfig,
    game_id_offset: u64,
    stats: Arc<SelfPlayStats>,
    workers: Vec<JoinHandle<Result<(), String>>>,
    writer_handle: Option<JoinHandle<Result<(), String>>>,
    db_handle: Option<JoinHandle<Result<(), String>>>,
    sample_tx: Sender<TrainingSample>,
    game_tx: Sender<GameRecord>,
}

impl SelfPlayCoordinator {
    #[must_use]
    pub fn new(config: SelfPlayConfig) -> Self {
        // ── Reserve a game-id range unique across runs ──────────────
        // Sources of truth: the per-directory state file and (when enabled)
        // the SQLite max id. The range is written back *before* the run
        // starts, so even a crashed run cannot reuse its ids later.
        let mut offset = read_next_game_id(&config.output_dir);
        if let Some(db) = &config.sqlite_path {
            offset = offset.max(max_db_game_id(db));
        }
        let reserved_end = offset + config.num_games as u64;
        assert!(
            u32::try_from(reserved_end).is_ok(),
            "game id space exhausted: the TKD1 sample format stores u32 game ids"
        );
        if let Err(e) = write_next_game_id(&config.output_dir, reserved_end) {
            eprintln!(
                "[selfplay] warning: could not reserve game ids in {}: {e} \
                 (continuing; the SQLite max-id fallback still protects the database)",
                config.output_dir
            );
        }
        println!(
            "[selfplay] game ids: {offset}..{reserved_end} (unique across runs in {})",
            config.output_dir
        );

        let stats = Arc::new(SelfPlayStats::default());
        let (sample_tx, sample_rx) = bounded(10000);
        let (game_tx, game_rx) = bounded(1000);

        let output_dir = config.output_dir.clone();
        let save_interval = config.save_interval;
        let writer_handle =
            std::thread::spawn(move || Self::writer_loop(sample_rx, output_dir, save_interval));

        let db_handle = if let Some(db_path) = &config.sqlite_path {
            let db_path = db_path.clone();
            let output_dir = config.output_dir.clone();
            Some(std::thread::spawn(move || {
                Self::db_loop(game_rx, db_path, output_dir)
            }))
        } else {
            None
        };

        Self {
            config,
            game_id_offset: offset,
            stats,
            workers: Vec::new(),
            writer_handle: Some(writer_handle),
            db_handle,
            sample_tx,
            game_tx,
        }
    }

    fn writer_loop(
        rx: Receiver<TrainingSample>,
        output_dir: String,
        save_interval: usize,
    ) -> Result<(), String> {
        // Create output dir, propagating errors instead of silencing them.
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("cannot create output dir {output_dir}: {e}"))?;

        // Discover the next free file index so we never overwrite samples
        // from a previous selfplay run in the same output_dir.
        let mut file_count = Self::next_free_index(&output_dir);
        let mut buffer: Vec<TrainingSample> = Vec::new();
        let mut last_print = std::time::Instant::now();
        let mut total_saved = 0u64;
        while let Ok(sample) = rx.recv() {
            buffer.push(sample);
            if buffer.len() >= save_interval {
                let filename = format!("{output_dir}/samples_{file_count:06}.bin");
                let mut data = SAMPLE_FILE_HEADER.to_vec();
                for s in &buffer {
                    s.encode_into(&mut data);
                }
                std::fs::write(&filename, &data).map_err(|e| {
                    format!(
                        "cannot write {filename}: {e} ({} samples LOST)",
                        buffer.len()
                    )
                })?;
                total_saved += buffer.len() as u64;
                println!(
                    "[writer] Saved {} samples (total {}) to {}",
                    buffer.len(),
                    total_saved,
                    filename
                );
                buffer.clear();
                file_count += 1;
            }
            if last_print.elapsed().as_secs() >= 2 {
                last_print = std::time::Instant::now();
                println!("[writer] buffer={} samples pending", buffer.len());
            }
        }
        if !buffer.is_empty() {
            let filename = format!("{output_dir}/samples_{file_count:06}.bin");
            let mut data = SAMPLE_FILE_HEADER.to_vec();
            for s in &buffer {
                s.encode_into(&mut data);
            }
            std::fs::write(&filename, &data).map_err(|e| {
                format!(
                    "cannot write {filename}: {e} ({} samples LOST)",
                    buffer.len()
                )
            })?;
            total_saved += buffer.len() as u64;
            println!(
                "[writer] Flushed {} samples (total {}) to {}",
                buffer.len(),
                total_saved,
                filename
            );
        }
        Ok(())
    }

    /// Returns the smallest `n >= 0` such that `output_dir/samples_{n:06}.bin`
    /// does not yet exist. Used to avoid clobbering samples from earlier runs.
    fn next_free_index(output_dir: &str) -> usize {
        let mut n = 0usize;
        while std::path::Path::new(&format!("{output_dir}/samples_{n:06}.bin")).exists() {
            n += 1;
        }
        n
    }

    fn db_loop(
        rx: Receiver<GameRecord>,
        db_path: String,
        output_dir: String,
    ) -> Result<(), String> {
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("cannot create dir {output_dir}: {e}"))?;
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("cannot open {db_path}: {e}"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS games (
                id INTEGER PRIMARY KEY,
                result INTEGER NOT NULL,
                total_moves INTEGER NOT NULL,
                depth INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at REAL NOT NULL DEFAULT (julianday('now'))
            );
            CREATE TABLE IF NOT EXISTS moves (
                game_id INTEGER NOT NULL,
                move_number INTEGER NOT NULL,
                from_sq INTEGER NOT NULL,
                to_sq INTEGER NOT NULL,
                promo INTEGER NOT NULL,
                captured INTEGER NOT NULL,
                side INTEGER NOT NULL,
                black_pieces INTEGER NOT NULL,
                white_pieces INTEGER NOT NULL,
                PRIMARY KEY (game_id, move_number)
            );",
        )
        .map_err(|e| format!("cannot create schema in {db_path}: {e}"))?;
        let mut total = 0u64;
        let mut failures = 0u64;
        let mut last_err = String::new();
        let mut last_print = std::time::Instant::now();
        while let Ok(record) = rx.recv() {
            total += 1;
            if let Err(e) = conn.execute(
                "INSERT INTO games (id, result, total_moves, depth, duration_ms) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![record.game_id as i64, i64::from(record.result), i64::from(record.total_moves), i64::from(record.depth), record.duration_ms as i64]
            ) {
                failures += 1;
                last_err = format!("insert game {}: {e}", record.game_id);
                continue;
            }
            match conn.unchecked_transaction() {
                Ok(t) => {
                    let mut failed = None;
                    for m in &record.moves {
                        if let Err(e) = t.execute(
                            "INSERT INTO moves (game_id, move_number, from_sq, to_sq, promo, captured, side, black_pieces, white_pieces) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                            rusqlite::params![
                                record.game_id as i64, i64::from(m.move_number), i64::from(m.from), i64::from(m.to),
                                i64::from(m.promo), i64::from(m.captured_piece), i64::from(m.side),
                                m.black_pieces as i64, m.white_pieces as i64,
                            ]
                        ) {
                            failed = Some(format!(
                                "insert move {} of game {}: {e}",
                                m.move_number, record.game_id
                            ));
                            break;
                        }
                    }
                    match failed {
                        Some(msg) => {
                            failures += 1;
                            last_err = msg;
                        }
                        None => {
                            if let Err(e) = t.commit() {
                                failures += 1;
                                last_err = format!("commit game {}: {e}", record.game_id);
                            }
                        }
                    }
                }
                Err(e) => {
                    failures += 1;
                    last_err = format!("transaction for game {}: {e}", record.game_id);
                }
            }
            if last_print.elapsed().as_secs() >= 2 {
                last_print = std::time::Instant::now();
                println!("[db] games={total} ({failures} failed) written to {db_path}");
            }
        }
        println!("[db] Done: {total} games written to {db_path} ({failures} failed)");
        if failures > 0 {
            Err(format!(
                "{failures} of {total} games failed to persist in {db_path} \
                 (last error: {last_err}) — the dataset is INCOMPLETE"
            ))
        } else {
            Ok(())
        }
    }

    pub fn start(&mut self) {
        let num_workers = self.config.num_workers.max(1);
        for i in 0..num_workers {
            let tx = self.sample_tx.clone();
            let gx = self.game_tx.clone();
            let config = self.config.clone();
            let stats = self.stats.clone();
            let offset = self.game_id_offset;
            let handle = std::thread::spawn(move || {
                let mut worker = SelfPlayWorker {
                    id: i,
                    config,
                    game_id_offset: offset,
                    stats,
                    sample_tx: tx,
                    game_tx: gx,
                };
                worker.run()
            });
            self.workers.push(handle);
        }
    }

    /// Joins every thread and reports any data loss. Returns `Err` when a
    /// worker, the sample writer or the database writer failed — the caller
    /// must exit non-zero so a broken dataset is never mistaken for success.
    pub fn wait(mut self) -> Result<(), String> {
        let mut errors: Vec<String> = Vec::new();
        for handle in self.workers.drain(..) {
            match handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(format!("worker: {e}")),
                Err(p) => errors.push(format!("worker panicked: {p:?}")),
            }
        }
        drop(self.sample_tx);
        drop(self.game_tx);
        if let Some(writer) = self.writer_handle.take() {
            match writer.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(format!("writer: {e}")),
                Err(p) => errors.push(format!("writer panicked: {p:?}")),
            }
        }
        if let Some(db) = self.db_handle.take() {
            match db.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => errors.push(format!("db: {e}")),
                Err(p) => errors.push(format!("db panicked: {p:?}")),
            }
        }
        self.stats.print();
        if errors.is_empty() {
            println!("\nSelf-play completed!");
            Ok(())
        } else {
            eprintln!("\n✗ SELF-PLAY FAILED — possible data loss:");
            for e in &errors {
                eprintln!("  - {e}");
            }
            Err(errors.join("; "))
        }
    }
}

pub fn run_selfplay(config: Option<SelfPlayConfig>) {
    let config = config.unwrap_or_default();
    let max_moves = if config.max_moves == 0 {
        "unlimited".to_string()
    } else {
        config.max_moves.to_string()
    };
    println!(
        "Starting self-play: {} games, {} workers, depth={}, max_moves={}",
        config.num_games, config.num_workers, config.depth, max_moves
    );
    let mut coord = SelfPlayCoordinator::new(config);
    coord.start();
    if coord.wait().is_err() {
        // Data loss was detected (or a writer died): the dataset on disk is
        // incomplete, so the process must not report success.
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_full_games_and_fast_search() {
        let cfg = SelfPlayConfig::default();
        assert_eq!(cfg.max_moves, 0);
        assert!(cfg.depth >= 2);
        assert!(cfg.time_limit_ms > 0);
    }

    #[test]
    fn stop_condition_respects_unbounded_mode() {
        assert!(!should_stop_game(1000, 0, None));
        assert!(should_stop_game(1000, 10, None));
        assert!(should_stop_game(10, 10, None));
        assert!(should_stop_game(5, 10, Some(GameResult::Draw)));
    }

    /// Unique scratch dir per test (tests run concurrently in one process).
    fn scratch(name: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir()
            .join(format!(
                "tk_selfplay_{}_{}_{}",
                name,
                std::process::id(),
                nanos
            ))
            .to_string_lossy()
            .into_owned()
    }

    fn tiny_config(output_dir: &str) -> SelfPlayConfig {
        SelfPlayConfig {
            num_games: 2,
            num_workers: 1,
            depth: 1,
            time_limit_ms: 10,
            max_moves: 6,
            output_dir: output_dir.to_string(),
            save_interval: 1,
            sqlite_path: Some(format!("{output_dir}/games.db")),
            opening_plies: 1,
            seed: 1,
            net: None,
        }
    }

    #[test]
    fn game_id_state_roundtrip_and_missing() {
        let dir = scratch("state");
        assert_eq!(
            read_next_game_id(&dir),
            0,
            "missing state file means ids start at 0"
        );
        write_next_game_id(&dir, 42).expect("game id file must be writable");
        assert_eq!(read_next_game_id(&dir), 42);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Audit fix A: two runs into the same directory must not collide — the
    /// second run continues the id range instead of restarting at 0, which
    /// previously made SQLite reject the inserts and made the trainer group
    /// samples of different runs as one game.
    #[test]
    fn second_run_in_same_dir_gets_fresh_game_ids() {
        let dir = scratch("ids");
        for _ in 0..2 {
            let mut coord = SelfPlayCoordinator::new(tiny_config(&dir));
            coord.start();
            coord.wait().expect("both runs must succeed");
        }
        let conn = rusqlite::Connection::open(format!("{dir}/games.db")).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM games", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 4, "both runs' games must be persisted");
        let (min, max): (i64, i64) = conn
            .query_row("SELECT MIN(id), MAX(id) FROM games", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(
            (min, max),
            (0, 3),
            "ids must be 0..3 across runs — no collision, no gap"
        );
        // And the sample writer must not have overwritten run 1's files.
        let samples: usize = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("samples_"))
            .count();
        assert!(samples >= 2, "each run must write its own sample files");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Audit fix B: a writer that cannot even create its output dir must
    /// surface as a failed run (non-zero exit via run_selfplay), not as a
    /// "completed!" message with a lost dataset.
    #[test]
    fn writer_failure_is_reported_not_silenced() {
        let dir = scratch("writerfail");
        std::fs::create_dir_all(&dir).unwrap();
        let blocker = format!("{dir}/not_a_dir");
        std::fs::write(&blocker, b"x").unwrap();
        let mut cfg = tiny_config(&blocker);
        cfg.sqlite_path = None;
        let mut coord = SelfPlayCoordinator::new(cfg);
        coord.start();
        let err = coord.wait().expect_err("writer failure must surface");
        assert!(
            err.contains("cannot create output dir"),
            "the write error must reach the coordinator: {err}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
