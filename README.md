# Taikyoku Shogi Engine

A complete engine for **Taikyoku Shogi** (大局将棋, "ultimate chess"), the largest known variant of shogi.

- **36 x 36** board (1,296 squares)
- **804 pieces** (402 per side)
- **209 initial piece types** with distinct movement patterns
- **92 additional promoted forms** (301 movement classes in total)

## Screenshots

The web GUI supports two piece-symbol modes: traditional Japanese kanji and English abbreviations.

| Interface view | English names | Kanji / Japonese |
|---------------------------|------------------|-----------------|
| ![Interfaz completa](docs/screenshots/Screenshot_2026-10-31_14-30-08.png) | ![Siglas en inglés](docs/screenshots/Screenshot_2026-10-31_14-30-37.png) | ![Kanji / Japonés](docs/screenshots/Screenshot_2026-10-31_14-38-29.png) |

## Rules Overview

### Objective

Capture all of your opponent's **royal pieces** (King and Crown Prince). The game ends immediately when a player's last royal piece is captured. Unlike standard shogi, there is no check or checkmate — you may move your King into danger.

### Turns

Black (先手) moves first. Players alternate moving exactly one piece per turn. Passing is not allowed. Captured pieces are permanently removed (no drops).

### Movement

Pieces move orthogonally or diagonally. Most pieces have unique movement patterns combining:

| Type | Description |
|------|-------------|
| **Step** | Move exactly 1 square |
| **Slide** | Move any number of empty squares in a line |
| **Limited** | Move up to N squares in a line (2-7) |
| **Jump** | Leap to a square, bypassing intervening pieces |
| **Hook** | Slide along a line, optionally turn 90 degrees once, then continue sliding |
| **Area** | Take multiple steps per turn (Lion-type moves) |
| **Range capture** | Fly over pieces along a line, capturing all of lower rank |
| **Igui** | Capture a neighboring piece without moving to its square (written `c5c5`, i.e. `from` = `to`) |

See [PIECES.md](PIECES.md) for the complete movement reference for every piece.

### Promotion

The **promotion zone** is the 11 rows on the opponent's side of the board.

- A piece **may promote** when it moves into the promotion zone from outside, or when it captures an opponent's piece while inside the zone.
- A piece that entered the zone without promoting may only promote later by capturing inside the zone, or by leaving and re-entering.
- Promotion is **optional**, except for forward-only pieces (Pawn, Stone General, Iron General, Dog, Wood General, Incense Chariot, Ox Chariot, Fierce Tiger) which **must promote** upon reaching the farthest rank.
- Promotion is permanent — a piece cannot revert to its unpromoted form.

### Range Capture Ranking

Range-capturing pieces — Great General, Vice General, Flying General,
Angle General, Fierce Dragon and Flying Crocodile (the only six pieces with
`range_capture` rays) — fly over pieces along a line and capture every
occupied square up to their landing square, **but only pieces of strictly
lower rank**. The engine models five tiers (`RANK_*` in `src/types/constants.rs`);
a piece can range-capture everything in the tiers *below* its own:

1. King, Crown Prince (`RANK_ROYAL`) — can never be range-captured
2. Great General (`RANK_GREAT`)
3. Vice General (`RANK_VICE`)
4. Flying General, Angle General, Fierce Dragon, Flying Crocodile (`RANK_RANGE_CAP`)
5. all remaining pieces (`RANK_NORMAL`)

Pieces of the same rank cannot take each other by range capture (e.g. one
Great General cannot capture another), the ray **stops** at the first piece
that cannot be captured (nothing can fly over a King), and ordinary —
non-range — captures ignore ranks entirely.

### Draw by No Progress (500-Move Rule)

The game is automatically drawn if **500 consecutive full moves** (1,000 plies) pass with neither player making a capture nor a promotion. The counter resets whenever a piece is captured or a promotion occurs. A game in which **both** sides lose all their royal pieces (e.g. mutual captures in the same exchange) is also a draw.

### Initial Setup

