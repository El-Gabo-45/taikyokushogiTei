# NNUE evaluator

The engine can replace its hand-crafted evaluator with a neural network
("NNUE", efficiently updatable). This is the single reference for the
architecture, the file format, the tools and the quality gates. Everything
here is checked by tests; if code and this document disagree, that is a bug.

* Source: `src/eval/nnue/` (`mod.rs`, `arch.rs`, `classes.rs`, `format.rs`, `dump.rs`)
* Architecture constants (single source of truth): `rules/nnue_arch.json`
* Tool: `cargo run --release --bin nnue -- <command>`
* Training: `training/` (PyTorch)
* Licensing: engine code GPL-3.0-or-later; network files and data ODbL-1.0
  (see [`nets/LICENSE-ODbL.md`](../nets/LICENSE-ODbL.md))

## Using a network

```rust
use std::sync::Arc;
use taikyokushogi::{Board, Network};

let net = Arc::new(Network::load("nets/tk.tkn")?);   // raw .tkn or .tkn.zst; Err on any problem
let mut board = Board::initial();
board.set_network(Some(net));                         // evaluate()/search() now use it
let result = board.search(4, 0);
board.set_network(None);                              // back to the hand-crafted evaluator
```

The network is a value, not a global. One `Arc<Network>` can serve any number
of boards and threads, and two boards in one process can use different
networks (this is how `match_race netA=… netB=…` compares two networks).

Python (build with `--features python`):

```python
import taikyokushogi as t
net = t.PyNetwork.load("nets/tk.tkn")      # raw .tkn or .tkn.zst; ValueError on any problem
board = t.PyBoard(); board.setup_initial()
board.set_network(net)                     # board.set_network(None) detaches
board.search_py(3, 0)
```

The transposition table is keyed per evaluator (`Board::tt_key`), so entries
searched with one network, another network or the hand-crafted evaluator are
never mixed, which matters when two evaluators alternate in one process
(`match_race`).

CLI: `taikyokushogi-cli --net nets/tk.tkn …` or `setoption net <path|none>`.
`--net`/`setoption net` also accept a zstd-compressed `.tkn.zst`. Self-play:
`selfplay … --net nets/tk.tkn`. Head to head:
`cargo run --release --example match_race -- 200 3 3 2000 6 netA=a.tkn netB=b.tkn sprt=0:10`.
Web server: `TAIKYOKU_NET=nets/tk.tkn cargo run --release --features server
--bin taikyokushogi-server` (`none` disables it).

## Architecture "S"

| Part | Definition |
|---|---|
| Input features | one per *(plane, square)*, per perspective. `plane = class` for the perspective's own pieces, `C + class` for enemy pieces. `index = plane * 1296 + square`. **No king anchor, no hashing.** |
| Perspective | 0 = Black, 1 = White. White sees the board rotated 180° (`square → 1295 − square`). |
| Piece classes | the 301 piece types are folded into `C` classes (27 now) by movement pattern and value tier, see below. |
| Feature transformer | `i16` weights, `i32` accumulator of width 256 per perspective, plus 8 PSQT values per feature. |
| Activation | `clip(x) = clamp(x >> 6, 0, 127)` |
| Network input | `[side to move ∥ other side]` (512), then pairwise product `(a·b) >> 7` → 256 |
| Dense layers | 256 → 16 → 32 → 1, `i8` weights, `>> 6` and clamp 0..127 between layers; one set per output bucket |
| Output bucket | 8 buckets by total piece count; 7 ascending thresholds in `rules/nnue_arch.json` |
| Output | `(psqt_diff/2 + dense_out) / 16`, clamped to `±eval_limit`, in the engine's internal units (pawn = 500) |

`psqt_diff = psqt[stm][bucket] − psqt[other][bucket]`. All divisions truncate
toward zero. The integer forward pass is `Network::forward`; its pure-Python
twin is `training/quantized_forward.py`.

### Why this design for Taikyoku

* **No king anchor.** There is no check; a game ends when the last royal is
  captured. What endangers a royal is a long-range slider, not the geometry of
  400 pieces around it, and a king-relative feature space (1296× larger) would
  need hashing that destroys piece-square identity.
* **Classes, not 301 types.** Many types are rare (a piece type that exists
  once per side is seen in a handful of positions). Pieces with the same
  movement pattern and value tier share a feature row, so every row gets data.
