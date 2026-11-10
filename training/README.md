# NNUE training pipeline

This directory trains the NNUE **"S"** evaluator used by the Rust engine and
exports it as a **TKN1 v2** `.tkn` network that `src/eval/nnue/` loads.

The architecture is documented in full in [`docs/NNUE.md`](../docs/NNUE.md);
this file covers the training workflow.

## Credits

The NNUE architecture "S" — class-based `(plane, square)` features, 180°
rotation for the second perspective, output buckets selected by total piece
count, and the TKN1 network container — follows the design documented by
**[Belzedar94/TaikyokuShogi-Stockfish](https://github.com/Belzedar94/TaikyokuShogi-Stockfish)**
(GPL-3.0), in particular `docs/nnue-taikyoku-s.md`, `rules/nnue_classes.json`
and `rules/output_buckets.json`. Credit for the architecture belongs to that
project.

The implementation in this repository is independent: the Rust inference,
the Python trainer, the class map and the frozen `rules/nnue_arch.json`
(27 classes, 8 buckets) are this engine's own. Network files are not
interchangeable between the two projects — TKN1 is versioned and every file
carries an architecture hash.

The training code is part of the engine and is licensed under
GPL-3.0-or-later. The network files it exports (`.tkn`/`.tkn.zst`) and the
data they encode are a separate work licensed under the Open Database License
(ODbL) 1.0; see [`nets/LICENSE-ODbL.md`](../nets/LICENSE-ODbL.md).

## What is in this directory

| File | Role |
|---|---|
| `arch.py` | Loads `rules/nnue_arch.json`; architecture constants, arch hash, output buckets |
| `dataset.py` | Reader for the `TKD1` self-play files (`samples_*.bin`) |
| `features.py` | Vectorised feature indices; must match `Network::feature_index` in Rust |
| `model.py` | PyTorch model (`TKNet`); float forward pass mirrors the integer inference |
| `train.py` | Training loop, label gate, validation and final export |
| `export.py` | Quantises a trained model and writes the TKN1 v2 file |
| `tkn.py` | TKN1 v2 reader/writer (layout documented in the module docstring) |
| `quantized_forward.py` | Pure-Python integer reference used by the parity gate |
| `check_label_units.py` | Gate: search-score labels must be on the engine's scale |
| `calibrate_buckets.py` | Computes the output-bucket thresholds from real games |
| `parity_gate.py` | Rust-vs-Python integer parity on dumped positions |
| `test_pipeline.py` | Unit tests for the Python side |

The single source of truth for the architecture is
[`rules/nnue_arch.json`](../rules/nnue_arch.json). It is embedded in the Rust
binary with `include_str!` and read by `training/arch.py`; nothing here
hard-codes the class map, thresholds or scales.

## Architecture in brief

| Part | Value in this repository |
|---|---|
| Piece classes | 27 (frozen class map, 302 entries including the unused index 0) |
| Input planes | 54 (`2 × 27`), no king anchor and no feature hashing |
| Feature index | `plane × 1296 + square`; White uses `square → 1295 − square` |
| Feature transformer | `i16` weights, `i32` accumulator of width `L1 = 256` per perspective |
| PSQT | 8 values per feature, one per output bucket |
| Dense stack | `[stm ∥ other]` (512) → pairwise product → 256 → 16 → 32 → 1, `i8` weights |
| Output buckets | 8, selected by total piece count (`thresholds_pieces` in the JSON) |
| Output | `(psqt_diff / 2 + dense_out) / 16`, clamped to `eval_limit = 250000` |
| Network file | TKN1 v2, about **38.1 MB** |

`Board::apply_move` / `undo_move` maintain the accumulator incrementally; no
move forces a full refresh. There is no global NNUE switch: the network is
attached to a board with `Board::set_network(Some(Arc<Network>))`.

## Requirements

- Rust toolchain (the workspace's normal requirements).
- Python 3.9+ with `torch>=2.0` and `numpy>=1.24`:

```bash
python3 -m pip install -r training/requirements.txt
```

The trainer currently runs on **CPU**. The pilot network was trained in about
6 minutes on one core; a GPU is not required.

## Workflow

The `Makefile` wraps the whole pipeline:

```bash
make nnue-data      # self-play -> data/samples_*.bin (TKD1)
make nnue-gates     # label-unit gate
make nnue-buckets   # calibrate thresholds, rewrite rules/nnue_arch.json, rebuild
make nnue-train     # train + export nets/tk.tkn
make nnue-parity    # Rust vs Python integer parity (normal + thinned positions)
make nnue-bench     # evaluations/s and apply+undo overhead
make nnue-test      # Rust tests + Python pipeline tests
```

Useful overrides: `make nnue-data GAMES=800 DEPTH=3 WORKERS=8`,
`make nnue-train EPOCHS=20 NET=nets/run2.tkn`.

### 1. Generate self-play data

```bash
cargo build --release --bin selfplay
./target/release/selfplay 400 3 0 4 500 --out data --opening-plies 8 --seed 1
```

Positional arguments:

```text
selfplay <games> <depth> <max_moves> <workers> <time_limit_ms>
         [--out DIR] [--opening-plies N] [--seed S] [--net FILE.tkn]
```

- `max_moves = 0` means unlimited.
- The engine is deterministic, so **without** `--opening-plies` every game is
  the same. The opening plies are random legal moves and are not recorded.
- `--net` makes self-play use a network; omit it to generate data from the
  hand-crafted evaluator + search labels.
- Output: `data/samples_*.bin` plus `data/games.db`.

Each sample is a `TKD1` v1 record (see `dataset.py` for the exact byte
layout): side to move, result, search score, hand-crafted eval, ply and the
piece list `(square, piece_type | color << 9)`. Scores are from the side to
move, in engine units (pawn = 500).

### 2. Label-unit gate

```bash
python3 training/check_label_units.py data
```

It regresses the search score on the hand-crafted eval of the same positions;
the slope must stay in `[0.8, 1.25]` and the correlation must be `> 0.9`.
This catches label-scale bugs that parity checks cannot see. `train.py` runs
the same gate automatically unless `--skip-unit-gate` is passed.

### 3. Calibrate output buckets

```bash
python3 training/calibrate_buckets.py data --apply
cargo build --release --bin nnue --bin selfplay
```

The thresholds are octiles of the piece count observed in the data. They are
frozen into `rules/nnue_arch.json`, which is embedded in the binary, so the
Rust tools must be rebuilt afterwards. Calibrate on **long** games: the pilot
data rarely went below 480 pieces, so its bucket 0 covers every endgame.

### 4. Train and export

```bash
python3 training/train.py data --out nets/tk.tkn --epochs 10 --batch 512
```

`train.py` validates, trains and writes the TKN1 file in one step. Important
flags:

- `--lambda 0.7` — weight of the search score vs the game result target.
- `--scale SC` — sigmoid scale; by default chosen from the data
  (median `|score|` maps to a 75% win probability).
- `--psqt-lr 2.0`, `--bias-lr 1.0` — separate learning rates; the PSQT starts
  as a material count (`model.init_material`) and must move by thousands of
  engine units.
- `--checkpoint checkpoints/latest.pt` — save the float `state_dict` after
  every epoch.
- `--allow-clip` — allow exporting a network whose weights saturate their
  integer types. Prefer fixing the training instead.
- `--max-abs-score 100000` — drop mate-like labels (the default).

The loss is MSE between `sigmoid(pred / SC)` and
`λ · sigmoid(search_score / SC) + (1 − λ) · (result + 1) / 2`.

To export an existing checkpoint later, reconstruct the model and call
`export.export`:

```python
import torch
from arch import Arch
from export import export
from model import TKNet

arch = Arch()
model = TKNet(arch)
model.load_state_dict(torch.load("checkpoints/latest.pt", map_location="cpu"))
model.eval()
export(model, arch, "nets/tk.tkn")
```

### 5. Parity gate

```bash
cargo run --release --bin nnue -- dump-positions \
    --net nets/tk.tkn --n 2000 --seed 11 --out /tmp/tk_pos.jsonl
cargo run --release --bin nnue -- dump-positions \
    --net nets/tk.tkn --n 2000 --seed 12 --thin --out /tmp/tk_pos_thin.jsonl

python3 training/parity_gate.py --net nets/tk.tkn --positions /tmp/tk_pos.jsonl
python3 training/parity_gate.py --net nets/tk.tkn --positions /tmp/tk_pos_thin.jsonl
```

`--thin` removes random pieces so every output bucket is exercised. The gate
recomputes feature indices, the bucket and the integer evaluation in Python;
any difference is a failure. This catches Rust/Python drift in the feature
encoding or quantisation.

### 6. Use the network

```rust
use std::sync::Arc;
use taikyokushogi::{Board, Network};

let net = Arc::new(Network::load("nets/tk.tkn")?);
let mut board = Board::initial();
board.set_network(Some(net));       // evaluate()/search() now use it
let result = board.search(3, 0);
board.set_network(None);            // back to the hand-crafted evaluator
```

CLI and tools:

```bash
taikyokushogi-cli --net nets/tk.tkn
# inside the REPL: setoption net nets/tk.tkn   /   setoption net none
./target/release/selfplay 100 3 0 4 200 --net nets/tk.tkn
cargo run --release --example match_race -- 200 3 3 2000 6 \
    netA=nets/a.tkn netB=nets/b.tkn sprt=0:10
cargo run --release --bin nnue -- info nets/tk.tkn
cargo run --release --bin nnue -- verify nets/tk.tkn
# Web server + GUI (engine panel shows Backend: nnue)
TAIKYOKU_NET=nets/tk.tkn \
  cargo run --release --features server --bin taikyokushogi-server
```

## Memory budget

The current S network is much smaller than the old HalfKP design:

| Component | Size |
|---|---|
| Feature transformer weights in the file (`i16`) | ~35.8 MB |
| PSQT (`i32`, 8 buckets) | ~2.2 MB |
| Dense layers + header | < 0.1 MB |
| **TKN1 file** | **~38.1 MB** |
| Float32 training state (weights + gradients + 2 Adam moments) | ~0.3 GB |
| Pilot training run | ~6 min, one CPU core, `--batch 512 --epochs 5` |

Peak memory grows with `--batch`; the default 512 is comfortable on a normal
laptop. If you hit memory limits, lower `--batch` first.

## What is verified

| Check | Where |
|---|---|
| Incremental accumulator == full refresh over random walks, lion/igui moves, null moves and undo | `src/eval/nnue/tests.rs` |
| White perspective is a 180° rotation; feature indices unique and in range | Rust and Python tests |
| Class map and class values equal the frozen JSON | `class_map_matches_frozen_arch` |
| Loader rejects bad magic, old versions, truncated/extended files, wrong hash, class map or thresholds | `loader_rejects_*` |
| Rust and Python compute the same architecture hash | `training/test_pipeline.py` (`NNUE_BIN=…`) |
| Float model ≈ integer reference after quantisation | `training/test_pipeline.py` |
| Rust == Python integer reference on thousands of positions | `training/parity_gate.py` |
| Label-unit gate | `training/check_label_units.py` |

## Current status

- The architecture, tools and gates are implemented and tested.
- **No strength gain has been demonstrated yet.** `nets/experimental-v1.tkn`
  was trained on 104k self-play positions: it predicts search scores slightly
  better than the hand-crafted eval (MSE 0.0077 vs 0.0081), but in 90 games at
  depth 2 it went 6–6–78 against the hand-crafted engine. See
  [`nets/README.md`](../nets/README.md).
- The next step is more and deeper data, recalibrated buckets, retraining, and
  a long SPRT (`match_race … sprt=0:10`) before claiming any gain.

## Migrating from the old pipeline

The previous HalfKP-style pipeline (`.nnue` files, `FT_BUCKETS` hashing,
`TAIKYOKU_NNUE_PATH`, `set_use_nnue()`) has been removed. It is **not**
compatible with the current code:

- Networks are `.tkn` (TKN1 v2), not `.nnue`. The Rust loaders (`Network::load`,
  CLI, server and `nnue` tool) also accept a zstd-compressed `.tkn.zst`
  transparently; `training/tkn.py` and `export.py` still read/write raw TKN1.
- The evaluator is attached per board with `Board::set_network`, not through a
  global switch or environment variable.
- Old training samples and checkpoints cannot be reused; regenerate `TKD1`
  data with the current `selfplay` and retrain.