Each side's 402 pieces occupy 12 ranks. Black occupies the bottom of the board (rows 25-36); White mirrors from the top (rows 1-12). The King sits at the center of the back rank (column 18), flanked by the Crown Prince (column 19).

### Engine Implementation Notes (variant rules vs. code)

- **No check filtering.** `legal_moves()` never disallows leaving a royal
  piece en prise — the variant has no check rule, so you may move your King
  into danger (`generate_legal_moves` is generation-only; the search avoids
  hanging royals through their material value: King = 100,000, Crown Prince
  = 50,000).
- **No stalemate-win rule.** When a side has no legal move: the search
  scores it as a loss for the side to move, engine-vs-engine games
  (`selfplay` / `match` / `sprt`) record it as a **draw**, and the web
  server simply ends the game.
- **Draws** come from the 500-move no-progress rule or from both sides
  losing all royal pieces (see above).

## CLI

The engine ships with a full user-facing CLI (not just a debug harness):

```bash
cargo build --release --bin taikyokushogi-cli
./target/release/taikyokushogi-cli                 # interactive REPL
./target/release/taikyokushogi-cli perft 2         # one-shot commands (`&&` chains)
./target/release/taikyokushogi-cli --help          # full flag reference
```

### Command-line flags (non-interactive)

```
taikyokushogi-cli [OPTIONS] [COMMANDS...]

  -d, --depth <N>        default search depth in plies (default 3)
  -t, --movetime <MS>    time budget per move in ms (0 = none)
  -j, --threads <N>      Lazy SMP workers, 1..=64 (default 1)
      --hash <MiB>       transposition table size in MiB
  -p, --position <TSFEN> start from this position (`startpos` = initial board;
                         quote a TSFEN so it arrives as one argument)
      --go               run one search, print `bestmove ...`, exit
      --perft <N>        print perft(N) for the position, exit
  -h, --help             show this help
      --version          print engine version and exit
```

Remaining arguments are interactive commands and `&&` chains them; with no
arguments the CLI reads commands from stdin (REPL). Examples:

```bash
# Search the start position with 4 Lazy-SMP workers and exit
./target/release/taikyokushogi-cli --threads 4 --go
# -> bestmove <mv> score <s> nodes <n> time <ms>ms

# Load a position, then search deeper from the REPL
./target/release/taikyokushogi-cli --threads 4 "position <tsfen>" && go depth 6
```

### Commands (REPL or one-shot)

| Command | Description |
|---------|-------------|
| `new`, `startpos`, `position <tsfen>` | reset / load a TSFEN position |
| `board`, `status`, `fen` | show board, status line, or TSFEN |
| `legal [<square>]` | list legal moves |
| `move <mv>` | apply a move (`f19f20`, `f19-f20`, `f19f20+`, `19,17-20,17`, igui `c5c5`) |
| `undo`, `history` | take back / list the session's moves |
| `go [depth N \| movetime MS \| threads N]`, `hint` | search and print the best move (does not apply it) |
| `play <black\|white\|both>`, `stop` | engine plays a side automatically |
| `eval`, `perft <N>`, `bench` | evaluation, node counts, micro-benchmarks |
| `save <file.json>` / `load <file.json>` | save/load a game (TSFEN + move list) |
| `selfplay [G D PLIES MS] [threadsA N] [threadsB N]` | engine-vs-engine games (per-side Lazy-SMP workers) |
| `match <G> <dA> <dB> [movetime MS] [maxplies N] [threadsA N] [threadsB N]` | match between two configs + **Elo estimate** |
| `sprt <elo0> <elo1> [G] [dA] [dB] [MS] [threadsA N] [threadsB N]` | SPRT test between two configs |
| `setoption depth <N> \| movetime <MS> \| threads <N> \| hash <MiB> \| nnue <on\|off>` | engine options |
| `about`, `help`, `version`, `quit` | info, help, engine version, exit |

Notes on behavior (so the docs match the engine):