* **180° rotation + `[stm ∥ other]`.** Both sides learn the same function, which
  doubles the effective data.
* **Output buckets by piece count.** A game goes from 804 pieces to a handful;
  one set of dense layers per phase.
* **PSQT inside the feature transformer.** Material is learned as part of the
  network, and the trainer starts it as a plain material count so only the
  correction has to be learned (see Training).

### Piece classes

`classes.rs` is a pure function of `pieces.rs`. Class key, in priority order:

1. royal · 2. range-capturing · 3. area/igui (lion-like) · 4. hook
2. non-sliders (steppers and jumpers) by value tier
3. orthogonal-only sliders by number of directions, forward-capable or not, and value tier
4. diagonal-only sliders · 9. mixed sliders by number of directions and value tier

Print the table with `nnue classes`. The result is frozen in
`rules/nnue_arch.json`; the test `class_map_matches_frozen_arch` fails if
`pieces.rs` changes it. Then run `nnue gen-classes`, which rewrites the JSON
(keeping the thresholds), and **retrain**: the class map is part of the
architecture hash.

## Single source of truth: `rules/nnue_arch.json`

Embedded in the binary (`include_str!`) and read by `training/arch.py`. It holds
the class map, the bucket thresholds, `eval_limit` and the scales. Nothing may
duplicate these values. `class_values` (mean material per class) is a training
hint and is *not* part of the hash.

The architecture descriptor is
`taikyoku-nnue-S;kb=1;planes=…;sq=1296;L1=256;ob=8;acc=i32;ftshift=6;act=clip0_127;pairwise=shr7;stack=16-32-1;classes=…;cmap=…;buckets=[…]`
and `arch_hash = SHA-256(descriptor)`. Rust and Python build the same string;
`nnue info` prints it.

## File format TKN1 v2

Little-endian, no padding, nothing after the last field:

```
"TKN1" | u16 version=2 | [u8;32] arch_hash
| u32 kingbuckets=1 | u32 planes | u32 L1 | u32 out_buckets | u32 npiece
| u8  class_map[npiece] | u16 thresholds[7]
| i16 ft[dims*256] | i32 ft_bias[256] | i32 psqt[dims*8]
| i8 fc0[8*256*16] | i32 b0[8*16] | i8 fc1[8*16*32] | i32 b1[8*32]
| i8 fc2[8*32]     | i32 b2[8]
```

Matrices are `[input][output]`, per bucket. A current network is about 38 MB.
The canonical file is raw TKN1. `Network::load` also accepts the same image
inside a zstd frame (`.tkn.zst`): it detects the frame magic, decompresses it
in memory, and then applies the exact validation below. Compression is only a
distribution/transport choice, not a different format or version.
`Network::load` validates magic, version, hash, dimensions, class map,
thresholds, exact file size, and proves the `i32` accumulators cannot overflow.
Every failure is a distinct `NnueError` with a clear message; an old or
mismatching file is rejected, never loaded silently.

## Quantisation

Float → integer scales (the trainer's float model reproduces the integer
inference up to rounding, see `training/model.py`):

| Tensor | Scale | Type |
|---|---|---|
| feature transformer weights and bias | ×64 | `i16`, `i32` |
| `fc0`, `fc1` weights and biases | ×64 | `i8`, `i32` |
| PSQT | ×16 | `i32` |
| `fc2` weights, biases | ×16 | `i8`, `i32` |

The dense-layer biases use the *same* scale as the products because the inputs
of those layers are plain integers 0..127. Export aborts if any weight
saturates (`--allow-clip` overrides).

## Incremental update

`Board::apply_move` adds and removes feature rows directly for every piece
moved, captured (also lion mid-captures, igui and range captures) or promoted.
There is no anchor, so no move ever forces a full refresh. Undo restores the
accumulator from a fixed ring of 256 snapshots: memory does not depend on the
game length, apply/undo never allocate, and an undo further back than the ring
refreshes from the board. A board without a network pays nothing.

## Workflow

```
make nnue-data      # self-play (games, depth, workers, time, --opening-plies, --net)
make nnue-gates     # unit gate on the data
make nnue-buckets   # calibrate thresholds, write rules/nnue_arch.json, then rebuild
make nnue-train     # train + export nets/tk.tkn
make nnue-parity    # Rust vs Python integer reference, normal and thinned positions
make nnue-bench
```

(or run the commands in the `Makefile` by hand). Details:

