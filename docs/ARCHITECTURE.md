# Architecture — quick overview

Module responsibilities:

- `src/board.rs`: board representation, rules, `apply`/`undo`, the
  incrementally-maintained material/PSQT scores and NNUE accumulator, and
  `game_result()` (royal-capture wins, both-sides-lost / 500-move draws).
- `src/movegen.rs`: legal move generation. There is **no check filtering**:
  the variant has no check rule, so `generate_legal_moves` never rejects a
  move for leaving a royal piece en prise.
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