- Defaults: `depth = 3`, `movetime = 0` (no limit), `threads = 1`
  (single-threaded), hash table size is the crate default until you set
  `--hash`/`setoption hash`.
- `threads` is clamped to `1..=64` (`search::MAX_THREADS`); values below 1
  or unparseable values are rejected with an error/warning and leave the
  previous setting untouched.
- In `selfplay`/`match`/`sprt`, the keyword arguments (`movetime`,
  `maxplies`, `threadsA`, `threadsB`) are case-insensitive and come
  **after** the positional arguments; a bad value warns and keeps the
  previous/default value. `movetime` applies to **both** sides.

## ELO measurement

Strength changes are measured with the built-in `elo` module
(`taikyokushogi::elo`), which the CLI exposes through the `match` and `sprt`
commands:

* **`match 20 2 3`** — plays 20 games alternating colors between a depth-2 and
  a depth-3 configuration, then prints W/D/L and the Elo difference with a
  95% confidence interval.
* **`sprt 0 10 100 2 3`** — runs a sequential probability ratio test with
  H0 ≤ 0 Elo vs H1 ≥ 10 Elo (α = β = 0.05), stopping as soon as the result
  is statistically decided.

Both commands accept keyword overrides **after** the positional arguments
(case-insensitive; a bad value warns and keeps the previous value):
`movetime MS` (per-move budget, applied to both sides), `maxplies N` (ply
cap; games hitting it count as draws), `threadsA N` / `threadsB N` (Lazy-SMP
workers per side, 1..=64). That makes the SMP-vs-single-thread A/B test a
one-liner:

```bash
# Same depth (3) and same time budget (300 ms/move) for both sides:
# side A with 4 Lazy-SMP workers vs side B single-threaded
./target/release/taikyokushogi-cli match 20 3 3 movetime 300 threadsA 4 threadsB 1
```

Because `threads` only changes how the same search is explored, Elo from
this comparison is the Lazy-SMP gain for the given depth/time budget on your
machine — run ≥ 100 games (or `sprt`) before quoting a number.

The same machinery is available as a library:

```rust
use taikyokushogi::elo::{self, EngineConfig, MatchConfig, Sprt};

let cfg = MatchConfig {
    games: 50,
    a: EngineConfig::depth(2),
    b: EngineConfig::depth(3),
    ..Default::default()
};
let report = elo::run_match(&cfg);   // report.elo, report.margin95
let sprt = elo::run_sprt(&cfg, &Sprt::default());

// Give side A four Lazy-SMP workers while side B stays single-threaded
// (EngineConfig.threads defaults to search::DEFAULT_THREADS = 1)
let smp_cfg = MatchConfig {
    a: EngineConfig { threads: 4, ..EngineConfig::depth(3) },
    ..cfg
};
```

Elo math follows the standard logistic model (`expected_score`,
`elo_from_score`, `Wdl::elo` / `elo_interval95`), with SPRT bounds
`±ln((1-β)/α)` in the Gaussian approximation used by fishtest.


## Getting Started

### Requirements

- A recent stable **Rust toolchain** (edition 2021). **Cargo 1.78+** is
  required because `Cargo.lock` is written in version 4 format.
- **Node.js + npm** only if you want to rebuild the `web/` frontend
  (TypeScript + Vite; a prebuilt `dist/` is committed).
- **Python 3.9+** only for the NNUE training pipeline in `training/` (see
  [training/README.md](training/README.md)).

### Build

```bash
git clone https://github.com/El-Gabo-45/taikyokushogiTei.git
cd taikyokushogiTei

# Build the engine, CLI tools and examples in release mode
cargo build --release

# The HTTP server lives behind the `server` feature:
cargo build --release --features server --bin taikyokushogi-server
```

### Web GUI

The compiled frontend (`web/dist/`) is committed in the repository, so you can run the server immediately after building:

```bash
# Start the HTTP server: REST API + static web frontend
cargo run --release --features server --bin taikyokushogi-server
# Then open http://localhost:8000 in your browser
```

If you modify `web/src/` and need to rebuild the frontend:

