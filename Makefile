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
