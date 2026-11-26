# Stockfish tooling — what this engine has

Stockfish (`official-stockfish/Stockfish`) ships a set of debugging/reporting
commands beyond the UCI protocol. This document compares them with this
engine's equivalents and records which ones were adopted.

## Comparison

| Stockfish command / feature | This engine | Status |
| --------------------------- | ----------- | ------ |
| `bench` (fixed positions, nodes/time/nps) | CLI `bench`, `bench search`; `examples/bench_fixed`, `bench_nps` | ✅ equivalent |
| `speedtest` (warmup, hashfull, thread binding, NUMA) | `examples/bench_nps` + `bench_thorough`; no warmup/NUMA report | ⚠️ partial (Taikyoku has no NUMA tuning) |
| `go perft N` | CLI `perft N`, `examples/perft`, `perft_breakdown` | ✅ equivalent |
| `d` (visualize position) | CLI `board`; **USI `d`** (added) | ✅ equivalent |
| `eval` (trace evaluation) | CLI `eval`, `debug eval`; `nnue explain` | ✅ equivalent (richer) |
| `flip` | Web UI flip button | ✅ equivalent |
| `compiler` (build provenance) | **`compiler`** in CLI/USI + `build_info()` (added) | ✅ added |
| `export_net` (save attached NNUE) | **`export_net`** in CLI + **`nnue export`** (added) | ✅ added |
| `help` / `--help` / `--license` | CLI `help`, `--help`; server/USI `--help` | ✅ equivalent |
| `info … nps` | USI `info … nps` | ✅ equivalent |
| `info … hashfull` | **`IterInfo.hashfull`** + USI `info … hashfull` (added) | ✅ added |
| `info … seldepth`, `currmove`, `currmovenumber`, `multipv` | Not exposed (single-PV search; callback only after each iteration) | ⚠️ not applicable yet |
| `UCI_ShowWDL` (win/draw/loss estimate) | No fitted WDL model for a 804-piece variant | ⚠️ not applicable |
| `go searchmoves` (root move restriction) | `research::multipv --max-root` prefilter | ⚠️ partial |
| `go movestogo` (moves remaining) | `wtime`/`winc`/`byoyomi`/`movetime` | ✅ equivalent for this UI |
| SPSA parameter tuner (`tune.cpp`) | `research::ablate` (decision-level) + Elo/SPRT matches | ⚠️ different approach |
| Syzygy tablebases | Impossible on a 36×36 board | ❌ not applicable |
| NUMA thread binding (`numa.h`) | Lock-free shared TT, 1–64 Lazy SMP workers | ⚠️ not applicable on our hosts |
| `incbin` (embed net in binary) | Net loaded at runtime | ⚠️ not needed (ODbL net is separate work) |
| Build SHA/date files | `build.rs` → `TAIKYOKU_BUILD_SHA` / `_DATE` / `TAIKYOKU_RUSTC` (added) | ✅ added |

## What was added

1. **Build provenance** (`build.rs`, `src/lib.rs`) — `build_info()` returns
   version, short git commit, commit date, `rustc --version` and target triple,
   all embedded at compile time. `compiler_info()` formats a Stockfish-style
   two-line banner. Exposed as:
   - `compiler` command in `taikyokushogi-cli` and in the USI loop
     (as `info string` lines, so GUIs ignore it),
   - `taikyokushogi-server --version` (existing flag, now backed by the same
     version string).
2. **`export_net <path>`** — CLI command saving the attached NNUE network as
   raw TKN1. Also `nnue export <input.tkn[.zst]> <output.tkn>` in the `nnue`
   tool, which doubles as a zstd decompressor. Verified by `nnue verify`.
3. **`hashfull`** — `Tt::hashfull()` samples up to 4096 hash slots and returns
   per-mille occupancy (like Stockfish's `hashfull`). It is carried in
   `search::IterInfo` and printed in USI `info depth … hashfull N …` lines, so
   a GUI/backend can see when the TT is too small for the search.
4. **USI `d`** — prints the current board through `Board::display()` as
   `info string` lines.

## Why some Stockfish tools were NOT copied

- **Syzygy / NUMA / WDL model** depend on 8×8 chess data or x86 topology
  conventions; there is no equivalent tablebase or fitted WDL model for a
  804-piece, 36×36 variant.
- **`currmove` / `seldepth`** need callbacks from inside the search iteration;
  this engine's per-iteration callback fires only after an iteration completes.
  Adding them would require touching the search hot path purely for telemetry.
- **SPSA tuning** would require a large, carefully validated parameter set;
  `research::ablate` + Elo/SPRT already answer "does this technique matter?"
  without changing the engine's playing behaviour.