```bash
cd web && npm install && npm run build && cd ..
```

> **Note:** `web/dist/` is tracked in git (`!web/dist/` in `.gitignore`). If you clone and the directory is missing, run `cd web && npm install && npm run build` before starting the server.

### Web GUI Features

- **Game modes**: Human vs Random, Human vs AI, Random vs Random, AI vs AI
- Play as **Black or White**; pick the AI strength (Random, D1–D3)
- Click a piece to see its legal moves highlighted on the board
  (green = move, red = capture); a modal disambiguates promotion/igui moves
- **Piece Inspector**: click any piece to see its name, value, promotion,
  Betza notation, slides/jumps, area and range-capture capabilities
- **Engine panel**: version, evaluation backend, hash size, piece database
  size, and telemetry (depth, score, nodes, time) of the last search
- **Analysis mode** (`Analyze`): run a search without moving, then inspect
  the best move and score
- **Material balance chart** with the full score history
- **Game export**: download the played game as CLI-compatible **JSON**
  (`tsfen`, `moves`, `side`, `move_number`; load it in the CLI with
  `load <file.json>`) or as a **PGN** file for reading/sharing. Taikyoku's
  36×36 board is outside the PGN standard, so the PGN uses coordinate moves
  and a `MoveFormat` tag.
- **TSFEN panel**: export, copy, download and load positions
- **Flip board**, **zoom slider**, kanji/English glyphs, last-move and hover
  highlights, responsive layout, keyboard shortcuts (N/U/A/K/F/±/Space)
- **New Game**, **Undo** (multi-ply via API), **Auto** play and **AI Move**

### REST API

The server exposes a JSON REST API — consumed by the frontend in
`web/src/api/client.ts`. The handlers live in `src/server/handlers.rs`
(the module is part of the library so `tests/server_api.rs` can exercise the
whole router in-process).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Liveness, engine version, board size, piece-type count |
| `/api/state` | GET | Full board state + TSFEN, eval score, material, version counter |
| `/api/status` | GET | Compact summary (result, ply count, piece counts, TSFEN) |
| `/api/history` | GET | Move log + score history |
| `/api/moves` | GET | Legal moves from a square (`?r=<row>&c=<col>`) |
| `/api/legal-moves` | GET | Every legal move of the side to move (with origins) |
| `/api/engine-info` | GET | Version, backend, hash size, threads, search techniques |
| `/api/evaluate` | GET | Static evaluation and material balance |
| `/api/pieces` | GET | Full piece database (301 movement classes) |
| `/api/piece-info/{abbrev}` | GET | Piece metadata incl. Betza and royal flag (404 if unknown) |
| `/api/position` | GET/POST | Read/set the position in TSFEN format |
| `/api/new-game` | POST | Reset to the initial position |
| `/api/move` | POST | Apply a human move (`from`, `to`, `promotion`, `mid_sq`) |
| `/api/ai-move` | POST | Search and apply a move; returns `search` telemetry |
| `/api/search` | POST | Analysis-only search; never mutates the board |
| `/api/undo` | POST | Undo N plies (`{"plies": 2}`, default 1) |
| `/api/save` | GET/POST | Serialize the position to TSFEN |
| `/api/load` | POST | Load a position from TSFEN |

Errors are JSON (`{"ok": false, "error": "..."}`) with real HTTP status codes:
`400` for invalid coordinates/TSFEN, `404` for unknown pieces/endpoints and
`409` when a move conflicts with the current game state.

### Command-Line Tools

| Binary | Purpose |
|---|---|
| `taikyokushogi-server` | HTTP server + web GUI (REST API in `src/server/`; build with `--features server`) |
| `taikyokushogi-cli` | Full user-facing CLI: REPL + argv flags (see the CLI section) |
| `debug-cli` | Minimal CLI search harness: best move, score, nodes, time |
| `selfplay` | Self-play training-data generator for the NNUE pipeline |

