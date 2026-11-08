"""Architecture of the NNUE "S" network, read from rules/nnue_arch.json.

`rules/nnue_arch.json` is the single source of truth shared with the Rust
engine (it is embedded in the binary with include_str!). Nothing here may
hard-code a class map, a threshold or a scale.
"""

import hashlib
import json
from pathlib import Path

ARCH_PATH = Path(__file__).resolve().parent.parent / "rules" / "nnue_arch.json"

NSQ = 1296
L1 = 256
H1 = 16
H2 = 32
OUT_BUCKETS = 8
FT_SHIFT = 6          # accumulator -> activation shift
FV_SCALE = 16         # output / PSQT fixed-point scale
FT_SCALE = 64         # float -> int scale of the feature transformer and dense layers
Q8_RANGE = 127


class Arch:
    def __init__(self, path=ARCH_PATH):
        spec = json.loads(Path(path).read_text())
        self.spec = spec
        assert spec["format"] == "TKN1" and spec["version"] == 2
        assert spec["num_squares"] == NSQ and spec["l1"] == L1
        assert spec["out_buckets"] == OUT_BUCKETS
        assert spec["ft_shift"] == FT_SHIFT and spec["fv_scale"] == FV_SCALE
        self.class_map = list(spec["class_map"])
        self.num_classes = spec["num_classes"]
        assert max(self.class_map[1:]) + 1 == self.num_classes
        self.thresholds = list(spec["thresholds_pieces"])
        assert len(self.thresholds) == OUT_BUCKETS - 1
        assert all(a < b for a, b in zip(self.thresholds, self.thresholds[1:]))
        self.class_values = list(spec.get("class_values", []))
        self.eval_limit = spec["eval_limit"]
        self.planes = 2 * self.num_classes
        self.dims = self.planes * NSQ

    def descriptor(self) -> str:
        cmap = hashlib.sha256(bytes(self.class_map)).digest()[:8].hex()
        th = ", ".join(str(t) for t in self.thresholds)
        return (
            f"taikyoku-nnue-S;kb=1;planes={self.planes};sq={NSQ};L1={L1};ob={OUT_BUCKETS};"
            f"acc=i32;ftshift={FT_SHIFT};act=clip0_127;pairwise=shr7;stack=16-32-1;"
            f"classes={self.num_classes};cmap={cmap};buckets=[{th}]"
        )

    def arch_hash(self) -> bytes:
        return hashlib.sha256(self.descriptor().encode()).digest()

    def bucket(self, piece_count: int) -> int:
        for k, t in enumerate(self.thresholds):
            if piece_count < t:
                return k
        return OUT_BUCKETS - 1
