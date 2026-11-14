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

.PHONY: nnue-data nnue-gates nnue-buckets nnue-train nnue-pack nnue-parity nnue-bench nnue-test

nnue-data:
	cargo run --release --bin selfplay -- $(GAMES) $(DEPTH) $(MAXMV) $(WORKERS) $(TIMEMS) --out $(DATA) --opening-plies $(OPEN)

nnue-gates:
	$(PY) training/check_label_units.py $(DATA)

nnue-buckets:
	$(PY) training/calibrate_buckets.py $(DATA) --apply
	cargo build --release --bin nnue --bin selfplay

nnue-train:
	mkdir -p $(dir $(NET))
	$(PY) training/train.py $(DATA) --out $(NET) --epochs $(EPOCHS)

nnue-pack:
	cd $(dir $(NET)) && $(ZSTD) -19 -f $(notdir $(NET)) -o $(notdir $(NET)).zst && sha256sum $(notdir $(NET)).zst > $(notdir $(NET)).zst.sha256

nnue-parity:
	cargo run --release --bin nnue -- dump-positions --net $(NET) --n 2000 --seed 11 --out /tmp/tk_pos.jsonl
	cargo run --release --bin nnue -- dump-positions --net $(NET) --n 2000 --seed 12 --thin --out /tmp/tk_pos_thin.jsonl
	$(PY) training/parity_gate.py --net $(NET) --positions /tmp/tk_pos.jsonl
	$(PY) training/parity_gate.py --net $(NET) --positions /tmp/tk_pos_thin.jsonl

nnue-bench:
	cargo run --release --bin nnue -- bench $(NET)

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