1. **Data.** `selfplay N depth max_moves workers time_ms --out DIR --opening-plies 8 --seed S [--net F]`.
   The engine is deterministic, so without `--opening-plies` every game is
   identical; the first plies are random legal moves and are not recorded.
   Use depth ≥ 2: at depth 1 the games observed during development ended in about 15 plies with a win.
   At depth ≥ 2 most of the games observed during development hit the move cap, so results are mostly draws and
   the search score carries most of the signal. Files are `TKD1` (see
   `training/dataset.py`): per position the side to move, result, search score,
   hand-crafted eval, ply and the piece list.
2. **Unit gate** (`training/check_label_units.py`). Regresses the search score
   on the hand-crafted eval of the same positions: slope in [0.8, 1.25] and
   correlation > 0.9, otherwise training refuses to start. This catches a wrong
   label scale, which the parity gate cannot.
3. **Buckets** (`training/calibrate_buckets.py`): octiles of the piece count.
   Calibrate on *long* games (the thresholds must cover the whole range of the
   game, from 804 pieces down), then freeze them. The thresholds in the
   repository were calibrated on 300 depth-2 games that rarely went below 480
   pieces, so bucket 0 holds every endgame; recalibrate with deeper data.
4. **Training** (`training/train.py`). Loss: MSE between `sigmoid(pred/SC)` and
   `λ·sigmoid(score/SC) + (1−λ)·(result+1)/2` (λ = 0.7). `SC` is chosen from the
   data so the median |score| maps to a 75 % win probability: Taikyoku pieces are
   worth 500..20000, so a fixed "N pawns" scale would saturate most labels.
   The PSQT starts as a material count (class values, royals capped at 30000)
   and has its own learning rate, because it must move by thousands of units.
5. **Parity gate** (`training/parity_gate.py`). `nnue dump-positions` writes
   positions the engine evaluated incrementally (with `--thin`, random pieces
   are removed so every output bucket is exercised); Python recomputes feature
   indices, bucket and evaluation from the piece list with the integer
   reference. Any difference fails.

## What is verified

| Check | Where |
|---|---|
| incremental accumulator == full refresh over random walks, also for every legal lion/igui move, null moves and undo beyond the ring | `src/eval/nnue/tests.rs` |
| White perspective is a 180° rotation; feature indices unique and in range | Rust and Python tests |
| class map and class values equal the frozen JSON | `class_map_matches_frozen_arch` |
| loader rejects bad magic, old versions, truncated or extended files, wrong hash, class map or thresholds, overflow | `loader_rejects_*` |
| Rust and Python compute the same architecture hash | `training/test_pipeline.py` (`NNUE_BIN=…`) |
| float model ≈ integer reference after quantisation | `training/test_pipeline.py` |
| Rust == Python integer reference on thousands of positions | `parity_gate.py` |

## Licensing

The Rust and Python source code that implements, trains and exports the
evaluator is part of the engine and is licensed under GPL-3.0-or-later (see
[`LICENSE`](../LICENSE)).

The network files themselves (`.tkn` and `.tkn.zst`) and the data they encode
are a separate work licensed under the Open Database License (ODbL) 1.0; the
full text is in [`nets/LICENSE-ODbL.md`](../nets/LICENSE-ODbL.md). Redistributing
a network or a derivative database requires keeping the ODbL notices and
complying with its share-alike terms. The attribution notice is:

> Contains information from the Taikyoku Shogi NNUE network database, which is
> made available here under the Open Database License (ODbL).

## Current status (read this)

* The architecture, tools and gates are implemented and tested.
* **No strength gain has been demonstrated.** `nets/experimental-v1.tkn` was
  trained on 104k self-play positions (see `nets/README.md`): it predicts search
  scores slightly better than the hand-crafted eval (MSE 0.0077 vs 0.0081) but
  in 90 games at depth 2 it neither won nor lost against it (6-6-78 pooled), and
  the match harness showed a small A/B position bias. Train on far more and
  deeper data and run a long SPRT (`match_race … sprt=0:10`) before claiming
  anything.
* Evaluation cost (release build, one core): about 1 µs per `evaluate()` with
  an up-to-date accumulator, 60 µs for a full refresh, and about 0.6 µs extra
  per apply+undo. The previous design documented about 3 ms per evaluation.
* Open question the tests cannot answer: whether 27 classes beat the 301
  individual types with a family factorisation. Test it with SPRT.
