# Networks

Network files (`*.tkn`, and optional zstd-compressed `*.tkn.zst`) are **not
tracked in Git**: the experimental one is 38.1 MB and every clone would pay
for it. They are distributed as release assets instead.

## Getting `experimental-v1.tkn`

Download the asset from a release (replace `<release-tag>` when one is
published) and verify its checksum:

```bash
gh release download <release-tag> --pattern experimental-v1.tkn --dir nets
cd nets && sha256sum -c experimental-v1.tkn.sha256
```

Expected SHA-256:

```text
61aa3fa0e4b2ac02b6f45281305c718a5f4a468f4ed69a7e498b355a91697a10  experimental-v1.tkn
```

Without the file the engine still works with the hand-crafted evaluator. To
use this network:

```bash
# CLI
./target/release/taikyokushogi-cli --net nets/experimental-v1.tkn

# Web server + GUI
TAIKYOKU_NET=nets/experimental-v1.tkn \
  cargo run --release --features server --bin taikyokushogi-server
```

The engine panel in the web UI shows `Backend: nnue` when the server loaded
the file. `TAIKYOKU_NET=none` (or unset) selects the hand-crafted evaluator.

## Compressed release asset (optional)

`Network::load` accepts a zstd-compressed network transparently: `--net`,
`setoption net`, `TAIKYOKU_NET`, `nnue info` and `nnue verify` all work with
either `.tkn` or `.tkn.zst`. The engine detects the zstd frame magic,
decompresses it in memory, and then runs the same TKN1 validation, so
inference is identical and no manual decompression step is needed.

If download size matters, publish a compressed asset next to the raw file:

```bash
# from the repository root (recommended; also writes the .sha256)
make nnue-pack NET=nets/experimental-v1.tkn

# equivalent manual commands:
zstd -19 nets/experimental-v1.tkn -o nets/experimental-v1.tkn.zst
sha256sum nets/experimental-v1.tkn.zst > nets/experimental-v1.tkn.zst.sha256
```

The measured size is 38.1 MB raw → 10.2 MB with `zstd -19`. Upload the
`.tkn.zst` and its `.sha256` (for example with
`gh release upload <release-tag> nets/experimental-v1.tkn.zst nets/experimental-v1.tkn.zst.sha256`);
users can pass the `.zst` directly. The raw `.tkn` remains the canonical format
and is still accepted. To download and verify the compressed asset:

```bash
gh release download <release-tag> --pattern 'experimental-v1.tkn.zst*' --dir nets
cd nets && sha256sum -c experimental-v1.tkn.zst.sha256
```

The prepared checksum is:

```text
1c5357c93c44de9b3e4a0986684d36699e8d08e81f4e01537825c5b47c734928  experimental-v1.tkn.zst
```

Compression is a distribution choice only: it does not change the network,
the inference result or the TKN1 version. Do not rename a `.tkn`/`.tkn.zst`
to `.nnue`; that extension belongs to Stockfish's incompatible format.

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
|---|---|
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
