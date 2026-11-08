"""Reader / writer of the TKN1 v2 network file (see src/eval/nnue/format.rs).

Layout (little-endian, nothing after the last field):
  "TKN1" | u16 version=2 | 32B arch_hash
  | u32 kingbuckets=1 | u32 planes | u32 L1 | u32 out_buckets | u32 npiece
  | u8 class_map[npiece] | u16 thresholds[out_buckets-1]
  | i16 ft[dims*L1] | i32 ft_bias[L1] | i32 psqt[dims*out_buckets]
  | i8 fc0[ob*L1*16] | i32 b0[ob*16] | i8 fc1[ob*16*32] | i32 b1[ob*32]
  | i8 fc2[ob*32] | i32 b2[ob]
Matrices are stored [input][output] per bucket.
"""

import struct
from dataclasses import dataclass

import numpy as np

from arch import H1, H2, L1, OUT_BUCKETS

MAGIC = b"TKN1"
VERSION = 2


@dataclass
class Net:
    ft: np.ndarray        # (dims, L1) int16
    ft_bias: np.ndarray   # (L1,) int32
    psqt: np.ndarray      # (dims, OUT_BUCKETS) int32
    fc0_w: np.ndarray     # (OB, L1, H1) int8
    fc0_b: np.ndarray     # (OB, H1) int32
    fc1_w: np.ndarray     # (OB, H1, H2) int8
    fc1_b: np.ndarray     # (OB, H2) int32
    fc2_w: np.ndarray     # (OB, H2) int8
    fc2_b: np.ndarray     # (OB,) int32


def _shapes(arch):
    return {
        "ft": ((arch.dims, L1), "<i2"),
        "ft_bias": ((L1,), "<i4"),
        "psqt": ((arch.dims, OUT_BUCKETS), "<i4"),
        "fc0_w": ((OUT_BUCKETS, L1, H1), "i1"),
        "fc0_b": ((OUT_BUCKETS, H1), "<i4"),
        "fc1_w": ((OUT_BUCKETS, H1, H2), "i1"),
        "fc1_b": ((OUT_BUCKETS, H2), "<i4"),
        "fc2_w": ((OUT_BUCKETS, H2), "i1"),
        "fc2_b": ((OUT_BUCKETS,), "<i4"),
    }


def to_bytes(arch, net: Net) -> bytes:
    out = bytearray()
    out += MAGIC + struct.pack("<H", VERSION) + arch.arch_hash()
    out += struct.pack("<5I", 1, arch.planes, L1, OUT_BUCKETS, len(arch.class_map))
    out += bytes(arch.class_map)
    out += struct.pack(f"<{len(arch.thresholds)}H", *arch.thresholds)
    for name, (shape, dt) in _shapes(arch).items():
        a = getattr(net, name)
        assert a.shape == shape, f"{name}: {a.shape} != {shape}"
        out += np.ascontiguousarray(a).astype(dt, copy=False).tobytes()
    return bytes(out)


def write(path, arch, net: Net):
    with open(path, "wb") as f:
        f.write(to_bytes(arch, net))


def read(path, arch) -> Net:
    """Read and validate a network against `arch`; raises ValueError."""
    with open(path, "rb") as f:
        buf = f.read()
    pos = 0

    def take(n):
        nonlocal pos
        if pos + n > len(buf):
            raise ValueError("file is truncated")
        b = buf[pos:pos + n]
        pos += n
        return b

    if take(4) != MAGIC:
        raise ValueError("bad magic (not TKN1)")
    (ver,) = struct.unpack("<H", take(2))
    if ver != VERSION:
        raise ValueError(f"unsupported version {ver}")
    if take(32) != arch.arch_hash():
        raise ValueError("architecture hash differs from rules/nnue_arch.json")
    kb, planes, l1, ob, npiece = struct.unpack("<5I", take(20))
    if (kb, planes, l1, ob, npiece) != (1, arch.planes, L1, OUT_BUCKETS, len(arch.class_map)):
        raise ValueError("dimensions differ from rules/nnue_arch.json")
    if list(take(npiece)) != arch.class_map:
        raise ValueError("class map differs")
    th = list(struct.unpack(f"<{OUT_BUCKETS - 1}H", take(2 * (OUT_BUCKETS - 1))))
    if th != arch.thresholds:
        raise ValueError("thresholds differ")
    fields = {}
    for name, (shape, dt) in _shapes(arch).items():
        n = int(np.prod(shape)) * np.dtype(dt).itemsize
        fields[name] = np.frombuffer(take(n), dtype=dt).reshape(shape).copy()
    if pos != len(buf):
        raise ValueError(f"{len(buf) - pos} trailing bytes")
    return Net(**fields)
