# NNUE workflow shortcuts. Variables can be overridden: make nnue-data GAMES=800 DEPTH=3
GAMES  ?= 400
DEPTH  ?= 3
MAXMV  ?= 0
WORKERS?= 4
TIMEMS ?= 500
OPEN   ?= 8
DATA   ?= data
NET    ?= nets/tk.tkn
EPOCHS ?= 10
PY     ?= python3
ZSTD   ?= zstd
# Training device: auto (cuda -> mps -> cpu), or force cpu/cuda/mps.
DEVICE ?= auto
# CUDA wheel index used by `make nnue-gpu-setup` (override for your driver).
GPU_INDEX ?= https://download.pytorch.org/whl/cu124

.PHONY: nnue-data nnue-gates nnue-buckets nnue-train nnue-pack nnue-parity nnue-bench nnue-test nnue-gpu-setup nnue-device nnue-gpu-train

nnue-data:
	cargo run --release --bin selfplay -- $(GAMES) $(DEPTH) $(MAXMV) $(WORKERS) $(TIMEMS) --out $(DATA) --opening-plies $(OPEN)

nnue-gates:
	$(PY) training/check_label_units.py $(DATA)

nnue-buckets:
	$(PY) training/calibrate_buckets.py $(DATA) --apply
	cargo build --release --bin nnue --bin selfplay

nnue-train:
	mkdir -p $(dir $(NET))
	$(PY) training/train.py $(DATA) --out $(NET) --epochs $(EPOCHS) --device $(DEVICE)

# GPU NNUE training through burn/Wgpu (Vulkan, Metal, DX12 — any GPU).
# Build once with: cargo build --release --features gpu-wgpu --bin nnue
GPU_EPOCHS ?= 2
GPU_BATCH  ?= 512
GPU_SAMPLES ?= 2048
nnue-gpu-train:
	cargo run --release --features gpu-wgpu --bin nnue -- gpu-train $(DATA) $(NET) $(GPU_EPOCHS) $(GPU_BATCH) $(GPU_SAMPLES)

# GPU setup / detection for the PyTorch pipeline (CUDA/MPS only).
nnue-gpu-setup:
	bash scripts/setup_gpu.sh $(GPU_INDEX)

nnue-device:
	cd training && $(PY) device_info.py

nnue-pack:
	cd $(dir $(NET)) && $(ZSTD) -19 -f $(notdir $(NET)) -o $(notdir $(NET)).zst && sha256sum $(notdir $(NET)).zst > $(notdir $(NET)).zst.sha256

nnue-parity:
	cargo run --release --bin nnue -- dump-positions --net $(NET) --n 2000 --seed 11 --out /tmp/tk_pos.jsonl
	cargo run --release --bin nnue -- dump-positions --net $(NET) --n 2000 --seed 12 --thin --out /tmp/tk_pos_thin.jsonl
	$(PY) training/parity_gate.py --net $(NET) --positions /tmp/tk_pos.jsonl
	$(PY) training/parity_gate.py --net $(NET) --positions /tmp/tk_pos_thin.jsonl

nnue-bench:
	cargo run --release --bin nnue -- bench $(NET)

# ── Research toolkit (audit-2) ──────────────────────────────────
# Position-based tools take ARGS verbatim, e.g.:
#   make nnue-multipv ARGS="--depth 3 --n 10 --net nets/tk.tkn"
#   make nnue-tree ARGS="--depth 3 --branching 3 --out tree.json"
#   make nnue-stability ARGS="--depth 6"
# Database tools point at a selfplay games.db, e.g.:
#   make nnue-critical ARGS="data/games.db --threshold 500"
#   make nnue-pieces ARGS="data/games.db --csv matrix.csv"
#   make nnue-ablate ARGS="--positions positions.txt --depth 3"
# Python dataset analyzers: make coverage ARGS="data" / make calibration ARGS="data"
.PHONY: nnue-multipv nnue-tree nnue-stability nnue-critical nnue-pieces nnue-ablate coverage calibration

nnue-multipv:
	cargo run --release --bin nnue -- multipv $(ARGS)

nnue-tree:
	cargo run --release --bin nnue -- tree $(ARGS)

nnue-stability:
	cargo run --release --bin nnue -- stability $(ARGS)

nnue-critical:
	cargo run --release --bin nnue -- critical $(ARGS)

nnue-pieces:
	cargo run --release --bin nnue -- pieces $(ARGS)

nnue-ablate:
	cargo run --release --bin nnue -- ablate $(ARGS)

