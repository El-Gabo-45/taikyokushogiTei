#!/usr/bin/env python3
"""Train the NNUE "S" network from self-play data and export a TKN1 file.

  cargo run --release --bin selfplay -- 200 3 0 4 500 --out data --opening-plies 8
  python training/calibrate_buckets.py data --apply      # once, then rebuild the engine
  python training/train.py data --out net.tkn

Target: MSE between sigmoid(prediction / SC) and
        lambda * sigmoid(search_score / SC) + (1 - lambda) * (result + 1) / 2
with SC chosen from the data (median |score| -> 75 % win probability). The unit gate runs first and refuses to train on
labels whose scale does not match the engine.
"""

import argparse

import time

import numpy as np
import torch

import check_label_units
import dataset
from arch import Arch
from export import export
from features import feature_indices
from model import TKNet

def auto_scale(scores):
    """Sigmoid scale SC such that the median |score| maps to a 75 % win
    probability: SC = median / ln(3). Taikyoku piece values span 500..20000, so
    a fixed "N pawns" rule would saturate most labels; the data decides."""
    med = float(np.median(np.abs(scores)))
    return float(np.clip(med / np.log(3.0), 2000.0, 200000.0))


def make_batch(arch, d, ids, device="cpu"):
    """EmbeddingBag inputs for the samples `ids` (vectorised, no per-piece loops)."""
    starts, lens = d.off[ids], d.npieces[ids]
    ent = dataset.gather_entries(starts, lens)
    i0, i1 = feature_indices(arch, d.sq[ent], d.pt[ent], d.color[ent])
    off = np.concatenate([[0], np.cumsum(lens)[:-1]])
    bucket = np.array([arch.bucket(int(n)) for n in lens])
    t = lambda a, dt=torch.long: torch.as_tensor(a, dtype=dt, device=device)
    return t(i0), t(off), t(i1), t(off), t(d.stm[ids]), t(bucket)


def targets(d, ids, lam, sc):
    score = torch.as_tensor(d.score[ids], dtype=torch.float32)
    res = torch.as_tensor((d.result[ids] + 1) / 2.0, dtype=torch.float32)
    return lam * torch.sigmoid(score / sc) + (1 - lam) * res


def evaluate_loss(model, arch, d, ids, lam, sc, batch):
    model.eval()
    tot, n = 0.0, 0
    with torch.no_grad():
        for s in range(0, len(ids), batch):
            b = ids[s:s + batch]
            pred = model(*make_batch(arch, d, b))
            loss = ((torch.sigmoid(pred / sc) - targets(d, b, lam, sc)) ** 2).sum().item()
            tot, n = tot + loss, n + len(b)
    model.train()
    return tot / max(n, 1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("data", nargs="+", help="samples_*.bin files or directories")
    ap.add_argument("--out", default="net.tkn")
    ap.add_argument("--epochs", type=int, default=10)
    ap.add_argument("--batch", type=int, default=512)
    ap.add_argument("--lr", type=float, default=2e-3)
    ap.add_argument("--lambda", dest="lam", type=float, default=0.7, help="weight of the search score vs the game result")
    ap.add_argument("--scale", type=float, default=None, help="sigmoid scale SC (default: from the data)")
    ap.add_argument("--psqt-lr", type=float, default=2.0, help="learning rate of the PSQT (engine units, ~thousands)")
    ap.add_argument("--bias-lr", type=float, default=1.0, help="learning rate of the output bias (engine units)")
    ap.add_argument("--no-material-init", action="store_true", help="start the PSQT at zero instead of material values")
    ap.add_argument("--max-abs-score", type=int, default=100_000, help="drop mate-like scores")
    ap.add_argument("--val-fraction", type=float, default=0.05)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--skip-unit-gate", action="store_true")
    ap.add_argument("--allow-clip", action="store_true")
    ap.add_argument("--checkpoint", default=None, help="save the float model here after every epoch")
    args = ap.parse_args()

    torch.manual_seed(args.seed)
    rng = np.random.default_rng(args.seed)
    arch = Arch()

    d = dataset.load(args.data)
    print(f"loaded {len(d)} samples ({int(d.npieces.mean())} pieces on average)")
    if not args.skip_unit_gate:
        ok, slope, corr = check_label_units.check(d)
        if not ok:
            raise SystemExit("unit gate failed: labels are not on the engine's scale (see check_label_units.py)")
    keep = np.abs(d.score) < args.max_abs_score
    d = d.subset(keep)
    sc = args.scale or auto_scale(d.score)
    print(f"{len(d)} samples after dropping mate-like scores; sigmoid scale SC = {sc:.0f}")

    # ── Split by game, not by position ──────────────────────────────
    # Positions from the same game are highly correlated. Splitting by
    # individual positions would leak training data into validation.
    # Instead, group samples by game_id, shuffle games, and allocate
    # whole games to train or validation.
    game_ids = np.unique(d.game_id)
    rng.shuffle(game_ids)
    nval_games = max(1, int(len(game_ids) * args.val_fraction))
    val_game_ids = set(game_ids[:nval_games].tolist())
    train_game_ids = set(game_ids[nval_games:].tolist())

    val_mask = np.isin(d.game_id, list(val_game_ids))
    train_mask = np.isin(d.game_id, list(train_game_ids))

    val_ids = np.flatnonzero(val_mask)
    train_ids = np.flatnonzero(train_mask)
    model = TKNet(arch)
    if not args.no_material_init:
        model.init_material(arch.class_values)
    special = {id(model.psqt.weight), id(model.fc2_b)}
    opt = torch.optim.Adam([
        {"params": [p for p in model.parameters() if id(p) not in special], "lr": args.lr},
        {"params": [model.psqt.weight], "lr": args.psqt_lr},
        {"params": [model.fc2_b], "lr": args.bias_lr},
    ])
    sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, T_max=max(1, args.epochs))

    t0 = time.time()
    for ep in range(args.epochs):
        perm = rng.permutation(train_ids)
        run, cnt = 0.0, 0
        for s in range(0, len(perm), args.batch):
            b = perm[s:s + args.batch]
            pred = model(*make_batch(arch, d, b))
            loss = torch.mean((torch.sigmoid(pred / sc) - targets(d, b, args.lam, sc)) ** 2)
            opt.zero_grad()
            loss.backward()
            opt.step()
            model.clamp_to_quant_range()
            run, cnt = run + loss.item() * len(b), cnt + len(b)
        sched.step()
        val = evaluate_loss(model, arch, d, val_ids, args.lam, sc, args.batch)
        print(f"epoch {ep + 1}/{args.epochs}  train {run / cnt:.5f}  val {val:.5f}  ({time.time() - t0:.0f}s)")
        if args.checkpoint:
            torch.save(model.state_dict(), args.checkpoint)

    export(model, arch, args.out, allow_clip=args.allow_clip)


if __name__ == "__main__":
    main()
