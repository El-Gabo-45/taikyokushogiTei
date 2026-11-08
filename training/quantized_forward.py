"""Pure-integer reference forward pass (no torch).

This is the AUTHORITY the Rust engine must match bit for bit (parity_gate.py).
It mirrors `Network::forward` in src/eval/nnue/mod.rs: i32 accumulators,
clip(x) = clamp(x >> 6, 0, 127), pairwise (a*b) >> 7, three dense layers per
output bucket, and truncating division (toward zero) like Rust's `/`.
"""

import numpy as np

from arch import FT_SHIFT, FV_SCALE, L1, OUT_BUCKETS
from features import feature_indices


def tdiv(a: int, b: int) -> int:
    """Integer division truncating toward zero (Rust `/`)."""
    q = abs(int(a)) // abs(int(b))
    return q if (a >= 0) == (b > 0) else -q


def _clip(x):
    return np.clip(x >> FT_SHIFT, 0, 127)


def evaluate(arch, net, sq, pt, color, stm):
    """Evaluate one position given its pieces. Returns a dict with the engine's
    quantities: bucket, psqt term, dense output, unclamped and clamped eval."""
    sq, pt, color = (np.asarray(a, dtype=np.int64) for a in (sq, pt, color))
    bucket = arch.bucket(len(sq))
    idx = feature_indices(arch, sq, pt, color)
    acc = []
    psq = []
    for p in (0, 1):
        acc.append(net.ft_bias.astype(np.int64) + net.ft[idx[p]].astype(np.int64).sum(axis=0))
        psq.append(net.psqt[idx[p]].astype(np.int64).sum(axis=0))
    us, them = acc[stm], acc[stm ^ 1]
    pw = (_clip(us) * _clip(them)) >> 7

    s0 = net.fc0_b[bucket].astype(np.int64) + pw @ net.fc0_w[bucket].astype(np.int64)
    h0 = np.clip(s0 >> 6, 0, 127)
    s1 = net.fc1_b[bucket].astype(np.int64) + h0 @ net.fc1_w[bucket].astype(np.int64)
    h1 = np.clip(s1 >> 6, 0, 127)
    out = int(net.fc2_b[bucket]) + int(h1 @ net.fc2_w[bucket].astype(np.int64))

    ps = tdiv(int(psq[stm][bucket]) - int(psq[stm ^ 1][bucket]), 2)
    total = tdiv(ps + out, FV_SCALE)
    clamped = max(-arch.eval_limit, min(arch.eval_limit, total))
    return {"bucket": bucket, "psqt": ps, "out": out, "total": total, "eval": clamped,
            "features": [np.sort(idx[0]), np.sort(idx[1])]}
