# Web GUI — `taikyokushogi-server`

The server (REST API + static web frontend, port 8000) lives behind the
`server` feature:

```bash
# Start the HTTP server: REST API + static web frontend
cargo run --release --features server --bin taikyokushogi-server
# Then open http://localhost:8000 in your browser
```

The compiled frontend (`web/dist/`) is committed in the repository, so you
can run the server immediately after building — no npm required. Rebuild it
only after modifying `web/src/`:

```bash
cd web && npm install && npm run build && cd ..
```

> **Note:** `web/dist/` is tracked in git (`!web/dist/` in `.gitignore`). If
> you clone and the directory is missing, run `cd web && npm install && npm
> run build` before starting the server.

## Features

- **Game modes**: Human vs Random, Human vs AI, Random vs Random, AI vs AI
- Play as **Black or White**; pick the AI strength (Random, D1–D3)
- Click a piece to see its legal moves highlighted on the board
  (green = move, red = capture); a modal disambiguates promotion/igui moves
- **Piece Inspector**: click any piece to see its name, value, promotion,
  Betza notation, slides/jumps, area and range-capture capabilities
- **Engine panel**: version, evaluation backend, hash size, piece database
  size, and telemetry (depth, score, nodes, time) of the last search
- **Analysis mode** (`Analyze`): run a search without moving, then inspect
  the best move and score
- **Material balance chart** with the full score history
- **Game export**: download the played game as CLI-compatible **JSON**
  (`tsfen`, `moves`, `side`, `move_number`; load it in the CLI with
  `load <file.json>`) or as a **PGN** file for reading/sharing. Taikyoku's
  36×36 board is outside the PGN standard, so the PGN uses coordinate moves
  and a `MoveFormat` tag.
- **TSFEN panel**: export, copy, download and load positions
- **Flip board**, **zoom slider**, kanji/English glyphs, last-move and hover
  highlights, responsive layout, keyboard shortcuts (N/U/A/K/F/±/Space)
- **New Game**, **Undo** (multi-ply via API), **Auto** play and **AI Move**

## REST API

The server exposes a JSON REST API — consumed by the frontend in
`web/src/api/client.ts`. The handlers live in `src/server/handlers.rs`
(the module is part of the library so `tests/server_api.rs` can exercise the
whole router in-process).

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Liveness, engine version, board size, piece-type count |
| `/api/state` | GET | Full board state + TSFEN, eval score, material, version counter |
| `/api/status` | GET | Compact summary (result, ply count, piece counts, TSFEN) |
| `/api/history` | GET | Move log + score history |
| `/api/moves` | GET | Legal moves from a square (`?r=<row>&c=<col>`) |
| `/api/legal-moves` | GET | Every legal move of the side to move (with origins) |
| `/api/engine-info` | GET | Version, backend, hash size, threads, search techniques |
| `/api/evaluate` | GET | Static evaluation and material balance |
| `/api/pieces` | GET | Full piece database (301 movement classes) |
| `/api/piece-info/{abbrev}` | GET | Piece metadata incl. Betza and royal flag (404 if unknown) |
| `/api/position` | GET/POST | Read/set the position in TSFEN format |
| `/api/new-game` | POST | Reset to the initial position |
| `/api/move` | POST | Apply a human move (`from`, `to`, `promotion`, `mid_sq`) |
| `/api/ai-move` | POST | Search and apply a move; returns `search` telemetry |
| `/api/search` | POST | Analysis-only search; never mutates the board |
| `/api/undo` | POST | Undo N plies (`{"plies": 2}`, default 1) |
| `/api/save` | GET/POST | Serialize the position to TSFEN |
| `/api/load` | POST | Load a position from TSFEN |

Errors are JSON (`{"ok": false, "error": "..."}`) with real HTTP status codes:
`400` for invalid coordinates/TSFEN, `404` for unknown pieces/endpoints and
`409` when a move conflicts with the current game state.
