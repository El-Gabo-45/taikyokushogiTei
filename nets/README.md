# Networks

Network files (`*.tkn`, and optional zstd-compressed `*.tkn.zst`) are **not
tracked in Git**: the experimental one is 38.1 MB and every clone would pay
for it. They are distributed as release assets instead.

## Getting the network

The [`nnue-v1`](https://github.com/El-Gabo-45/taikyokushogiTei/releases/tag/nnue-v1)
release publishes the compressed transport asset `experimental-v1.tkn.zst`
(10.2 MB). The engine loads it directly; no manual decompression is needed.
Download it and verify the checksum:

```bash
gh release download nnue-v1 --pattern 'experimental-v1.tkn.zst*' --dir nets
cd nets && sha256sum -c experimental-v1.tkn.zst.sha256
```

Expected SHA-256:

```text
1c5357c93c44de9b3e4a0986684d36699e8d08e81f4e01537825c5b47c734928  experimental-v1.tkn.zst
```

Without the file the engine still works with the hand-crafted evaluator. To
use this network:

```bash
# CLI
./target/release/taikyokushogi-cli --net nets/experimental-v1.tkn.zst

# Web server + GUI
TAIKYOKU_NET=nets/experimental-v1.tkn.zst \
  cargo run --release --features server --bin taikyokushogi-server
```

The engine panel in the web UI shows `Backend: nnue` when the server loaded
the file. `TAIKYOKU_NET=none` (or unset) selects the hand-crafted evaluator.

## Compressed transport asset

`Network::load` accepts a zstd-compressed network transparently: `--net`,
`setoption net`, `TAIKYOKU_NET`, `nnue info` and `nnue verify` all work with
either `.tkn` or `.tkn.zst`. The engine detects the zstd frame magic,
decompresses it in memory, and then runs the same TKN1 validation, so
inference is identical and no manual decompression step is needed.

The `nnue-v1` release ships the compressed asset to keep downloads small. To
regenerate it from the canonical raw file:

```bash
# from the repository root (recommended; also writes the .sha256)
make nnue-pack NET=nets/experimental-v1.tkn

# equivalent manual commands:
zstd -19 nets/experimental-v1.tkn -o nets/experimental-v1.tkn.zst
sha256sum nets/experimental-v1.tkn.zst > nets/experimental-v1.tkn.zst.sha256
```

The measured size is 38.1 MB raw → 10.2 MB with `zstd -19`. The raw `.tkn`
remains the canonical format and is still accepted; it is only needed when you
want to inspect, retrain or repackage the uncompressed data. Compression is a
distribution choice only: it does not change the network, the inference result
or the TKN1 version. Do not rename a `.tkn`/`.tkn.zst` to `.nnue`; that
extension belongs to Stockfish's incompatible format.

## License

The NNUE network files and the data they encode (`experimental-v1.tkn`,
`experimental-v1.tkn.zst` and future network releases) are licensed under the
[Open Database License (ODbL) 1.0](LICENSE-ODbL.md) (SPDX: `ODbL-1.0`).

The engine source code is a separate work and remains under the
[GNU General Public License v3.0 or later](../LICENSE) (SPDX:
`GPL-3.0-or-later`). The ODbL does not apply to the engine code, and the GPL
does not apply to the network data.

Attribution notice for produced works, as described by ODbL Section 4.3:

> Contains information from the Taikyoku Shogi NNUE network database, which is
> made available here under the Open Database License (ODbL).

If you redistribute a network or a derivative database, keep the license and
attribution notices and comply with the ODbL share-alike terms. The canonical
license text is also available at
<https://opendatacommons.org/licenses/odbl/1-0/>.

## Why not commit it?

- Git stores every version forever; a 38 MB binary bloats clones and CI.
- The network is experimental and **has no demonstrated strength gain**, so
  it should not be the default binary payload.
- GitHub's hard limit is 100 MB per file and it warns above 50 MB; staying
  away from that limit keeps the repository healthy.
- Stockfish-style builds download a pinned network and verify it instead of
  committing the file. The `*.sha256` next to this README is our pinned check.

If you prefer versioned binaries, use **Git LFS**:

```bash
git lfs install
git lfs track "nets/*.tkn"
git add .gitattributes nets/experimental-v1.tkn
```

That still requires every clone to install Git LFS and counts against the LFS
quota, so releases are simpler for a single experimental network.

`experimental-v1.tkn` — **experimental, no demonstrated strength gain.**
It exists so the whole pipeline (load → search → parity) can be tried without
training anything first.

| | |
| --- | --- |
| Architecture | NNUE "S", TKN1 v2, 27 classes, 8 buckets (hash `7152ece7…eea8`, see `nnue info`) |
| Data | 300 self-play games, depth 2, 100 ms, 8 random opening plies, seed 100 (136 902 moves) |
| Training samples | 104 277 after dropping mate-like scores, sigmoid scale SC = 39 894 |
| Training | `train.py --epochs 5 --batch 512 --lr 2e-3` (λ = 0.7), 6.3 min on one CPU core |
| Validation loss | 0.0105 (epoch 1) → 0.0077 (epoch 5); train ≈ val; no saturated weights |
| As predictor | MSE 0.00771 vs 0.00812 for the hand-crafted eval on the same targets |
| Parity | Rust == Python integer reference on 4000 positions (normal and thinned) |
| Strength | depth 2, 200 ms, 8 opening plies: net as B 0-4-26 vs classic (30 games), net as A 2-6-52 (60 games). Pooled by engine: 6 wins, 6 losses, 78 draws. **No measurable difference**; the side that played as B won in both runs, so a small A/B position bias also exists in the match harness. |

The output-bucket thresholds in `rules/nnue_arch.json` were calibrated on that
data, so this network only loads with that file. Games averaged 456 plies and
rarely got below 480 pieces, so bucket 0 covers every position under 482
pieces: endgames are under-represented. For a real strength test generate much
more and deeper data, recalibrate (`make nnue-buckets`), retrain and run a
long SPRT (`match_race … sprt=0:10`).
