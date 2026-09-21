# Examples & Benchmarks

This file documents the `examples/` included in the repository.

General usage:

```bash
cargo run --release --example <name> -- [args...]
```

- `bench_fixed`: fixed-depth search benchmark. Example:
  `cargo run --release --example bench_fixed -- 4 5 6`
- `bench_nps`: measures NPS and component timings (movegen, eval, apply/undo).
- `check_movecounts`: legal move count from the initial position.
- `stress_undo`: stress-test for apply/undo correctness.
- `export_piece_metadata`: writes `training/piece_metadata.json`:
  `cargo run --release --example export_piece_metadata > training/piece_metadata.json`
- `toggle_nnue`: smoke test to toggle NNUE at runtime.
- `play`: minimal command-line gameplay loop.
- `check_db`: sanity checks for the self-play SQLite database.

For each example, pass arguments after `--` which are forwarded to the binary.
If you want per-example flag documentation, I can add detailed flag lists per example.