coverage:
	$(PY) training/coverage.py $(ARGS)

calibration:
	$(PY) training/calibration.py $(ARGS)

nnue-test:
	cargo test --release --lib --tests
	NNUE_BIN=target/release/nnue $(PY) -m unittest training/test_pipeline.py

# ── Cross-version strength test ─────────────────────────────────
# Plays the current build against an older commit (built in a throwaway git
# worktree) and reports W/L/D, Elo CI and optional SPRT.
#   make version-race                                # vs HEAD~1
#   make version-race OLD=HEAD~5 RACE_ARGS="sprt=0:10"
#   make version-race OLD=v1.2 NET=nets/experimental-v1.tkn.zst   # with NNUE
OLD          ?= HEAD~1
RACE_GAMES   ?= 40
RACE_DEPTH   ?= 2
RACE_MOVETIME?= 2000
RACE_OPEN    ?= 6
RACE_MAXPLIES?= 300
RACE_NET     ?=
RACE_ARGS    ?=
WT           ?= target/version-race

.PHONY: version-race

version-race:
	cargo build --release --bin taikyokushogi-cli
	rm -rf $(WT) && git worktree prune
	git worktree add --detach $(WT) $(OLD)
	cd $(WT) && cargo build --release --bin taikyokushogi-cli || (git worktree remove --force $(WT); exit 1)
	cargo run --release --example version_race -- $(RACE_GAMES) $(RACE_DEPTH) $(RACE_MOVETIME) $(RACE_OPEN) $(RACE_MAXPLIES) engineA=target/release/taikyokushogi-cli engineB=$(WT)/target/release/taikyokushogi-cli $(if $(RACE_NET),netA=$(RACE_NET) netB=$(RACE_NET),) $(RACE_ARGS)
	git worktree remove --force $(WT)

# ── Elo ladder / strength report ────────────────────────────────
# Measures the engine's CURRENT strength on its self-referential depth
# ladder (depth d vs d-1, anchored at depth 1 = 0) and refreshes
# docs/STRENGTH.md (the <!-- elo-report:begin/end --> block) plus
# bench/elo.json for scripts/CI. See docs/STRENGTH.md and
# examples/elo_report.rs for the methodology.
#   make strength                              # defaults below
#   make strength STRENGTH_GAMES=100 STRENGTH_MAXDEPTH=4
STRENGTH_GAMES   ?= 40
STRENGTH_MAXDEPTH ?= 3
STRENGTH_MOVETIME?= 300
STRENGTH_MAXPLIES?= 250
STRENGTH_OPEN    ?= 8
STRENGTH_THREADS ?= 1
STRENGTH_NET     ?=

.PHONY: strength

strength:
	cargo build --release --example elo_report
	cargo run --release --example elo_report -- \
		--games $(STRENGTH_GAMES) --max-depth $(STRENGTH_MAXDEPTH) \
		--movetime $(STRENGTH_MOVETIME) --maxplies $(STRENGTH_MAXPLIES) \
		--open $(STRENGTH_OPEN) --threads $(STRENGTH_THREADS) \
		$(if $(STRENGTH_NET),--net $(STRENGTH_NET),) \
		--md docs/STRENGTH.md --json bench/elo.json

# ── Local CI mirror ─────────────────────────────────────────────
# Runs the same gates as .github/workflows/ci.yml before a push:
# toolchain sync (rustup update stable), fmt, clippy -D warnings, check,
# USI smoke, tests, web (if web/node_modules exists) and python lint
# (if ruff/pyright are installed). MSRV/PyO3/NNUE jobs stay in CI.
#   make ci-local
.PHONY: ci-local

ci-local:
	bash scripts/ci-check.sh

# ── Release ─────────────────────────────────────────────────────
# One-command release: CI gates -> version bump (Cargo + Cargo.lock + web in
# lockstep) -> commit -> annotated tag vX.Y.Z -> push. The tag push is what
# publishes the GitHub Release (release.yml). Aborts at the first failed
# guard, before touching anything; safe to re-run after a failed push.
#   make release VERSION=1.3.0
#   make release VERSION=1.3.0 MSG="Release 1.3.0: movegen + clippy fixes"
#   Preview without changes:  bash scripts/release.sh 1.3.0 --dry-run
VERSION ?=
MSG     ?=

.PHONY: release

release:
ifeq ($(VERSION),)
	$(error usage: make release VERSION=1.3.0 [MSG="..."])
endif
	bash scripts/release.sh $(VERSION) $(if $(MSG),"$(MSG)",)
