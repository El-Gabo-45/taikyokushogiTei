# Contributing

Thank you for contributing. Quick guidelines and development steps.

## Module structure

The crate is organized into layers with strict visibility rules:

| Layer | Path | Visibility | Purpose |
|-------|------|-----------|---------|
| Public API | `src/lib.rs` | `pub` | User-facing types (`Board`, `Move`, `Square`, `Piece`, `Color`, `GameResult`, `PieceInfo`, `SearchResult`) |
| Internal core | `src/board/`, `src/movegen/`, `src/types/` | `mod` (private) | Board state, move generation, core types and tables |
| Evaluation | `src/eval/` | `mod` (private) | Hand-crafted + NNUE evaluators |
| Search | `src/search/` | `mod` (private) | PVS, Lazy SMP, TT, heuristics, ordering |
| Supporting | `src/attack.rs`, `src/bitboard.rs`, `src/pieces.rs`, `src/tsfen.rs` | `mod` (private) | Bitboards, attack tables, piece definitions, TSFEN |
| Binaries | `src/main.rs`, `src/cli.rs`, `src/debug_cli.rs`, `src/selfplay_main.rs` | `[[bin]]` | Server, CLI, debug harness, self-play |
| Libraries | `src/elo.rs`, `src/selfplay.rs`, `src/python.rs` | `pub mod` / `mod` | Elo measurement, self-play engine, Python bindings |

**Rule**: Internal modules are never `pub mod`. External crates can only
depend on the public API in `lib.rs`. If you add a new file, place it in
the directory matching its concern and declare it as `mod` (not `pub mod`)
in the parent module's `mod.rs`.

## Development requirements

- Install Rust via `rustup` and use the stable toolchain.
- Style: match the surrounding code. The historical codebase is **not**
  rustfmt-clean, so do *not* run `cargo fmt` across the tree (it would
  reformat hundreds of unrelated lines); format only the code you touch.

Before submitting a PR

- Run `cargo build --release` and `cargo test` (the whole suite must stay
  green: unit + `src/correctness_tests.rs` + doc-tests).
- Run `cargo build --all-targets` and keep it warning-free.
- Update documentation if your change affects the public API or behavior
  (README.md, docs/ARCHITECTURE.md, docs/MODULE_GUIDE.md,
  examples/README.md, training/README.md and the CLI `--help` / `help`
  texts must stay in sync with what the engine actually does).

Commit & PR checklist

- Clear commit messages; reference related issues when applicable.
- PR description explaining what changed and why.
- Include tests or examples demonstrating correctness when relevant.

NNUE / datasets

- If you modify the `TrainingSample` layout or feature extraction, regenerate the `training_data` and add notes in `training/README.md`.

Contact

- Open an issue on the repository for longer discussions.
