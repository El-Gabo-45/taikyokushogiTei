# Architecture — quick overview

## Module dependency diagram

```text
                    ┌─────────────────────────────┐
                    │     PUBLIC API (lib.rs)     │
                    │  Board, Move, Square, Piece │
                    │  Color, GameResult, etc.    │
                    └───────────┬─────────────────┘
                                │ (re-exports / thin wrappers)
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
┌───────▼───────┐    ┌────────▼────────┐    ┌────────▼─────────┐
│   board/      │    │   movegen/      │    │    types/        │
│   core.rs     │    │   core.rs       │    │  constants.rs    │
│   mod.rs      │    │   mod.rs        │    │  cell.rs         │
└───────┬───────┘    └───────┬─────────┘    │  square.rs       │
        │                   │               │  mv.rs             │
        │                   │               │  movement.rs       │
        │                   │               │  tables.rs         │
        │                   │               │  mod.rs            │
        │                   │               └────────┬──────────┘
        │                   │                        │
        │        ┌──────────┴──────────┐             │
        │        │   attack.rs         │             │
        │        │   bitboard.rs       │             │
        │        │   pieces.rs         │             │
        │        │   tsfen.rs          │             │
        └────────┴──────────┬──────────┴─────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
┌──────▼──────┐   ┌──────▼──────┐   ┌──────▼──────┐
│   eval/     │   │   search/   │   │   elo.rs      │
│   mod.rs    │   │   mod.rs    │   │   selfplay.rs │
│   families  │   │   pvs.rs    │   │   cli.rs      │
│   psqt.rs   │   │   root.rs   │   │   main.rs     │
│   zones.rs  │   │   smp.rs    │   │   usi.rs      │
│   nnue/     │   │   tt.rs     │   │   python.rs   │
└─────────────┘   └─────────────┘   │   debugging/  │
                                    └───────────────┘
```

### Crate boundary rules

| Layer | Visibility | Can depend on |
|-------|-----------|---------------|
| `lib.rs` public items | `pub` (external users) | Anything in `crate` |
| `board/`, `movegen/`, `types/`, `eval/`, `search/` | `mod` (crate-internal) | Other internal modules via `crate::` |
| Examples, tests, binaries | External to `crate` | Only `taikyokushogi` public API |

No internal module is `pub`, so external crates **cannot** depend on
`crate::board::Board`, `crate::movegen`, or `crate::types` directly.
All external access goes through the public facade in `lib.rs`.

- `src/board/`: board representation, rules, `apply`/`undo`, the
  incrementally-maintained material/PSQT scores and NNUE accumulator, and
  `game_result()` (royal-capture wins, both-sides-lost / 500-move draws).
  - `core.rs` — `Board` struct, `Clone`, `new()`, `setup_initial()`,
    `place_piece()`, `at()`, `clone_without_history()`
  - `mod.rs` — re-exports `Board` as `crate::board::Board`
- `src/movegen/`: legal move generation. There is **no check filtering**:
  the variant has no check rule, so `generate_legal_moves` never rejects a
  move for leaving a royal piece en prise.
  - `core.rs` — all move generation logic: sliding, jumping, range capture,
    igui, promotion, dedup
  - `mod.rs` — re-exports everything from `core`
- `src/types/`: core types and precomputed tables.
  - `constants.rs` — board dimensions (36×36), colors, direction constants,
    piece ranks (`RANK_*`), draw threshold
  - `cell.rs` — `Cell` encoding, `EMPTY_CELL`, `INVALID_SQ`, `make_cell`,
    `cell_piece`, `cell_color`, `sq_index`
  - `square.rs` — `sq_row`, `sq_col`, `step_sq`, `get_deltas`, precomputed
    row/column lookup tables
  - `mv.rs` — internal `Move` struct and `UndoInfo`
  - `movement.rs` — `Movement` struct and `HookType` enum
  - `tables.rs` — precomputed ray table, Zobrist hash keys, promotion-zone
    and farthest-rank lookup tables
  - `mod.rs` — re-exports from all submodules so `use crate::types::*`
    continues to work
