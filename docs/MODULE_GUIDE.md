# Module Guide — Developer's Reference

This guide explains how the Taikyoku Shogi engine's internal code is
organized, where to add new code, and how the module boundaries work. It
complements [ARCHITECTURE.md](ARCHITECTURE.md) (high-level design) and
[CONTRIBUTING.md](../CONTRIBUTING.md) (process).

## Top-level layout

```
src/
  lib.rs          — Public API facade (thin wrappers over internal types)
  board/          — Board state, apply/undo, game_result, incremental scores
  movegen/        — Move generation (sliding, jumping, range capture, igui)
  types/          — Core types, constants, cell encoding, tables
  eval/           — Hand-crafted + NNUE evaluators
  search/         — PVS, Lazy SMP, TT, heuristics, ordering
  attack.rs       — Bitboard attack generation
  bitboard.rs     — 36×36 wide bitboards (21 u64 words)
  pieces.rs       — 301 piece definitions, Betza parser, setup
  tsfen.rs        — TSFEN position encoding/decoding
  elo.rs          — Engine-vs-engine matches, Elo, SPRT
  selfplay.rs     — Self-play game generation for NNUE training
  usi.rs          — USI (Universal Shogi Interface) front-end
  python.rs       — PyO3 bindings (feature-gated)
  debugging/      — Debug helpers for eval and search
  main.rs         — axum HTTP server (+ web GUI)
  server/         — axum router, handlers, JSON types (library module; driven by tests/server_api.rs)
  nnue_main.rs    — NNUE tool entry point (bin nnue; see docs/NNUE.md)
  cli.rs          — Interactive CLI REPL
  debug_cli.rs    — Minimal one-shot search harness
```

## `src/board/` — Board state and rules

| File | Contents |
|------|----------|
| `core.rs` | `Board` struct (~50 fields), `Clone`, `new()`, `setup_initial()`, `place_piece()`, `at()`, `clone_without_history()`, `apply_move()`, `undo_move()`, `null_move()`, `undo_null_move()`, `king_square()`, `game_result()`, `display()`, bitboard occupancy maintenance, `set_network()`, NNUE accumulator updates |
| `mod.rs` | Re-exports `Board` as `crate::board::Board` |

### When to touch `board/`

- Changing board state fields → `core.rs`
- Modifying how moves are applied/undone → `core.rs`
- Changing royal piece tracking → `core.rs`
- Changing draw/no-progress rules → `core.rs`

**Do not** put search algorithms, evaluators, or piece movement definitions here.

## `src/movegen/` — Move generation

| File | Contents |
|------|----------|
| `core.rs` | All move generation: `generate_legal_moves()`, sliding, jumping, range capture, igui, promotion, dedup (`dedup_piece_range()`, `push_move_raw()`) |
| `mod.rs` | Re-exports everything from `core` |

### When to touch `movegen/`

- Adding a new movement type → `core.rs`
- Fixing move generation bugs → `core.rs`
- Optimizing move generation speed → `core.rs`

**Do not** put board state or search code here.

**Important**: There is **no check filtering** — the variant has no check rule,
so `generate_legal_moves()` never rejects a move for leaving a royal piece en
prise.

## `src/eval/` — Evaluation

| File | Contents |
|------|----------|
| `mod.rs` | `evaluate()` entry point (uses the NNUE network attached to the board, if any), `material_score()`, `MATE_SCORE`, king safety |
| `families.rs` | `Family` enum, `classify()`, `classify_fast()`, `family_value()`, `family_value_fast()`, precomputed tables |
| `psqt.rs` | Piece-Square Tables, `psqt()`, `psqt_score()`, `psqt_delta()`, precomputed PSQT table |
| `zones.rs` | `Zone` enum, `zone_score()`, `zone_family_bonus()` |
| `nnue/` | NNUE "S" evaluator: `mod.rs` (`Network`, `Accumulator`, forward), `arch.rs` (single source of truth), `classes.rs` (piece classes), `format.rs` (TKN1 v2), `dump.rs` (parity dumps) |

### When to touch `eval/`

- Changing the hand-crafted eval formula → `mod.rs` / `families.rs` / `zones.rs`
- Changing PSQT values → `psqt.rs`
- Working on NNUE → `nnue/` (see `docs/NNUE.md`)

## `src/search/` — Search algorithms

| File | Contents |
|------|----------|
| `mod.rs` | `Searcher` struct, iterative deepening, aspiration windows, `search()`, `search_threads()`, `search_with_stop()` (stop flag + per-iteration `IterInfo` callback, used by the USI front-end), `set_hash_mb()`, `DEFAULT_THREADS=1`, `MAX_THREADS=64` |
| `pvs.rs` | PVS core: null-window re-search, LMR, null-move pruning, razoring, reverse-futility, futility, static-eval prune, IID, staged movegen |
| `root.rs` | Root iteration |
| `smp.rs` | Lazy SMP: `SharedBest`, per-worker root-order rotation |
| `tt.rs` | Lock-free bucketed transposition table |
| `buffers.rs` | Pooled per-node scratch buffers |
| `heuristics.rs` | Killer moves, butterfly history, counter moves |
| `ordering.rs` | Move scoring (TT > MVV-LVA > killers > history) |
| `qsearch.rs` | Capture-only quiescence |
| `params.rs` | All tunable constants and margins |