```bash
# Search the initial position at depth 5 with a 10 s time limit
cargo run --release --bin debug-cli 5 10000

# Same search through the full CLI, with 4 Lazy-SMP workers
cargo run --release --bin taikyokushogi-cli -- --depth 5 --movetime 10000 --threads 4 --go

# Generate 100 self-play games (depth 3, no ply cap, 4 workers, 200 ms per move)
cargo run --release --bin selfplay 100 3 0 4 200
# -> writes training_data/samples_*.bin and training_data/games.db
```

`debug-cli` takes positional args `<depth> <time_ms> [--debug|-d]`;
`selfplay` takes `<num_games> <depth> <max_moves> <num_workers> <time_limit_ms>`
(defaults: 10 / 2 / 0 = unlimited / 4 / 500).

### Examples and Benchmarks

All examples run with `cargo run --release --example <name>`:

| Example | Purpose |
|---|---|
| `bench_fixed` | Deterministic fixed-depth search benchmark (depths as args) |
| `bench_nps` | NPS + component timing (movegen, eval, apply/undo) + bottleneck analysis |
| `check_movecounts` | Legal move count from the initial position |
| `stress_undo` | Apply/undo stress test (checks the board state round-trips) |
| `bench`, `bench_deep`, `bench_depth1`, `bench_midgame`, `bench_thorough`, `bench_time` | Additional benchmarks |
| `export_piece_metadata` | Writes `training/piece_metadata.json` for the NNUE pipeline |
| `toggle_nnue` | Attach/detach an NNUE network on a board (smoke test) |
| `play` | Minimal command-line game loop |
| `check_db` | Self-play SQLite database sanity checks |
| `match_race` | Head-to-head Elo/SPRT matches between two configs |
| `perft`, `perft_breakdown` | Perft counts / perft(1) breakdown by piece type |
| `oracle_walk` | Random-walk invariant oracle (apply/undo round-trip checks) |
| `nnue_speed` | NNUE incremental `evaluate()` vs full refresh speed |
| `depth_quick`, `depth_time` | Midgame depth sweep with a 2 s budget / untimed depths 1–5 from the start position |

Examples that take arguments: `bench_fixed <depths...>`, `perft [depth]`,
`match_race [games] [depthA] [depthB] [time_ms] [opening_plies]
[netA=PATH|netB=PATH|sprt=elo0:elo1]`; the rest run as-is.

```bash
cargo run --release --example bench_fixed 4 5 6   # fixed-depth search at depths 4, 5, 6
cargo run --release --example check_movecounts    # -> 512 legal moves
cargo run --release --example bench_nps           # full benchmark report
```

## Project Structure

