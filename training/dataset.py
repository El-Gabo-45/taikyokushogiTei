"""Reader for the "TKD1" training files written by `selfplay` (src/selfplay.rs).

File:   "TKD1" | u16 version=1 | u16 reserved
Record: u8 stm | i8 result | i32 score | i32 classic_eval | u16 ply
        | u32 game_id | u16 n | n * (u16 square | u16 piece_type | color << 9)

`result` and `score` are from the side to move's point of view; `score` is the
search score in the engine's internal units (pawn = 500), `classic_eval` the
hand-crafted evaluation of the same position.

`game_id` identifies the game the sample came from. Train/validation splits
must be done by game, not by individual position, to avoid leaking
correlated positions across the split.
"""

import glob
import os
import struct

import numpy as np

HEADER = b"TKD1\x01\x00\x00\x00"
_REC = struct.Struct("<BbiiHIH")


class Data:
    """All samples as flat arrays; `off[i]:off[i+1]` slices the pieces of sample i."""

    def __init__(self, sq, pt, color, off, stm, result, score, classic, ply, game_id):
        self.sq, self.pt, self.color, self.off = sq, pt, color, off
        self.stm, self.result, self.score, self.classic, self.ply = stm, result, score, classic, ply
        self.game_id = game_id

    def __len__(self):
        return len(self.stm)

    @property
    def npieces(self):
        return np.diff(self.off)

    def subset(self, mask_or_idx):
        idx = np.flatnonzero(mask_or_idx) if np.asarray(mask_or_idx).dtype == bool else np.asarray(mask_or_idx)
        lens = self.npieces[idx]
        off = np.concatenate([[0], np.cumsum(lens)])
        ent = gather_entries(self.off[idx], lens)
        return Data(self.sq[ent], self.pt[ent], self.color[ent], off,
                    self.stm[idx], self.result[idx], self.score[idx], self.classic[idx], self.ply[idx],
                    self.game_id[idx])


def gather_entries(starts, lens):
    """Indices of the pieces of the samples with the given starts/lengths."""
    total = int(lens.sum())
    cum = np.concatenate([[0], np.cumsum(lens)[:-1]])
    return np.repeat(starts - cum, lens) + np.arange(total)


def read_file(path):
    with open(path, "rb") as f:
        buf = f.read()
    if buf[:8] != HEADER:
        raise ValueError(f"{path}: not a TKD1 v1 file (regenerate it with the current selfplay)")
    pos, n = 8, 0
    recs, pieces = [], []
    while pos < len(buf):
        stm, res, score, classic, ply, game_id, npc = _REC.unpack_from(buf, pos)
        pos += _REC.size
        if pos + 4 * npc > len(buf):
            raise ValueError(f"{path}: truncated record")
        recs.append((stm, res, score, classic, ply, game_id, npc))
        pieces.append(np.frombuffer(buf, dtype="<u2", count=2 * npc, offset=pos).reshape(npc, 2))
        pos += 4 * npc
        n += 1
    return recs, pieces


def load(paths):
    """Load files or directories (samples_*.bin) into one `Data`."""
    files = []
    for p in paths:
        files += sorted(glob.glob(os.path.join(p, "samples_*.bin"))) if os.path.isdir(p) else [p]
    if not files:
        raise FileNotFoundError("no samples_*.bin files found")
    recs, pieces = [], []
    for f in files:
        r, p = read_file(f)
        recs += r
        pieces += p
    if not recs:
        raise ValueError("the files contain no samples")
    meta = np.array(recs, dtype=np.int64)
    lens = meta[:, 6]
    flat = np.concatenate(pieces)
    del pieces
    return Data(
        # Compact dtypes: a large dataset has tens of millions of pieces.
        sq=flat[:, 0].astype(np.int16),
        pt=(flat[:, 1] & 0x1FF).astype(np.int16),
        color=(flat[:, 1] >> 9).astype(np.int8),
        off=np.concatenate([[0], np.cumsum(lens)]),
        stm=meta[:, 0], result=meta[:, 1], score=meta[:, 2], classic=meta[:, 3], ply=meta[:, 4],
        game_id=meta[:, 5],
    )
