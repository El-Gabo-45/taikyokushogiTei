# Networks

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
