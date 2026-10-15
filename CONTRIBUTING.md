# Contributing

Thank you for contributing. Quick guidelines and development steps.

Development requirements

- Install Rust via `rustup` and use the stable toolchain.
- Style: match the surrounding code. The historical codebase is **not**
  rustfmt-clean, so do *not* run `cargo fmt` across the tree (it would
  reformat hundreds of unrelated lines); format only the code you touch.

Before submitting a PR

- Run `cargo build --release` and `cargo test` (the whole suite must stay
  green: unit + `src/correctness_tests.rs` + doc-tests).
- Run `cargo build --all-targets` and keep it warning-free.
- Update documentation if your change affects the public API or behavior
  (README.md, docs/ARCHITECTURE.md, examples/README.md, training/README.md
  and the CLI `--help` / `help` texts must stay in sync with what the
  engine actually does).

Commit & PR checklist

- Clear commit messages; reference related issues when applicable.
- PR description explaining what changed and why.
- Include tests or examples demonstrating correctness when relevant.

NNUE / datasets

- If you modify the `TrainingSample` layout or feature extraction, regenerate the `training_data` and add notes in `training/README.md`.

Contact

- Open an issue on the repository for longer discussions.