```
taikyokushogiTei/
  src/
    lib.rs              # Public crate API (Board, Move, PieceInfo, search, TSFEN, elo, ...)
    main.rs             # Thin launcher for the HTTP server        (bin: taikyokushogi-server)
    server/
      mod.rs            #   AppState/GameState, router, static serving
      handlers.rs       #   REST handlers + status codes
      types.rs          #   JSON request/response types
    cli.rs              # Full user-facing CLI: REPL + argv flags   (bin: taikyokushogi-cli)
    debug_cli.rs        # Minimal CLI search harness                (bin: debug-cli)
    selfplay_main.rs    # Self-play entry point                     (bin: selfplay)
    selfplay.rs         # Self-play logic + TKD1 training-sample format
    elo.rs              # Match / SPRT / Elo estimation (EngineConfig, threads A/B)
    types.rs            # Core types, constants, ray tables
    pieces.rs           # 301 piece types, Betza notation parser, initial setup ranks
    board.rs            # Board representation, apply/undo, game rules
    movegen.rs          # Legal move generation (no check filtering — variant has no check)
    attack.rs           # Attack / ray bitboards
    bitboard.rs         # Bitboard helpers
    search/             # PVS + TT + move ordering + pruning + quiescence + Lazy SMP
      mod.rs            #   Entry point: iterative deepening + aspiration windows + search_threads
      params.rs         #   Every tunable constant in one place (incl. DEFAULT_THREADS/MAX_THREADS)
      pvs.rs            #   PVS core (pruning stages, staged movegen, beams)
      root.rs           #   Root iteration (the old depth-1..3 material fast path was removed)
      smp.rs            #   Lazy SMP: shared best result across worker threads
      buffers.rs        #   Pooled per-node scratch buffers (no allocs on the hot path)
      qsearch.rs        #   Capture-only quiescence
      tt.rs             #   Lock-free bucketed transposition table
      heuristics.rs     #   Killers / butterfly history / counter moves
      ordering.rs       #   Move scoring
    correctness_tests.rs #   Correctness suite (cargo test; internal because release uses panic=abort)
    tsfen.rs            # TSFEN position notation (encode / parse)
    eval/
      mod.rs            # Evaluator dispatcher (hand-crafted <-> NNUE)
      families.rs       # Piece-family material values
      psqt.rs           # Piece-square tables
      zones.rs          # Threat zones and king-safety heuristics
      nnue/             # NNUE "S" evaluator, TKN1 loader, tools (see docs/NNUE.md)
    debugging/          # Search/eval introspection utilities
  examples/             # Benchmarks, smoke tests and tooling (see examples/README.md)
  docs/                 # ARCHITECTURE.md
  web/                  # TypeScript + Vite frontend (served by the Rust server)
    src/
      main.ts           #   Entry point
      game/state.ts     #   Game state machine + modes
      api/client.ts     #   REST client
      ui/               #   Canvas renderer, input controller, sidebar panels
      data/kanji.ts     #   Piece kanji glyphs
  training/             # NNUE PyTorch pipeline — see training/README.md
  training_data/        # Generated self-play .bin samples + games.db
  PIECES.md             # Complete piece movement reference
  Cargo.toml            # Rust project config (bins, features, examples)
```

There is **no `tests/` directory**: the integration-style suite lives inside
the crate as `src/correctness_tests.rs` (plus unit tests in the modules and
9 doc-tests), all run by plain `cargo test`. External `tests/*.rs` targets
are deliberately not used because the release profile sets `panic = "abort"`.

## Quick Start — Build & Run

Build (release):

```bash
git clone https://github.com/El-Gabo-45/taikyokushogiTei.git
cd taikyokushogiTei
cargo build --release
```

