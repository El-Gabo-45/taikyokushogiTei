# Using Taikyoku Shogi as a Rust crate

Add to your `Cargo.toml`:

```toml
[dependencies]
taikyokushogi = "1.4"
```

```rust
use taikyokushogi::{Board, Color};

let mut board = Board::initial();
let moves = board.legal_moves();
println!("{} legal moves", moves.len()); // 512

board.apply(&moves[0]);
println!("Score: {}", board.material_score());
board.undo();

// Search: depth + time limit in ms (0 = no limit) — single-threaded
let result = board.search(2, 5000);
if let Some(mv) = result.best_move {
    println!("Best: {}, score: {}", mv, result.score);
}

// Lazy SMP: the same search with 4 workers sharing one transposition table
// (workers are opt-in; the default is search::DEFAULT_THREADS = 1)
let smp = board.search_threads(3, 5000, 4);
if let Some(mv) = smp.best_move {
    println!("SMP best: {} after {} nodes", mv, smp.nodes);
}

// Transposition table size in MiB (returns the size actually allocated)
let mb = taikyokushogi::set_hash_mb(64);
println!("hash = {}MiB (now {}MiB)", mb, taikyokushogi::search::hash_mb());

// TSFEN notation (the "FEN" of Taikyoku Shogi)
let fen = board.to_tsfen();
let restored = Board::from_tsfen(fen).unwrap();

// Piece metadata
let info = taikyokushogi::piece_info("LN").unwrap();
println!("{}: value={} area={} igui={}",
         info.name, info.value, info.area_steps, info.has_igui);

// Switch between the hand-crafted and the NNUE evaluator
// (requires a trained .tkn file — see docs/NNUE.md)
// board.set_network(Some(std::sync::Arc::new(taikyokushogi::Network::load("nets/tk.tkn")?)));

// Own a Searcher to keep tables warm across moves and to size the TT:
let mut searcher = taikyokushogi::search::Searcher::new();
let r = board.search_with(&mut searcher, 4, 0);
```

## Public API overview

The crate's public facade lives in `src/lib.rs` and re-exports these types:

| Type | What it is |
| ---- | ---------- |
| `Board` | The game: `initial()`, `empty()`, `legal_moves()`, `apply`/`apply_legal`/`apply_by_coord`, `undo`, `evaluate`, `game_result`, `to_tsfen`/`from_tsfen`, `set_network` |
| `Move` | A move: `from()`, `to()`, `is_promotion()`, `is_igui()`, `captured()`, `mid_sq()`, `Display` (`f19f20+`) |
| `Square` | `(row, col)` on the 36×36 board, `index()` |
| `Piece` | `abbrev()`, `name()`, `value()`, `is_royal()`, `promotes_to()`, `Display` (`^K`, `vGG`) |
| `Color` | `Black` / `White`, `opponent()`, `from_raw()` |
| `GameResult` | `BlackWins` / `WhiteWins` / `Draw` |
| `PieceInfo` | Metadata per piece type (abbrev, name, value, promotes_to, slides, jumps, hooks, range capture, igui, royal, Betza) |
| `SearchResult` | `best_move`, `score`, `nodes`, `time_ms` |
| `Network` | NNUE network: `load(path)`, `forward(&board)`, `explain(&board)` |
| `NnueError` | Errors from `Network::load` |
| `eval_only(&board)` | Static evaluation, bypassing the `Board` method |

Constants: `BOARD_SIZE` (36), `NUM_SQUARES` (1296), `VERSION` (crate version, e.g. `"1.4.6"`), `MATE_SCORE`.

Functions: `piece_info(abbrev)`, `piece_info_by_id(id)`, `all_pieces()`, `num_piece_types()`, `is_royal_piece_type(id)`, `set_hash_mb(mb)`, `hash_mb()`.

## Search API (`taikyokushogi::search`)

| Item | Meaning |
| ---- | ------- |
| `search(board, depth, ms)` | Single-threaded alpha-beta with the default per-thread `Searcher` |
| `search_threads(board, depth, ms, threads)` | Same with Lazy SMP workers (1 = plain search, clamped to 1..=64) |
| `Searcher` | Caller-owned engine: TT, heuristics, scratch pool, root cache; `new()`, `with_config(cfg)`, `share_tt()`, `clear()`, `set_hash_mb(mb)`, `hash_mb()`, `config()`, `search()`, `search_with_stop()`, `search_threads()` |
| `IterInfo` | Per-iteration snapshot (`depth`, `score`, `nodes`, `time_ms`, `best_move`) for `search_with_stop`'s callback |
| `SearchConfig` | Technique on/off switches: `production()` (all on), `minimal()` (none), `classic()` (TT+ordering+reductions, no pruning) |
| `DEFAULT_THREADS` / `MAX_THREADS` | 1 / 64 |

## Module map

The crate's internal code is split into module trees by concern. The public
API in `lib.rs` exposes `Board`, `Move`, `Square`, `Piece`, `Color`,
`GameResult`, `PieceInfo`, `SearchResult` and `Network`; the
protocol/service modules `search`, `elo`, `selfplay`, `usi`, `server`,
`debugging` and `research` are also `pub` (the binaries and external tooling
drive them directly). Everything else is crate-internal.

| Directory | Responsibility |
| ----------- | --------------- |
| `src/lib.rs` | Public API facade: thin wrappers over internal types |
| `src/board/` | Board state, `apply`/`undo`, incremental scores, `game_result()` |
| `src/movegen/` | Move generation: sliding, jumping, range capture, igui, promotion |
| `src/types/` | Constants, cell encoding, square math, `Move`/`UndoInfo`, tables |
| `src/eval/` | Hand-crafted evaluator + NNUE evaluator (`nnue/` sub-tree) |
| `src/search/` | PVS, Lazy SMP, transposition table, heuristics, ordering, `Searcher`, `SearchConfig` |
| `src/attack.rs` | Bitboard attack generation |
| `src/bitboard.rs` | 36×36 wide bitboards (21 u64 words) |
| `src/pieces.rs` | 301 piece definitions, Betza parser, promotions, setup |
| `src/tsfen.rs` | TSFEN position encoding/decoding |
| `src/elo.rs` | Engine-vs-engine matches, Elo + 95% CI, SPRT (`EngineConfig`, `MatchConfig`, `Sprt`) |
| `src/selfplay.rs` | Self-play game generation for NNUE training (`SelfPlayCoordinator`, `SelfPlayConfig`) |
| `src/usi.rs` | USI (Universal Shogi Interface) front-end for shogi GUIs / harnesses (bin `taikyokushogi-usi`) |
| `src/debugging/` | Search/eval introspection utilities (`eval_debug/`, `search_debug/`) |
| `src/research/` | Research toolkit (audit-2 kit): `ablate`, `critical`, `multipv`, `pieces`, `stability`, `tree` |
| `src/nnue_main.rs` | NNUE tool: classes, verify, random-net, dump-positions, explain, bench (bin `nnue`) |
| `src/python.rs` | PyO3 Python bindings (feature-gated) |
| `src/server/` | axum router, REST handlers, JSON types (`src/main.rs` is the thin launcher) |
| `src/cli.rs` | Interactive CLI REPL + one-shot commands |
| `src/debug_cli.rs` | Minimal one-shot search harness |

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full module dependency
diagram, crate boundary rules, design rationale, and stability policy, and
[MODULE_GUIDE.md](MODULE_GUIDE.md) for the developer's guide to each module.