- `src/search/`: search algorithms.
  - `mod.rs` — iterative deepening + aspiration windows; the `search`,
    `search_threads` (Lazy SMP) and `search_with_stop` (stop flag +
    per-iteration info callback, consumed by the USI front-end) entry
    points; hash-size configuration.
  - `pvs.rs` — PVS core: null-window re-search, LMR, null-move pruning,
    razoring / reverse-futility / futility / static-eval prune, IID, staged movegen.
  - `root.rs` — root iteration (the historical depth 1–3 material-delta
    fast path was removed outright, not toggled off).
  - `smp.rs` — Lazy SMP: `SharedBest` (lock-free "best completed
    iteration"), per-worker root-order rotation, deadline sharing.
  - `tt.rs` — lock-free bucketed transposition table; `buffers.rs` — pooled
    per-node scratch; `heuristics.rs` / `ordering.rs` — move ordering;
    `qsearch.rs` — capture-only quiescence; `params.rs` — every tunable
    constant, including `DEFAULT_THREADS = 1` and `MAX_THREADS = 64`.
- `src/eval/`: evaluators — hand-crafted (`families.rs`, `psqt.rs`,
  `zones.rs`) and `nnue/` (NNUE "S", see `docs/NNUE.md`). A network is
  attached to a `Board` at **runtime** with `set_network(Some(Arc<Network>))`;
  there is no global backend switch and no cargo feature involved.
- `src/cli.rs` → bin `taikyokushogi-cli`: user-facing REPL plus argv flags
  (`--depth`, `--movetime`, `--threads`, `--hash`, `--position`, `--go`,
  `--perft`, `--help`) and `&&`-chained one-shot commands.
- `src/main.rs` → bin `taikyokushogi-server` (feature `server`): thin
  launcher; the axum router, handlers and JSON types live in
  `src/server/{mod,handlers,types}.rs` inside the library so integration
  tests can drive the whole API in-process. Serves the REST API and the
  static web GUI (port 8000; `TAIKYOKU_BIND`, optional `TAIKYOKU_NET`).
- `src/elo.rs`: engine-vs-engine matches, Elo + 95% CI, SPRT
  (`EngineConfig { depth, time_ms, net, threads }` per side; `net` is an
  optional `Arc<Network>`).
- `src/selfplay.rs` + `src/selfplay_main.rs` → bin `selfplay`: self-play
  game generation for the NNUE training pipeline.
- `src/debug_cli.rs` → bin `debug-cli`: minimal one-shot search harness
  (`debug-cli <depth> <time_ms> [--debug]`).
- `src/usi.rs` + `src/usi_main.rs` → bin `taikyokushogi-usi`: USI
  (Universal Shogi Interface) front-end so shogi GUIs (ShogiGUI,
  ShogiHome, …) and USI harnesses can drive the engine. The protocol lives
  in the library (`taikyokushogi::usi`, unit-tested) and the bin is a thin
  launcher: TSFEN positions, 36×36 coordinate move tokens, clock handling
  (`wtime`/`btime`/`winc`/`byoyomi`/`movetime`), `stop`/`ponder` support
  and the `Threads`/`Hash`/`DepthLimit`/`NetworkFile` options.
- `training/`: PyTorch pipeline to train and export `.tkn` networks.
- `web/`: TypeScript + Vite frontend and REST client
  (`web/src/api/client.ts`).

Tests: `src/correctness_tests.rs` plus per-module unit tests, doc-tests and
the integration suites in `tests/`: `smoke.rs` (engine) and `server_api.rs`
(full HTTP router in-process, `cargo test --features server --test
server_api`) — all run with plain `cargo test`.

Typical NNUE workflow (details and quality gates in `docs/NNUE.md`):

1. `selfplay … --out data --opening-plies 8` writes `TKD1` samples.
2. `training/check_label_units.py` (unit gate) and
   `training/calibrate_buckets.py` (output-bucket thresholds).
3. `training/train.py` trains and exports a `TKN1` v2 file.
4. `nnue dump-positions` + `training/parity_gate.py` prove Rust == Python.
5. `Network::load(path)` and `board.set_network(Some(net))` (CLI: `--net PATH`).

Integration notes:

- NNUE features are (piece class, square) with no king anchor; the accumulator
  is maintained incrementally in `Board::apply_move` and restored from a fixed
  ring of snapshots on undo.
- Lazy SMP is **opt-in per call** (CLI `--threads` / `setoption threads` /
  `go threads` / `threadsA`/`threadsB` in `selfplay|match|sprt` /
  `Board::search_threads`), defaults to 1 worker, and is capped at 64;
  workers share one lock-free TT and results with `threads > 1` are not
  bit-reproducible.
- Main binaries: `taikyokushogi-server`, `taikyokushogi-cli`,
  `taikyokushogi-usi`, `debug-cli` and `selfplay`.

## Modularization notes (v0.1.0)

The following internal modules were split from single large files into
module trees to improve maintainability and testability:

| Before | After | Rationale |
|--------|-------|-----------|
| `src/types.rs` | `src/types/{constants,cell,square,mv,movement,tables}.rs` | Separate constants, cell encoding, square math, move struct, movement rules, and precomputed tables |
| `src/board.rs` | `src/board/{core,mod}.rs` | `Board` struct in `core.rs`; `mod.rs` re-exports for `crate::board::Board` |
| `src/movegen.rs` | `src/movegen/{core,mod}.rs` | Move generation logic in `core.rs`; `mod.rs` re-exports |

No public API changed — all user-facing types and functions remain
identical. Internal code accesses `crate::board::Board`, `crate::movegen`,
and `crate::types` via `crate::` paths (visible within the crate only).

## Design rationale

### Why modularize?

The original codebase had three "god files" (`types.rs`, `board.rs`,
`movegen.rs`) totaling >10,000 lines that mixed unrelated concerns:
constants, cell encoding, piece movement, board state, move generation,
and precomputed tables. Splitting them into module trees:

- **Improves stability**: changes to one concern (e.g., cell encoding)
  cannot accidentally break unrelated code (e.g., move generation)
- **Improves testability**: each submodule can be unit-tested in
  isolation without loading the entire board/movegen graph
- **Improves navigability**: new contributors can find code by concern
  rather than scrolling through multi-thousand-line files

### Why no public exposure of internal modules?

`board/`, `movegen/`, and `types/` are deliberately `mod` (private) rather
than `pub mod`. This gives the maintainers freedom to refactor internals
without breaking external users. The public API in `lib.rs` is the
contract; everything else is implementation detail.

### Why `panic = "abort"` in release?

The release profile uses `panic = "abort"` to minimize binary size and
maximize search speed. As a side effect, external integration tests in
`tests/` cannot link against the crate. All testing therefore lives
inside the crate (`src/correctness_tests.rs`, per-module `#[cfg(test)]`,
and doc-tests) and runs with `cargo test`.

## Stability policy

| Aspect | Stability | Notes |
|--------|-----------|-------|
| Public API (`lib.rs` items) | **Stable** | Breaking changes require a minor version bump |
| Internal modules (`board/`, `movegen/`, `types/`, etc.) | **Unstable** | May change between any releases |
| `Cargo.toml` features | **Stable** | `cpu`, `gpu-*`, `pyo3` semantics preserved (the unused `nnue` feature was removed) |
| CLI flags and REPL commands | **Stable** | Documented in `cli.rs` and `--help` |
| TSFEN format | **Stable** | Round-trip guaranteed for all legal positions |
| `.tkn` network file format | **Stable** | TKN1 v2, defined in `src/eval/nnue/format.rs` and `docs/NNUE.md` |
| `.bin` training sample format | **Stable** | `TKD1` v1, sparse records, matches `training/dataset.py` |
