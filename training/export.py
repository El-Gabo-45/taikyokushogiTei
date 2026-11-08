"""Quantise a trained TKNet and write it as a TKN1 v2 file.

Scales (float -> int), matching the engine's integer inference:
  ft weights / ft bias / fc0, fc1 weights and biases : x64   (i16 / i32 / i8)
  psqt, fc2 biases                                   : x16   (i32)
  fc2 weights                                        : x16   (i8)
Biases of the dense layers use the SAME scale as the products (x64), because
inputs of those layers are plain integers in 0..127. Saturation is reported
per tensor; exporting a saturated network fails unless --allow-clip is given.
"""

import numpy as np

import tkn
from arch import FT_SCALE, FV_SCALE


def _q(x, scale, lo, hi, name, report):
    r = np.round(x.detach().cpu().numpy().astype(np.float64) * scale)
    sat = int(((r < lo) | (r > hi)).sum())
    report[name] = sat
    return np.clip(r, lo, hi)


def quantize(model):
    rep = {}
    I16, I32, I8 = (-32768, 32767), (-(2 ** 31), 2 ** 31 - 1), (-127, 127)
    net = tkn.Net(
        ft=_q(model.ft.weight, FT_SCALE, *I16, "ft", rep).astype("<i2"),
        ft_bias=_q(model.ft_bias, FT_SCALE, *I32, "ft_bias", rep).astype("<i4"),
        psqt=_q(model.psqt.weight, FV_SCALE, *I32, "psqt", rep).astype("<i4"),
        fc0_w=_q(model.fc0_w, FT_SCALE, *I8, "fc0_w", rep).astype("i1"),
        fc0_b=_q(model.fc0_b, FT_SCALE, *I32, "fc0_b", rep).astype("<i4"),
        fc1_w=_q(model.fc1_w, FT_SCALE, *I8, "fc1_w", rep).astype("i1"),
        fc1_b=_q(model.fc1_b, FT_SCALE, *I32, "fc1_b", rep).astype("<i4"),
        fc2_w=_q(model.fc2_w, FV_SCALE, *I8, "fc2_w", rep).astype("i1"),
        fc2_b=_q(model.fc2_b, FV_SCALE, *I32, "fc2_b", rep).astype("<i4"),
    )
    return net, rep


def export(model, arch, path, allow_clip=False):
    net, rep = quantize(model)
    print("saturated weights per tensor:", rep)
    if any(rep.values()) and not allow_clip:
        raise SystemExit("export aborted: weights saturate (use --allow-clip to override)")
    tkn.write(path, arch, net)
    print(f"wrote {path}")
    return net
