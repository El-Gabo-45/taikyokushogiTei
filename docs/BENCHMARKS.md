# Benchmarks, performance and examples

## Performance

Measured on the development machine (Intel Xeon E3-1230 V2, 4c/8t) with the
release-mode benchmarks (`cargo run --release --example bench_nps` and
`--example bench_fixed 1 2 3 4`). All search rows are **single-threaded**
(`threads = 1`). These are single-run measurements — expect roughly
±10–20 % run-to-run variance depending on machine load.

| Metric | Value |
| --- | --- |
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
futility / static-eval pruning, internal iterative deepening, staged move
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

## Strength testing

Strength changes are measured with the built-in `elo` module — see
[ELO measurement](CLI.md#elo-measurement) for the `match` / `sprt` commands, and
`cargo run --release --example match_race -- 20 3 2 5000 4` for the
head-to-head example (depth 3 vs depth 2, 20 games).

Head-to-head matches between any two search configurations (depths, time
budgets, handcrafted vs NNUE via the eval toggle) with alternating colors
and deterministic opening randomization. For publishable Elo claims, run
>= 1000 games and compute a confidence interval or SPRT bounds on the
W/L/D counts.

## Comparing engine versions

`match_race` can only compare configurations of the *same* binary. To
measure a code change itself, race two engine executables with
`version_race` (the harness orchestrates both processes with
`position`/`go`; it never searches):

```bash
# current build vs the previous commit (builds the old ref in a worktree)
make version-race
make version-race OLD=HEAD~5

# with NNUE on both sides and an SPRT stop rule
make version-race OLD=HEAD~3 RACE_NET=nets/experimental-v1.tkn.zst RACE_ARGS="sprt=0:10"

# or point the harness at any two binaries yourself
cargo run --release --example version_race -- 40 2 2000 6 \
  engineA=target/release/taikyokushogi-cli engineB=/path/to/old/taikyokushogi-cli
```

The Makefile target builds `OLD` in a throwaway git worktree under
`target/version-race/` (removed afterwards), so both sides are genuine
binaries of the two versions. Side A plays Black on even games; results
are deterministic for a given invocation (seeded openings).

## Examples and Benchmarks

All examples run with `cargo run --release --example <name>`:

| Example | Purpose |
| --- | --- |
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

See [../examples/README.md](../examples/README.md) for more details on
examples and flags.
