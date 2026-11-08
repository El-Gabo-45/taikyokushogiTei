"""Tests of the Python side of the NNUE pipeline.

  python -m unittest training/test_pipeline.py        (from the repo root)
  python -m unittest test_pipeline                    (from training/)

Set NNUE_BIN=target/release/nnue to also check the architecture hash against
the Rust build.
"""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent))

import dataset
import tkn
from arch import NSQ, Arch
from export import quantize
from features import feature_indices
from model import TKNet
from quantized_forward import evaluate, tdiv
from train import auto_scale, make_batch

ARCH = Arch()


def random_position(rng, n=300):
    sq = rng.choice(NSQ, size=n, replace=False)
    pt = rng.integers(1, len(ARCH.class_map), size=n)
    color = rng.integers(0, 2, size=n)
    return sq, pt, color


def as_data(positions, stm):
    """Wrap positions into a `dataset.Data` so make_batch can be reused."""
    lens = np.array([len(p[0]) for p in positions])
    z = np.zeros(len(positions), dtype=np.int64)
    return dataset.Data(
        sq=np.concatenate([p[0] for p in positions]), pt=np.concatenate([p[1] for p in positions]),
        color=np.concatenate([p[2] for p in positions]), off=np.concatenate([[0], np.cumsum(lens)]),
        stm=np.asarray(stm), result=z, score=z, classic=z, ply=z)


class FeatureTests(unittest.TestCase):
    def test_white_view_is_a_180_degree_rotation(self):
        # White's view of (square, own piece) == Black's view of the rotated square.
        for r, c in [(0, 0), (3, 17), (35, 35), (12, 30)]:
            sq, rot = r * 36 + c, (35 - r) * 36 + (35 - c)
            _, white = feature_indices(ARCH, [sq], [20], [1])
            black, _ = feature_indices(ARCH, [rot], [20], [0])
            self.assertEqual(white[0], black[0])

    def test_indices_in_range_and_unique(self):
        rng = np.random.default_rng(1)
        for _ in range(20):
            for idx in feature_indices(ARCH, *random_position(rng, 500)):
                self.assertTrue(idx.min() >= 0 and idx.max() < ARCH.dims)
                self.assertEqual(len(np.unique(idx)), len(idx))

    def test_bucket_boundaries(self):
        t = ARCH.thresholds
        self.assertEqual(ARCH.bucket(0), 0)
        self.assertEqual(ARCH.bucket(t[0] - 1), 0)
        self.assertEqual(ARCH.bucket(t[0]), 1)
        self.assertEqual(ARCH.bucket(t[-1]), 7)
        self.assertEqual(ARCH.bucket(804), 7)

    def test_tdiv_truncates_toward_zero(self):
        self.assertEqual([tdiv(-7, 2), tdiv(7, 2), tdiv(-8, 16), tdiv(15, 16)], [-3, 3, 0, 0])

    def test_arch_hash_matches_rust(self):
        exe = os.environ.get("NNUE_BIN")
        if not exe:
            self.skipTest("NNUE_BIN not set")
        with tempfile.TemporaryDirectory() as d:
            net = Path(d) / "r.tkn"
            subprocess.run([exe, "random-net", str(net)], check=True, capture_output=True)
            info = subprocess.run([exe, "info", str(net)], check=True, capture_output=True, text=True).stdout
            self.assertIn(ARCH.arch_hash().hex(), info)


class ModelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        torch.manual_seed(0)
        m = TKNet(ARCH)
        with torch.no_grad():
            # Wide, non-trivial activations so every layer is exercised.
            m.ft.weight.normal_(0, 0.4)
            m.ft_bias.fill_(30.0)
            m.psqt.weight.zero_()
            m.fc0_b.normal_(0, 5.0)
            m.fc1_b.normal_(0, 5.0)
            m.fc2_b.normal_(0, 50.0)
        m.clamp_to_quant_range()
        cls.model = m
        rng = np.random.default_rng(3)
        cls.positions = [random_position(rng, int(rng.integers(40, 700))) for _ in range(30)]
        cls.stm = rng.integers(0, 2, size=len(cls.positions))

    def test_float_model_matches_integer_reference(self):
        """Quantisation must cost almost nothing: float forward vs integer forward."""
        d = as_data(self.positions, self.stm)
        batch = make_batch(ARCH, d, np.arange(len(self.positions)))
        with torch.no_grad():
            f = self.model(*batch).numpy()
        net, rep = quantize(self.model)
        self.assertFalse(any(rep.values()), rep)
        ints = np.array([evaluate(ARCH, net, *self.positions[i], int(self.stm[i]))["total"]
                         for i in range(len(self.positions))], dtype=np.float64)
        self.assertGreater(f.std(), 20.0, "test would be trivial: outputs do not vary")
        err = np.abs(f - ints)
        self.assertLess(err.mean(), 0.05 * f.std(), f"mean error {err.mean():.2f}, output std {f.std():.2f}")
        self.assertGreater(np.corrcoef(f, ints)[0, 1], 0.999)

    def test_export_roundtrip(self):
        net, _ = quantize(self.model)
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / "n.tkn"
            tkn.write(p, ARCH, net)
            back = tkn.read(p, ARCH)
            for name in net.__dataclass_fields__:
                np.testing.assert_array_equal(getattr(net, name), getattr(back, name), err_msg=name)
            raw = p.read_bytes()
            for bad, what in [(raw[:-1], "truncated"), (raw + b"\0", "trailing"), (b"XXXX" + raw[4:], "magic")]:
                q = Path(d) / "bad.tkn"
                q.write_bytes(bad)
                with self.assertRaises(ValueError, msg=what):
                    tkn.read(q, ARCH)

    def test_material_init_equals_material_count(self):
        m = TKNet(ARCH)
        m.init_material(ARCH.class_values)
        with torch.no_grad():
            m.ft.weight.zero_()
            m.ft_bias.zero_()
            m.fc2_b.zero_()
            m.fc2_w.zero_()
        pos = self.positions[0]
        vals = np.minimum(np.array(ARCH.class_values, dtype=float), 30000)
        cls = np.array(ARCH.class_map)[pos[2 * 0 + 1]]
        for stm in (0, 1):
            d = as_data([pos], [stm])
            with torch.no_grad():
                got = m(*make_batch(ARCH, d, np.array([0]))).item()
            mine = pos[2] == stm
            want = vals[cls[mine]].sum() - vals[cls[~mine]].sum()
            self.assertAlmostEqual(got, want, delta=1.0)


class DatasetTests(unittest.TestCase):
    def test_auto_scale_maps_median_to_75_percent(self):
        s = auto_scale(np.array([-30000, 30000, 30000, -30000, 30000]))
        self.assertAlmostEqual(1 / (1 + np.exp(-30000 / s)), 0.75, places=2)


if __name__ == "__main__":
    unittest.main()
