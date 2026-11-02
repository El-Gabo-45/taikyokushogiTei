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
│   zones.rs  │   │   smp.rs    │   │   python.rs   │
│   nnue.rs   │   │   tt.rs     │   │   debugging/  │
└─────────────┘   └─────────────┘   └───────────────┘
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
  - `mod.rs` — iterative deepening + aspiration windows; the `search` and
    `search_threads` (Lazy SMP) entry points; hash-size configuration.
  - `pvs.rs` — PVS core: null-window re-search, LMR, null-move pruning,
    razoring / reverse-futility / futility / ProbCut, IID, staged movegen.
  - `root.rs` — root iteration (the historical depth 1–3 material-delta
    fast path was removed outright, not toggled off).
  - `smp.rs` — Lazy SMP: `SharedBest` (lock-free "best completed
    iteration"), per-worker root-order rotation, deadline sharing.
  - `tt.rs` — lock-free bucketed transposition table; `buffers.rs` — pooled
    per-node scratch; `heuristics.rs` / `ordering.rs` — move ordering;
    `qsearch.rs` — capture-only quiescence; `params.rs` — every tunable
    constant, including `DEFAULT_THREADS = 1` and `MAX_THREADS = 64`.
- `src/eval/`: evaluators — hand-crafted (`families.rs`, `psqt.rs`,
  `zones.rs`) and `nnue.rs` (HalfKP-style NNUE). Selected at **runtime**
  via `set_use_nnue(bool)`; the NNUE module compiles unconditionally (the
  `nnue` cargo feature only pulls optional dependencies and is not needed
  to use it).
- `src/cli.rs` → bin `taikyokushogi-cli`: user-facing REPL plus argv flags
  (`--depth`, `--movetime`, `--threads`, `--hash`, `--position`, `--go`,
  `--perft`, `--help`) and `&&`-chained one-shot commands.
- `src/main.rs` → bin `taikyokushogi-server`: axum HTTP server, REST API,
  static web GUI (port 8000).
- `src/elo.rs`: engine-vs-engine matches, Elo + 95% CI, SPRT
  (`EngineConfig { depth, time_ms, nnue, threads }` per side).
- `src/selfplay.rs` + `src/selfplay_main.rs` → bin `selfplay`: self-play
  game generation for the NNUE training pipeline.
- `src/debug_cli.rs` → bin `debug-cli`: minimal one-shot search harness
  (`debug-cli <depth> <time_ms> [--debug]`).
- `training/`: PyTorch pipeline to train and export `.nnue` files.
- `web/`: TypeScript + Vite frontend and REST client
  (`web/src/api/client.ts`).

Tests: `src/correctness_tests.rs` plus per-module unit tests and
doc-tests — all run with plain `cargo test`. There is deliberately **no
`tests/` directory**: the release profile uses `panic = "abort"`, which
external integration-test targets cannot link against.

Typical NNUE workflow:

1. Run `selfplay` to produce `.bin` samples in `training_data/`.
2. Parse them with `training/dataset.py` (HalfKP feature extraction lives
   in `training/features.py`).
3. Train the network using `training/train.py`.
4. Export weights using `training/export.py` to a `.nnue` file.
5. Load the `.nnue` at runtime via the `TAIKYOKU_NNUE_PATH` environment
   variable and call `taikyokushogi::set_use_nnue(true)` (or the CLI's
   `setoption nnue on`).

Integration notes:

- NNUE features use a HalfKP-style encoding; the accumulator is maintained
  incrementally in `Board::apply_move` and restored in O(1) on undo.
- Lazy SMP is **opt-in per call** (CLI `--threads` / `setoption threads` /
  `go threads` / `threadsA`/`threadsB` in `selfplay|match|sprt` /
  `Board::search_threads`), defaults to 1 worker, and is capped at 64;
  workers share one lock-free TT and results with `threads > 1` are not
  bit-reproducible.
- Main binaries: `taikyokushogi-server`, `taikyokushogi-cli`, `debug-cli`
  and `selfplay`.

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
| `Cargo.toml` features | **Stable** | `cpu`, `nnue`, `gpu-*`, `pyo3` semantics preserved |
| CLI flags and REPL commands | **Stable** | Documented in `cli.rs` and `--help` |
| TSFEN format | **Stable** | Round-trip guaranteed for all legal positions |
| `.nnue` file format | **Stable** | Defined in `src/eval/nnue.rs` serialization docs |
| `.bin` training sample format | **Stable** | 2612 bytes, little-endian, matches `training/dataset.py` |
