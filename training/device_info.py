#!/usr/bin/env python3
"""Report which device the NNUE trainer will use (`make nnue-device`).

Dependency-light: only torch is required, no data files are read. It mirrors
`train.py --device auto` so you can confirm a GPU is visible before a long run.
"""

try:
    import torch
except ImportError:
    raise SystemExit(
        "torch is not installed. Install the CPU lockfile for CI or run "
        "`make nnue-gpu-setup` for a GPU build."
    ) from None


def best_device() -> torch.device:
    if torch.cuda.is_available():
        return torch.device("cuda")
    mps = getattr(torch.backends, "mps", None)
    if mps and mps.is_available():
        return torch.device("mps")
    return torch.device("cpu")


def main() -> None:
    cuda = torch.cuda.is_available()
    mps = bool(getattr(torch.backends, "mps", None) and torch.backends.mps.is_available())
    device = best_device()
    print(f"torch {torch.__version__}")
    print(f"cuda available : {cuda}")
    if cuda:
        for i in range(torch.cuda.device_count()):
            print(f"  cuda:{i} -> {torch.cuda.get_device_name(i)}")
    print(f"mps  available : {mps}")
    print(f"auto device    : {device}")
    if device.type == "cpu":
        print(
            "WARNING: no GPU detected. Install CUDA torch with "
            "`make nnue-gpu-setup` (or run on Apple Silicon for MPS)."
        )


if __name__ == "__main__":
    main()
