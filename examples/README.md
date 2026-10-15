# Examples & Benchmarks

All 21 examples in `examples/`, run from the repo root as:

```bash
cargo run --release --example <name> -- [args...]
```

## Benchmarks

- `bench_fixed`: fixed-depth search from the initial position with **no
  time limit** (node counts are directly comparable between engine
  versions); **takes depth arguments**, default `4 5 6`:
  `cargo run --release --example bench_fixed -- 4 5 6`
- `bench_nps`: full NPS report — component timings (movegen, eval,
  apply+undo, clone), perft, search NPS by depth, selfplay/midgame NPS and
  a bottleneck summary. No arguments.
- `bench`: quick micro-benchmark of movegen / eval / depth-1 search. No
  arguments.
- `bench_deep`: deeper, longer-thinking search stress run. No arguments.
- `bench_depth1`: high-precision depth-1 NPS (loops internally — depth 1
  finishes too fast to time in a single run). No arguments.
- `bench_midgame`: `legal_moves()` cost in a realistic midgame position
  (12 selfplay plies) instead of the initial position. No arguments.
- `bench_thorough`: runs each measurement multiple times and reports
  medians + min/max, plus time-limit scaling (depth 4 from 250 ms to 10 s).
  No arguments (takes several minutes).
- `bench_time`: verifies searches actually respect `time_limit_ms`
  (depths 2–5). No arguments.
- `depth_quick`: builds a midgame (12 plies), then sweeps depths
  2/4/5/6/8/10 with a 2 s budget each. No arguments.
- `depth_time`: untimed fixed-depth search (depths 1–5) from the start
  position — wall-clock per depth. No arguments.

## Correctness / smoke tests

- `check_movecounts`: prints the legal move count from the initial
  position (`512`) and an approximate capture/promotion split. No arguments.
- `perft`: perft from the initial position; **optional depth argument**
  (default 2): `cargo run --release --example perft -- 3`
- `perft_breakdown`: perft(1) broken down by piece type; checks the total.
  No arguments.
- `oracle_walk`: random-walk invariant oracle — after every apply/undo
  verifies TSFEN, incremental material and piece counts round-trip
  exactly (exercises hooks, range captures, lions, promotions). No
  arguments.
- `stress_undo`: apply/undo stress test that checks the board state
  round-trips (builds a midgame first). No arguments.
- `toggle_nnue`: runtime NNUE on/off smoke test (compare code paths).
  No arguments.
- `check_db`: sanity checks of the self-play SQLite database
  (`training_data/games.db`). No arguments.
- `play`: plays a full game from the initial position with real search
  (prints every move, nodes and time) until `game_result()` ends the game
  or one side has no legal move. No arguments.

## Strength testing

- `match_race`: head-to-head matches with Elo + 95% CI (and optional
  GSPRT) between two configurations. **Positional args:**
  `games [depthA] [depthB] [time_ms] [opening_plies]`; **optional flags:**
  `nnueA`, `nnueB`, `sprt=elo0:elo1`. Examples:

  ```bash
  # depth 3 vs depth 2, 100 games, report every batch
  cargo run --release --example match_race -- 100 3 2 2000 6
  # SPRT: is depth 3 at least +10 Elo over depth 2?
  cargo run --release --example match_race -- 400 3 2 2000 6 sprt=0:10
  # handcrafted vs NNUE (needs TAIKYOKU_NNUE_PATH)
  TAIKYOKU_NNUE_PATH=net.nnue cargo run --release --example match_race -- 100 3 3 2000 6 nnueB
  ```

  Deterministic (seeded LCG): identical invocations reproduce identical
  games. The CLI's `match`/`sprt` commands offer the same machinery with
  `threadsA`/`threadsB` for Lazy-SMP A/B tests (see the main README).

## NNUE pipeline tooling

- `export_piece_metadata`: writes `training/piece_metadata.json`:
  `cargo run --release --example export_piece_metadata > training/piece_metadata.json`
- `nnue_speed`: micro-benchmark of `evaluate()` — from-scratch refresh vs
  the incrementally-maintained accumulator. No arguments.
