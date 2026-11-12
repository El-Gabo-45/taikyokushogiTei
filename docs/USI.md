# USI front-end — `taikyokushogi-usi`

The engine ships with a **USI** (Universal Shogi Interface) front-end so it
can be registered in shogi GUIs (ShogiGUI, ShogiHome, Tama, …) and driven by
USI test harnesses:

```bash
cargo build --release --bin taikyokushogi-usi
./target/release/taikyokushogi-usi          # USI loop over stdin/stdout
```

A session is plain text on stdin/stdout:

```text
> usi
id name TaikyokuShogi 0.1.0
id author ...
option name Threads type spin default 1 min 1 max 64
...
usiok
> position startpos
> go depth 1
info depth 1 ... pv r33r2
bestmove r33r2
```

Register `target/release/taikyokushogi-usi` (`.exe` on Windows) as the
engine path in the GUI. Games must start from the Taikyoku initial position
(`position startpos`) or from a TSFEN previously produced by the engine —
they cannot come from the GUI's standard-shogi board.

## Taikyoku Shogi USI profile

USI was designed for 9×9 shogi and cannot describe a 36×36 board with 804
pieces, so — like other large-variant engines — the USI envelope is reused
with the engine's own position format (documented in full in the
`taikyokushogi::usi` module docs):

| Concept | This engine |
| --------- | ------------- |
| `position startpos` | the initial 804-piece position |
| `position sfen <FEN>` | the engine's **TSFEN** notation (same as `Board::to_tsfen()`); a 9×9 SFEN is rejected with `info string` |
| move tokens | engine coordinate notation: `from`+`to` with files `a`–`z` (columns 1–26), `A`–`J` (columns 27–36) and ranks `1`–`36` counted from the top, e.g. `r33r2`; a trailing `+` promotes; igui (capture without moving) is `from`==`to` (`c5c5`, `c5=c5` also accepted) |
| ambiguous moves | moves that share a from/to pair (igui victims, lion mid-captures, range captures) always resolve to the **most valuable capture** — consistently for input, `bestmove` and `info … pv` |

## Time control and `go` parameters

| `go` parameter | Support |
| ---------------- | --------- |
| `wtime` / `btime` / `winc` / `binc` / `byoyomi` | Standard clock policy: 200 ms is always reserved; with < 1 s usable the engine spends 90 % of the clock, otherwise usable/30 + 90 % of the increment; byoyomi only stretches the budget within what the main clock can still afford |
| `depth <N>` | fixed-depth search |
| `movetime <MS>` | convenience extension: fixed time per move |
| `infinite`, bare `go` | search until `stop` |
| `ponder`, `ponderhit`, `stop` | honored per the USI spec; the engine never announces a `ponder` move (reply prediction on a 500+ branching factor is a coin toss) and `stop` during pondering suppresses `bestmove` |
| `nodes <N>` | not implemented: answered with `info string` + unlimited search |
| `mate` | normal search — the variant has no checks, so "mating" is the forced capture of the last royal piece (reported as `score mate <n>`) |

## Options

| Option | Default | Meaning |
| -------- | --------- | --------- |
| `Threads` | 1 | Lazy SMP workers (1–64) |
| `Hash` | 256 | transposition table in MiB (16–4096) |
| `DepthLimit` | 0 | cap the search depth (0 = the time budget decides) |
| `NetworkFile` | `TAIKYOKU_NET` | NNUE network path (`.tkn` / `.tkn.zst`) or `none` for the hand-crafted evaluator |
| `Ponder` | false | accepted for GUI compatibility (see `go ponder` above) |

`TAIKYOKU_NET` works exactly as for the web server:

```bash
TAIKYOKU_NET=nets/experimental-v1.tkn cargo run --release --bin taikyokushogi-usi
```
