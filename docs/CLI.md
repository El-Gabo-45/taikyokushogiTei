# CLI — `taikyokushogi-cli`

Full user-facing CLI (not just a debug harness): interactive REPL plus
one-shot argv commands.

```bash
cargo build --release --bin taikyokushogi-cli
./target/release/taikyokushogi-cli                 # interactive REPL
./target/release/taikyokushogi-cli perft 2         # one-shot commands (`&&` chains)
./target/release/taikyokushogi-cli --help          # full flag reference
```

## Command-line flags (non-interactive)

```text
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

## Commands (REPL or one-shot)

| Command | Description |
| --------- | ------------- |
| `new`, `startpos`, `position <tsfen>` | reset / load a TSFEN position |
| `board`, `status`, `fen` | show board, status line, or TSFEN |
| `legal [<square>]` | list legal moves |
| `move <mv>` | apply a move (`f19f20`, `f19-f20`, `f19f20+`, `19,17-20,17`, igui `c5c5`) |
| `undo`, `history` | take back / list the session's moves |
| `go [depth N \| movetime MS \| threads N]`, `hint` | search and print the best move (does not apply it) |
| `play <black\|white\|both>`, `stop` | engine plays a side automatically |
| `eval`, `perft <N>`, `bench` | evaluation, node counts, micro-benchmarks |
| `bench search [D]` | **deterministic Stockfish-style bench**: fixed positions, depths 1..=D, single-threaded, untimed — node counts and the final `bench hash` line are bit-reproducible regression tests (diff that line across commits) |
| `debug eval` | evaluation breakdown: active backend + full hand-crafted decomposition (material / family / zones / royal-center / king-safety / per-piece), cross-checked against the engine score |
| `debug nnue` | NNUE forward-pass breakdown of the current position (bucket, accumulator stats, PSQT, dense, incremental==refresh parity) — needs a network attached (`setoption net`) |
| `debug search [D]` | per-iteration deepening trace (depth / score / nodes / time / best) |
| `save <file.json>` / `load <file.json>` | save/load a game (TSFEN + move list) |
| `selfplay [G D PLIES MS] [threadsA N] [threadsB N] [open K] [seed S]` | engine-vs-engine games (per-side Lazy-SMP workers) |
| `match <G> <dA> <dB> [movetime MS] [maxplies N] [threadsA N] [threadsB N] [open K] [seed S]` | match between two configs + **Elo estimate** |
| `sprt <elo0> <elo1> [G] [dA] [dB] [MS] [threadsA N] [threadsB N] [open K] [seed S]` | SPRT test between two configs |
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
  `maxplies`, `threadsA`, `threadsB`, `open`, `seed`) are case-insensitive
  and come **after** the positional arguments; a bad value warns and keeps
  the previous/default value. `movetime` applies to **both** sides.
- `open K` plays `K` uniformly random legal moves before the engines take
  over (0 = default, the old deterministic initial position); `seed S`
  seeds the opening randomizer so a match is reproducible while its games
  still diverge.

## Debugging

`debug` is the Stockfish-style introspection group — for engine
development, not for playing:

```bash
./target/release/taikyokushogi-cli "debug eval"      # eval breakdown + cross-check
./target/release/taikyokushogi-cli --net nets/experimental-v1.tkn.zst "debug nnue"
./target/release/taikyokushogi-cli "debug search 4"  # deepening trace
```

- **`debug eval`** prints the active evaluation backend and a full
  decomposition of the hand-crafted score (material, family weights,
  zones, royal-center penalty, king safety, per-piece counts). It
  cross-checks the decomposition against the engine's own score and
  prints `✓` / `MISMATCH` — a mismatch means an incremental-update bug.
- **`debug nnue`** prints the NNUE forward pass for the current position:
  selected bucket, accumulator statistics per perspective (min/max/mean/
  clipped counts), PSQT term, dense output and the incremental-vs-refresh
  parity check (a mismatch there means an apply/undo bookkeeping bug).
  Without a network attached it explains how to attach one.
- **`debug search [depth]`** runs one single-threaded search and prints
  one line per completed deepening iteration — useful to watch aspiration
  windows, time management and how the PV stabilizes.
- **`bench search [depth]`** is the regression bench: 4 fixed positions
  (`src/bench_data.rs` — start position + 3 seeded-walk midgames), depths
  `1..=D`, untimed, single-threaded. Node counts and the final
  `bench hash <hex>` are bit-reproducible across machines; if that line
  changes between two commits, the engine's *behaviour* changed. Never
  edit the bench positions in place.

## ELO measurement

Strength changes are measured with the built-in `elo` module
(`taikyokushogi::elo`), which the CLI exposes through the `match` and `sprt`
commands:

- **`match 20 2 3`** — plays 20 games alternating colors between a depth-2 and
  a depth-3 configuration, then prints W/D/L and the Elo difference with a
  95% confidence interval.
- **`sprt 0 10 100 2 3`** — runs a sequential probability ratio test with
  H0 ≤ 0 Elo vs H1 ≥ 10 Elo (α = β = 0.05), stopping as soon as the result
  is statistically decided.

Both commands accept keyword overrides **after** the positional arguments
(case-insensitive; a bad value warns and keeps the previous value):
`movetime MS` (per-move budget, applied to both sides), `maxplies N` (ply
cap; games hitting it count as draws), `threadsA N` / `threadsB N` (Lazy-SMP
workers per side, 1..=64), `open K` (random opening plies) and `seed S`
(opening randomizer seed). That makes the SMP-vs-single-thread A/B test a
one-liner:

```bash
# Same depth (3) and same time budget (300 ms/move) for both sides:
# side A with 4 Lazy-SMP workers vs side B single-threaded
./target/release/taikyokushogi-cli match 20 3 3 movetime 300 threadsA 4 threadsB 1

# With 8 seeded random opening plies (seed 42) so games diverge
./target/release/taikyokushogi-cli match 20 3 3 open 8 seed 42
```

Because `threads` only changes how the same search is explored, Elo from
this comparison is the Lazy-SMP gain for the given depth/time budget on your
machine — run ≥ 100 games (or `sprt`) before quoting a number. With the
default `open 0` every game replays the *same* deterministic opening, which
is fine for regression checks but underestimates real strength variance;
prefer `open 6..=10` for measurement.

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

The engine's **current absolute strength** — a self-referential Elo depth
ladder anchored at depth 1 = 0 — is published in
[STRENGTH.md](STRENGTH.md) and can be refreshed with `make strength` or
`cargo run --release --example elo_report`.

## Other command-line tools

```bash
# Search the initial position at depth 5 with a 10 s time limit
cargo run --release --bin debug-cli 5 10000

# Generate 100 self-play games (depth 3, no ply cap, 4 workers, 200 ms per move)
cargo run --release --bin selfplay 100 3 0 4 200
# -> writes training_data/samples_*.bin and training_data/games.db
```

`debug-cli` takes positional args `<depth> <time_ms> [--debug|-d]`;
`selfplay` takes `<num_games> <depth> <max_moves> <num_workers> <time_limit_ms>`
(defaults: 10 / 2 / 0 = unlimited / 4 / 500).

The NNUE tool is `cargo run --release --bin nnue --
<classes|gen-classes|info|verify|random-net|dump-positions|explain|bench>` —
see [NNUE.md](NNUE.md).