NNUE needs **no feature flag**: the evaluator is always compiled and a
network is attached to a board at runtime (`board.set_network(Some(net))`). Optional
features are described under [Cargo Features](#cargo-features) below — note
that the GPU features currently have no consuming code.

Build Python bindings (optional):

```bash
pip install maturin
# src/python.rs is gated on the `python` cfg (set by the pyo3 build via
# RUSTFLAGS — it is NOT a Cargo feature), together with the pyo3 dependency:
RUSTFLAGS='--cfg feature="python"' maturin build --release --cargo-extra-args "--features pyo3"
```

Run the HTTP server (REST API + static frontend):

```bash
cargo run --release --features server --bin taikyokushogi-server
# open http://localhost:8000
```

Run examples/benchmarks:

```bash
cargo run --release --example bench_fixed -- 4 5 6
cargo run --release --example bench_nps
```

See `examples/README.md` for more details on examples and flags.

## Performance

Measured on the development machine (Intel Xeon E3-1230 V2, 4c/8t) with the
release-mode benchmarks (`cargo run --release --example bench_nps` and
`--example bench_fixed 1 2 3 4`). All search rows are **single-threaded**
(`threads = 1`). These are single-run measurements — expect roughly
±10–20 % run-to-run variance depending on machine load.

| Metric | Value |
|---|---|
| Legal moves from the initial position | 512 |
| Move generation (`legal_moves`) | ~76 µs/call |
| Static evaluation (`evaluate`) | ~51 µs/call |
| `apply` + `undo` round trip | ~52 µs/call |
| `Board::clone` (cost per Lazy-SMP worker) | ~62 µs/call |
| Perft(2) | 260,917 nodes @ ~10–18 M nodes/s |
| Search — depth 1 | ~13 ms (1,535 nodes) |
| Search — depth 2 | ~81 ms (39,392 nodes, ~490 k nps) |
| Search — depth 3 | ~100 ms (68,883 nodes, ~690 k nps) |
| Search — depth 4 | ~1.25 s (668,122 nodes, ~530 k nps) |
| Search — depth 5 | ~2.8 s (1,147,671 nodes, ~400 k nps) |
| Search — depth 6 | ≥ 30 s (10,581,487 nodes at ~350 k nps — hits the 30 s benchmark budget) |

Reproduce with:

```bash
cargo run --release --example check_movecounts   # -> 512 legal moves
./target/release/taikyokushogi-cli perft 2       # -> 260917
cargo run --release --example bench_fixed 1 2 3 4
cargo run --release --example bench_nps          # full component + NPS report
```

Every depth from 1 upward runs a real alpha-beta search: the old depth≤3
material-delta shortcut (which made shallow depths equivalent to depth 1)
was **removed outright** rather than left as a switch — see the historical
note in `src/search/root.rs`.

The search (`src/search/`) uses iterative deepening with aspiration
windows, principal-variation search (PVS), a transposition table with
lock-free concurrent access, killer / counter / history move ordering,
late move reductions, null-move pruning, razoring / reverse-futility /
futility / ProbCut pruning, internal iterative deepening, staged move
generation and quiescence search.

**Lazy SMP is opt-in, not automatic.** The default is one worker
(`search::DEFAULT_THREADS = 1`), which is also what every number above
measures. Workers are requested per call — CLI `--threads N` /
`setoption threads N` / `go threads N`, the per-side `threadsA` /
`threadsB` keywords of `selfplay` / `match` / `sprt`, or
`Board::search_threads(depth, ms, threads)` / `search::search_threads` from
the library. Up to `search::MAX_THREADS = 64` workers share one lock-free
transposition table; each worker rotates the root move ordering by its
index, and the master adopts a worker's result when that worker completed
a deeper iteration than the master did. Results with `threads > 1` are
**not bit-reproducible**.

### Strength testing

```bash
cargo run --release --example match_race -- 20 3 2 5000 4   # depth 3 vs depth 2, 20 games

# Lazy SMP vs single-thread at equal depth and equal time budget
./target/release/taikyokushogi-cli match 20 3 3 movetime 300 threadsA 4 threadsB 1
```

Head-to-head matches between any two search configurations (depths, time
budgets, handcrafted vs NNUE via the eval toggle) with alternating colors
and deterministic opening randomization. For publishable Elo claims, run
>= 1000 games and compute a confidence interval or SPRT bounds on the
W/L/D counts.

## Neural Evaluation (NNUE)

A neural evaluator (NNUE, architecture "S": piece classes × squares, no king
anchor, 180° rotation for White, pairwise 256 → 16 → 32 → 1 per output bucket)
replaces the hand-crafted score when a network is attached to a board:

```rust
use std::sync::Arc;
let net = Arc::new(taikyokushogi::Network::load("nets/tk.tkn")?);
let mut board = taikyokushogi::Board::initial();
board.set_network(Some(net));      // evaluate() and search() now use it
```

The network is a value (`Arc<Network>`), not a global, so different boards
and match sides can use different networks. Loading never falls back to random
weights: any problem is an error. CLI: `--net PATH` / `setoption net <path|none>`;
`PATH` may be a raw `.tkn` or a zstd-compressed `.tkn.zst` (detected by
content, not by extension). Web server: `TAIKYOKU_NET=nets/experimental-v1.tkn
cargo run --release --features server --bin taikyokushogi-server`
(`TAIKYOKU_NET=none` disables it); the GUI engine panel then shows
`Backend: nnue`.

The engine source is licensed under GPL-3.0-or-later. The NNUE network files
and the data they encode are a separate work licensed under the
[Open Database License (ODbL) 1.0](nets/LICENSE-ODbL.md). See
[`nets/README.md`](nets/README.md) for the release assets and the attribution
notice.

The accumulator is maintained incrementally by `Board::apply_move` with no
king anchor (no move forces a full refresh) and restored from a fixed ring of
snapshots on undo. Measured on one core in a release build: about 1 µs per
`evaluate()`, 60 µs for a full refresh. **No strength gain has been
demonstrated**; `nets/experimental-v1.tkn` is only there to try the pipeline
(see `nets/README.md`). Train a real one with the
workflow in [docs/NNUE.md](docs/NNUE.md) (`make nnue-data`, `nnue-train`, …)
and measure it with `match_race … sprt=0:10`. Tools: `cargo run --release
--bin nnue -- <classes|gen-classes|info|verify|random-net|dump-positions|bench>`.

## Module Structure

The crate's internal code is split into module trees by concern. Internal
modules are **not** `pub` — external users only see the public API in
`lib.rs` (`Board`, `Move`, `Square`, `Piece`, `Color`, `GameResult`,
`PieceInfo`, `SearchResult`).

| Directory | Responsibility |
|-----------|---------------|
| `src/lib.rs` | Public API facade: thin wrappers over internal types |
| `src/board/` | Board state, `apply`/`undo`, incremental scores, `game_result()` |
| `src/movegen/` | Move generation: sliding, jumping, range capture, igui, promotion |
| `src/types/` | Constants, cell encoding, square math, `Move`/`UndoInfo`, tables |
| `src/eval/` | Hand-crafted evaluator + NNUE evaluator |
| `src/search/` | PVS, Lazy SMP, transposition table, heuristics, ordering |
| `src/attack.rs` | Bitboard attack generation |
| `src/bitboard.rs` | 36×36 wide bitboards (21 u64 words) |
| `src/pieces.rs` | 301 piece definitions, Betza parser, promotions, setup |
| `src/tsfen.rs` | TSFEN position encoding/decoding |
| `src/elo.rs` | Engine-vs-engine matches, Elo + 95% CI, SPRT |
| `src/selfplay.rs` | Self-play game generation for NNUE training |
| `src/python.rs` | PyO3 Python bindings (feature-gated) |
| `src/main.rs` | axum HTTP server + web GUI (port 8000) |
| `src/cli.rs` | Interactive CLI REPL + one-shot commands |
| `src/debug_cli.rs` | Minimal one-shot search harness |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full module
dependency diagram, crate boundary rules, design rationale, and stability
policy.

## Using as a Rust Crate

Add to your `Cargo.toml`:

```toml
[dependencies]
taikyokushogi = "0.1"
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
```

## Cargo Features

- `cpu` (default) — the pure-CPU build (no alternative backend is compiled
  in today; the flag is a namespace for future ones).
- `gpu-cuda`, `gpu-metal`, `gpu-wgpu`, `gpu-vulkan` — enable the optional
  GPU crates (`burn`/`wgpu`). No code in this tree consumes them yet, so
  these features currently change nothing at runtime.
- `pyo3` (implicit feature of the optional `pyo3` dependency) — required to
  *compile* the Python extension module; the module itself
  (`src/python.rs`) is gated on the `python` cfg, which cargo does not know
  about and must be passed via `RUSTFLAGS='--cfg feature="python"'`
  (see the build section above).

## Credits and copyright

This was a *fork* of **[taikyokushogi](https://github.com/jh85/taikyokushogi)**, a complete engine for Taikyoku Shogi originally developed by **[jh85](https://github.com/jh85)** due to the differences I decided to make this project independent but I'm really grateful with the creator and recommend take a see I. he's project.

The original base code was released under the MIT License. The engine source
in this repository is distributed under the [GNU General Public License v3.0
or later](LICENSE). The NNUE network files and the data they encode are a
separate work distributed under the [Open Database License (ODbL)
1.0](nets/LICENSE-ODbL.md). We thank the original author for his excellent
work on the optimization and logic of the 36×36 board in Rust.

## References

- [Taikyoku shogi — Wikipedia](https://en.wikipedia.org/wiki/Taikyoku_shogi)