### When to touch `search/`

- Adding a new search technique → `params.rs` (toggle) + relevant file (implementation)
- Tuning search constants → `params.rs`
- Fixing search bugs → the relevant algorithm file (`pvs.rs`, `root.rs`, etc.)

## `src/usi.rs` — USI protocol front-end

| File | Contents |
|------|----------|
| `usi.rs` | The whole USI command surface: `usi` / `isready` / `setoption` / `usinewgame` / `position` / `go` / `stop` / `ponderhit` / `gameover` / `quit`, `GoParams` parsing + time-budget policy, TSFEN/coordinate notation parsing & formatting, `info` formatting (cp / mate / nps / pv), plus unit tests |
| `usi_main.rs` | Thin bin wrapper (`taikyokushogi-usi`) around `taikyokushogi::usi::run()` |

The module header docs define the **Taikyoku Shogi USI profile** (TSFEN
positions, 36×36 coordinate notation, supported `go` parameters, options);
keep it in sync with the implementation.

### When to touch `usi.rs`

- Adding or changing a USI command / option → `usi.rs`
- Time-management policy → `budget_from_params`
- Move-text notation → `usi_move_str` / `parse_move`; note
  `split_two_squares` — squares are variable length (`e34`, `A12`), so
  never assume fixed-width 4-char move tokens
- The search thread owns the `Searcher` (moved in/out of a
  `JoinHandle<Searcher>`) so the TT stays warm between moves; `usinewgame`
  clears it

**Note**: `usi` is one of the few `pub mod` declarations in `lib.rs`
(alongside `search`, `elo`, `selfplay`, `server`) because the
`taikyokushogi-usi` binary is a separate crate that calls
`taikyokushogi::usi::run()`. Everything else stays crate-internal.

## `src/lib.rs` — Public API facade

This is the **only** module visible to external users. It defines:

- `Board`, `Move`, `Square`, `Piece`, `Color`, `GameResult`, `PieceInfo`, `SearchResult`
- Free functions: `piece_info()`, `set_hash_mb()`, `eval_only()`
- The `python` pymodule entry point

Each public type is a thin wrapper over an internal type:

```rust
// Public facade type
pub struct Board { inner: board::Board }

// Internal type (not visible externally)
pub struct Board {  // in src/board/core.rs
    pub cells: [Cell; NUM_SQUARES],
    pub side_to_move: u8,
    // ... all fields are `pub` within the crate
}
```

### Rule: never add `pub mod` for internal code

All internal modules (`board`, `movegen`, `types`, `eval`, `bitboard`,
`pieces`, `tsfen`, `attack`) are declared as plain `mod` in `lib.rs`, never
`pub mod`. This ensures external crates cannot depend on implementation
details. The exceptions — `search`, `elo`, `selfplay`, `usi` and
`server` — expose stable, testable entry points (binaries live in separate
crates, e.g. `usi_main.rs`, and external tooling drives `search`/`elo`).

## Adding a new file: checklist

1. **Pick the right directory** by concern:
   - Board state/rules → `src/board/core.rs` (or new submodule)
   - Move generation → `src/movegen/core.rs`
   - Core types/tables → `src/types/`
   - Evaluation → `src/eval/`
   - Search → `src/search/`
   - Utilities → existing flat file or new file in `src/`

2. **Create `mod.rs`** if you're making a new directory:
   ```rust
   // src/foo/mod.rs
   pub use self::core::*;  // or selective re-exports
   mod core;
   ```

3. **Declare the module** in the parent `mod.rs` or `lib.rs`:
   ```rust
   // Plain `mod`, never `pub mod` for internal code
   mod foo;
   ```

4. **Add tests** in a `#[cfg(test)] mod tests` at the bottom of the file.

5. **Update documentation** if the change affects the public API:
   - `README.md` — public API examples
   - `docs/ARCHITECTURE.md` — module diagram if structure changed
   - `CONTRIBUTING.md` — if the module structure table changed

6. **Verify**: `cargo build --all-targets` and `cargo test` must both pass.

## Stability boundaries

| Boundary | Stability | Can change? |
|----------|-----------|-------------|
| `taikyokushogi` public API (`lib.rs` items) | Stable | Only with minor version bump |
| Internal module file layout | Unstable | Any release |
| Internal struct fields | Unstable | Any release |
| TSFEN format | Stable | No |
| `.tkn` (TKN1 v2) network format | Stable | No |
| `.bin` training sample format | Stable | No |
| CLI flags / REPL commands | Stable | Only with minor version bump |

## Common pitfalls

- **Don't make internal modules `pub`**: this locks the implementation and prevents future refactoring.
- **Don't put board state in `types/`**: types should only contain primitives and precomputed tables.
- **Don't put search logic in `board/`**: board should only know about state, not about searching.
- **Don't add `pub` fields to `Board` unless external users truly need them**: prefer accessor methods on the public `Board` wrapper.
- **Don't forget `#[cfg(test)] mod tests`**: every non-trivial module should have unit tests.