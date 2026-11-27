# Contributing

Thank you for contributing. Quick guidelines and development steps.

## Module structure

The crate is organized into layers with strict visibility rules:

| Layer | Path | Visibility | Purpose |
| --- | --- | --- | --- |
| Public API | `src/lib.rs` | `pub` | User-facing types (`Board`, `Move`, `Square`, `Piece`, `Color`, `GameResult`, `PieceInfo`, `SearchResult`) |
| Internal core | `src/board/`, `src/movegen/`, `src/types/` | `mod` (private) | Board state, move generation, core types and tables |
| Evaluation | `src/eval/` | `mod` (private) | Hand-crafted + NNUE evaluators |
| Search | `src/search/` | `mod` (private) | PVS, Lazy SMP, TT, heuristics, ordering |
| Supporting | `src/attack.rs`, `src/bitboard.rs`, `src/pieces.rs`, `src/tsfen.rs` | `mod` (private) | Bitboards, attack tables, piece definitions, TSFEN |
| Binaries | `src/main.rs`, `src/cli.rs`, `src/debug_cli.rs`, `src/selfplay_main.rs` | `[[bin]]` | Server, CLI, debug harness, self-play |
| Libraries | `src/elo.rs`, `src/selfplay.rs`, `src/python.rs` | `pub mod` / `mod` | Elo measurement, self-play engine, Python bindings |
| GPU detect | `src/gpu.rs` | `pub mod` (feature `gpu-detect`) | Enumerate Vulkan/Metal/DX12/GL adapters via `wgpu` |
| GPU training | `src/gpu_train.rs` | `pub mod` (feature `gpu-wgpu`) | Autodiff NNUE training on any GPU via `burn`/Wgpu |
| HTTP logging | `src/server/log.rs` | `pub mod` | Structured ring buffer + optional JSONL log file |

**Rule**: Internal modules are never `pub mod`. External crates can only
depend on the public API in `lib.rs`. If you add a new file, place it in
the directory matching its concern and declare it as `mod` (not `pub mod`)
in the parent module's `mod.rs`.

## Development requirements

- Install Rust via `rustup` and use the stable toolchain.
- Style: match the surrounding code. The tree is **rustfmt-clean** and CI
  enforces it with `cargo fmt --check`, so running `cargo fmt` is safe;
  format the code you touch before committing.

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
- Two training paths exist: the PyTorch pipeline (`make nnue-train DEVICE=…`, CUDA/MPS/CPU) and the GPU-agnostic Rust trainer (`make nnue-gpu-train`, burn/Wgpu → Vulkan/Metal/DX12). Both must keep exporting valid TKN1 v2 files; verify with `nnue verify <net>`.
- `rules/nnue_arch.json` is the single source of truth for the architecture; keep Rust, Python and the burner in sync with it.

Optional features

- `server`, `python`, `gpu-detect`, `gpu-{cuda,metal,wgpu,vulkan}`. CI checks the default feature set and `--features server`; code behind GPU features must be gated with `#[cfg(feature = …)]` so those builds stay green.

Contact

- Open an issue on the repository for longer discussions.
