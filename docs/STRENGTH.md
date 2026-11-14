# Current strength (Elo ladder)

The engine has no external rating pool to anchor to, so "current Elo" is
measured with a **self-referential depth ladder** — the same idea
Stockfish uses to sanity-check search changes, minus the fish test pool:

- Rung `d` plays depth `d` against depth `d - 1` (alternating colors,
  8 seeded random opening plies per game).
- Each rung's W/D/L gives a pairwise Elo difference with a 95 % CI
  (logistic model, `taikyokushogi::elo`).
- The ratings accumulate into one scale **anchored at depth 1 = 0**.

The ladder is fully reproducible: the same invocation replays the same
games (seeded openings, single-threaded search). The default depth the
CLI plays is 3, so "current default strength ≈ the depth-3 cumulative
number below".

**Machine:** Intel Xeon E3-1230 V2 (4c/8t), 16 GB RAM, Linux, release
build, single-threaded search. These numbers are self-referential — they
describe how much stronger one depth is than another *under these exact
conditions*; they are not comparable to external rating lists.

<!-- elo-report:begin -->
Ladder measured 2026-11-14 (1.0.2) — 40 games per rung, movetime 300 ms, 8 opening plies, 1 thread(s), ply cap 250. Evaluator: hand-crafted. Depth 1 is the 0-Elo anchor.

| Depth | W-D-L vs previous | Pair Elo | Cumulative Elo (95% CI) |
| --- | --- | --- | --- |
| 1 | — | — | 0 (anchor) |
| 2 | 12-27-1 | +98.1 | +98.1 ± 58 |
| 3 | 1-36-3 | -17.4 | +80.7 ± 67 |

**Current default depth (3) ≈ +81 Elo relative to the depth-1 baseline** under these conditions. Small samples: treat the CI as part of the number.
<!-- elo-report:end -->

## How to reproduce

```bash
make strength                                     # refresh the block above
make strength STRENGTH_GAMES=100 STRENGTH_MAXDEPTH=4   # more games / deeper
```

Or directly:

```bash
cargo run --release --example elo_report -- \
    --games 40 --max-depth 3 --movetime 300 --maxplies 250 --open 8 \
    --md docs/STRENGTH.md --json bench/elo.json
```

`bench/elo.json` gets the machine-readable version of the same report
(scripts/CI can diff it; it is a generated artifact and is not committed
— see `.gitignore`). Head-to-head matches between arbitrary
configs, SPRT tests and cross-version races are covered in
[BENCHMARKS.md](BENCHMARKS.md#strength-testing) and [CLI.md](CLI.md#elo-measurement);
the tooling itself is documented in [../examples/README.md](../examples/README.md).
