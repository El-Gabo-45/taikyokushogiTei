#!/usr/bin/env bash
# Install a PyTorch build with GPU support and verify it is usable.
#
#   bash scripts/setup_gpu.sh                       # NVIDIA: default CUDA index
#   bash scripts/setup_gpu.sh https://.../cu121     # override the CUDA index
#   make nnue-gpu-setup                             # same thing via make
#
# On Apple Silicon (arm64 macOS) the standard wheel already includes MPS, so
# no special index is needed. The CPU-only lockfile in training/ is kept for
# CI/reproducibility; this script is for the machine that actually trains.
set -euo pipefail

REPO="$(git rev-parse --show-toplevel 2>/dev/null)" || REPO="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO"

PY="${PYTHON:-python3}"
command -v "$PY" >/dev/null 2>&1 || { echo "error: $PY not found" >&2; exit 1; }

if command -v nvidia-smi >/dev/null 2>&1; then
    INDEX="${1:-https://download.pytorch.org/whl/cu124}"
    echo "NVIDIA GPU detected ($(nvidia-smi --query-gpu=name --format=csv,noheader | head -1))"
    echo "Installing torch from $INDEX"
    "$PY" -m pip install --upgrade torch --index-url "$INDEX"
    "$PY" - <<'PY'
import torch
assert torch.cuda.is_available(), "torch.cuda is still unavailable after install"
print(f"CUDA OK: torch {torch.__version__}, device {torch.cuda.get_device_name(0)}")
PY
elif [ "$(uname -s)" = "Darwin" ] && [ "$(uname -m)" = "arm64" ]; then
    echo "Apple Silicon detected; installing standard torch (MPS included)"
    "$PY" -m pip install --upgrade torch
    "$PY" - <<'PY'
import torch
assert torch.backends.mps.is_available(), "MPS unavailable after install"
print(f"MPS OK: torch {torch.__version__}")
PY
else
    echo "No NVIDIA GPU or Apple Silicon detected." >&2
    echo "Training will use CPU. To force it: make nnue-train DEVICE=cpu" >&2
    exit 1
fi

echo "Now run: make nnue-device && make nnue-train DEVICE=auto"
