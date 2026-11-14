# Examples & Benchmarks

The examples in `examples/`, run from the repo root as:

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
  GSPRT) between two configurations of **the same binary**. **Positional
  args:** `games [depthA] [depthB] [time_ms] [opening_plies]`; **optional
  flags:** `netA=PATH`, `netB=PATH`, `sprt=elo0:elo1`. Examples:

  ```bash
  # depth 3 vs depth 2, 100 games, report every batch
  cargo run --release --example match_race -- 100 3 2 2000 6
  # SPRT: is depth 3 at least +10 Elo over depth 2?
  cargo run --release --example match_race -- 400 3 2 2000 6 sprt=0:10
  # handcrafted (A) vs NNUE (B)
  cargo run --release --example match_race -- 100 3 3 2000 6 netB=nets/experimental-v1.tkn.zst
  # two different networks head to head
  cargo run --release --example match_race -- 100 3 3 2000 6 netA=a.tkn netB=b.tkn
  ```

  Deterministic (seeded LCG): identical invocations reproduce identical
  games. The CLI's `match`/`sprt` commands offer the same machinery with
  `threadsA`/`threadsB` for Lazy-SMP A/B tests (see the main README).

- `elo_report`: measures the engine's **current playing strength** as a
  Stockfish-style self-referential **Elo depth ladder** — depth `d` plays
  depth `d−1` on seeded random openings and the pairwise Elo gains
  accumulate into one rating anchored at depth 1 = 0. Refreshes the
  `<!-- elo-report:begin/end -->` block in a markdown file and/or writes a
  JSON report for scripts/CI. **Flags:** `--games N` (per rung, default
  40), `--max-depth D` (default 3), `--movetime MS` (default 300 — must
  stay above the top depth's cost, else that rung is truncated by the
  clock), `--maxplies P` (default 250), `--open K` (default 8),
  `--threads N`
  (default 1 — reproducible), `--net PATH`, `--json PATH`, `--md PATH`.
  `make strength` runs the canonical invocation (updates `docs/STRENGTH.md`
  and `bench/elo.json`):

  ```bash
  make strength
  # or directly:
  cargo run --release --example elo_report -- \
      --games 40 --max-depth 3 --movetime 300 --md docs/STRENGTH.md --json bench/elo.json
  ```

  Results and methodology: [docs/STRENGTH.md](../docs/STRENGTH.md).

- `version_race`: the cross-version counterpart — matches between two
  engine **executables** (e.g. the current build vs an older commit), so
  code changes themselves can be measured. The harness only orchestrates
  (`position <tsfen>` + `go`, reading `bestmove`); it never searches, so
  both sides are genuinely different builds. **Positional args:** `games
  [depth] [time_ms] [opening_plies] [maxplies]`; **flags:** `engineA=PATH`,
  `engineB=PATH`, `netA=`, `netB=`, `depthA=`, `depthB=`, `threadsA=`,
  `threadsB=`, `hash=`, `argA=`, `argB=`, `sprt=elo0:elo1`. The `make
  version-race` target automates building the old commit in a git worktree:

  ```bash
  # current vs the previous commit (builds the old ref for you)
  make version-race
  make version-race OLD=HEAD~5
  # with NNUE on both sides, plus an SPRT stop rule
  make version-race OLD=HEAD~3 RACE_NET=nets/experimental-v1.tkn.zst RACE_ARGS="sprt=0:10"
  # or run the harness directly with your own two binaries
  cargo run --release --example version_race -- 40 2 2000 6 \
    engineA=target/release/taikyokushogi-cli engineB=/path/to/old-cli
  ```

  Engines run as one persistent process each (warm transposition table, as
  in real play). Newer builds also emit a `mid <sq>` token after `bestmove`
  to replay igui/lion mid-captures exactly; older builds fall back to the
  same move choice the CLI makes for a human-typed move.

## NNUE pipeline tooling

- `export_piece_metadata`: writes `training/piece_metadata.json`:
  `cargo run --release --example export_piece_metadata > training/piece_metadata.json`
- `nnue_speed`: micro-benchmark of `evaluate()` — from-scratch refresh vs
  the incrementally-maintained accumulator. No arguments.
