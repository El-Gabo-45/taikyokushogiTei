# Contributing

Thank you for contributing. Quick guidelines and development steps.

Development requirements

- Install Rust via `rustup` and use the stable toolchain.
- Run `cargo fmt` to format your code.

Before submitting a PR

- Run `cargo build --release` and `cargo test`.
- Run `cargo fmt` and fix formatting issues.
- Update documentation if your change affects the public API or behavior.

Commit & PR checklist

- Clear commit messages; reference related issues when applicable.
- PR description explaining what changed and why.
- Include tests or examples demonstrating correctness when relevant.

NNUE / datasets

- If you modify the `TrainingSample` layout or feature extraction, regenerate the `training_data` and add notes in `training/README.md`.

Contact

- Open an issue on the repository for longer discussions.
