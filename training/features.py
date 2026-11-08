"""Feature indices of the NNUE "S" architecture (vectorised).

index = plane * 1296 + square, per perspective (0 = Black, 1 = White):
  plane  = class          for the perspective's own pieces
         = C + class      for enemy pieces
  square = sq             for Black
         = 1295 - sq      for White (180-degree rotation)

No king anchor, no hashing. Must equal `Network::feature_index` in Rust; the
parity gate (parity_gate.py) checks this on thousands of positions.
"""

import numpy as np

from arch import NSQ


def feature_indices(arch, sq, pt, color):
    """sq, pt, color: int arrays of the pieces of ONE position.

    Returns (idx_black, idx_white) as int64 arrays.
    """
    sq = np.asarray(sq).astype(np.int64)
    pt = np.asarray(pt).astype(np.int64)
    color = np.asarray(color).astype(np.int64)
    cls = np.asarray(arch.class_map, dtype=np.int64)[pt]
    out = []
    for p in (0, 1):
        plane = np.where(color == p, cls, arch.num_classes + cls)
        vsq = sq if p == 0 else (NSQ - 1 - sq)
        out.append(plane * NSQ + vsq)
    return out[0], out[1]
