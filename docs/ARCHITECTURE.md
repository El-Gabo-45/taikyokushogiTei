# Architecture — quick overview

Module responsibilities:

- `src/board.rs`: board representation, rules, `apply`/`undo`.
- `src/movegen.rs`: legal move generation.
- `src/search/`: search algorithms (PVS, qsearch, ordering, transposition table).
- `src/eval/`: evaluators (hand-crafted and `nnue.rs` for NNUE evaluation).
- `src/selfplay.rs` + `src/selfplay_main.rs`: self-play game generation for training.
- `training/`: PyTorch pipeline to train and export `.nnue` files.
- `web/`: TypeScript + Vite frontend and REST client (`web/src/api/client.ts`).

Typical NNUE workflow:

1. Run `selfplay` to produce samples in `training_data/`.
2. Convert/extract features with `training/dataset.py`.
3. Train the network using `training/train.py`.
4. Export weights using `training/export.py` to a `.nnue` file.
5. Load the `.nnue` at runtime via the `TAIKYOKU_NNUE_PATH` environment variable.

Integration notes:

- NNUE features use a HalfKP-style encoding; the accumulator is maintained incrementally in `Board::apply_move`.
- Main binaries: `taikyokushogi-server`, `debug-cli`, and `selfplay`.
