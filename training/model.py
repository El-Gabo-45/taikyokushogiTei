"""Float model of the NNUE "S" network, trained in PyTorch.

The float forward pass reproduces the integer inference of the engine
(src/eval/nnue/mod.rs) up to rounding, so quantisation after training loses
almost nothing:

  integer (engine)                    float (here)
  acc_int  = 64 * acc_f               acc_f
  clip(x)  = clamp(x >> 6, 0, 127)    clamp(acc_f, 0, 127)
  pairwise = (a * b) >> 7             a * b / 128
  fc0/fc1: >> 6, clamp 0..127         clamp(., 0, 127), weights/biases * 64
  fc2 + psqt: engine units * 16       engine units (pawn = 500)

Everything is trained in the engine's own evaluation units.
"""

import torch
import torch.nn as nn

from arch import FT_SCALE, FV_SCALE, H1, H2, L1, NSQ, OUT_BUCKETS


class TKNet(nn.Module):
    def __init__(self, arch):
        super().__init__()
        self.arch = arch
        self.ft = nn.EmbeddingBag(arch.dims, L1, mode="sum")
        self.ft_bias = nn.Parameter(torch.full((L1,), 8.0))
        self.psqt = nn.EmbeddingBag(arch.dims, OUT_BUCKETS, mode="sum")
        self.fc0_w = nn.Parameter(torch.empty(OUT_BUCKETS, L1, H1))
        self.fc0_b = nn.Parameter(torch.zeros(OUT_BUCKETS, H1))
        self.fc1_w = nn.Parameter(torch.empty(OUT_BUCKETS, H1, H2))
        self.fc1_b = nn.Parameter(torch.zeros(OUT_BUCKETS, H2))
        self.fc2_w = nn.Parameter(torch.empty(OUT_BUCKETS, H2))
        self.fc2_b = nn.Parameter(torch.zeros(OUT_BUCKETS))
        nn.init.normal_(self.ft.weight, std=0.05)
        nn.init.zeros_(self.psqt.weight)
        nn.init.normal_(self.fc0_w, std=1.0 / L1 ** 0.5 * 4)
        nn.init.normal_(self.fc1_w, std=1.0 / H1 ** 0.5)
        nn.init.normal_(self.fc2_w, std=1.0 / H2 ** 0.5)

    def forward(self, idx0, off0, idx1, off1, stm, bucket):
        """idx*/off*: flat feature indices / offsets of both perspectives (EmbeddingBag
        layout); stm, bucket: (B,) long tensors. Returns the evaluation (B,) from the
        side to move, in engine units."""
        a0 = self.ft(idx0, off0) + self.ft_bias
        a1 = self.ft(idx1, off1) + self.ft_bias
        black_to_move = (stm == 0).unsqueeze(1)
        us = torch.where(black_to_move, a0, a1).clamp(0, 127)
        them = torch.where(black_to_move, a1, a0).clamp(0, 127)
        pw = us * them / 128.0

        h0 = (torch.einsum("bi,bio->bo", pw, self.fc0_w[bucket]) + self.fc0_b[bucket]).clamp(0, 127)
        h1 = (torch.einsum("bi,bio->bo", h0, self.fc1_w[bucket]) + self.fc1_b[bucket]).clamp(0, 127)
        out = (h1 * self.fc2_w[bucket]).sum(1) + self.fc2_b[bucket]

        p0 = self.psqt(idx0, off0).gather(1, bucket.unsqueeze(1)).squeeze(1)
        p1 = self.psqt(idx1, off1).gather(1, bucket.unsqueeze(1)).squeeze(1)
        psqt_term = torch.where(stm == 0, p0 - p1, p1 - p0) / 2.0
        return psqt_term + out

    @torch.no_grad()
    def init_material(self, class_values, cap=30000):
        """Start the PSQT as a plain material count (engine units).

        psqt_term = (P_us - P_them) / 2 with P_p the sum of the PSQT rows of the
        perspective's features. Own-piece planes get +v and enemy planes -v, so
        psqt_term equals (own material - enemy material) for the side to move,
        and training only has to learn the correction. Royal values are capped.
        """
        C = self.arch.num_classes
        assert len(class_values) == C, "rules/nnue_arch.json has no class_values; run `nnue gen-classes`"
        v = torch.tensor([min(float(x), cap) for x in class_values])
        w = torch.zeros(self.arch.planes, NSQ, OUT_BUCKETS)
        w[:C] = v.view(C, 1, 1)
        w[C:] = -v.view(C, 1, 1)
        self.psqt.weight.copy_(w.view(-1, OUT_BUCKETS))

    @torch.no_grad()
    def clamp_to_quant_range(self):
        """Keep every weight inside what its integer type can store."""
        self.ft.weight.clamp_(-32767 / FT_SCALE, 32767 / FT_SCALE)
        self.ft_bias.clamp_(-127, 127)
        for w in (self.fc0_w, self.fc1_w):
            w.clamp_(-127 / FT_SCALE, 127 / FT_SCALE)
        self.fc2_w.clamp_(-127 / FV_SCALE, 127 / FV_SCALE)
