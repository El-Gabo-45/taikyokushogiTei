# Web GUI — `taikyokushogi-server`

The server (REST API + static web frontend, port 8000) lives behind the
`server` feature:

```bash
# Start the HTTP server: REST API + static web frontend
cargo run --release --features server --bin taikyokushogi-server
# Then open http://localhost:8000 in your browser
```

> **Note:** by default the server keeps a **single shared `GameState`** —
> every visitor sees the same board, move history and side panels. It is
> designed for local play and analysis; for several people playing at the
> same time use **session mode** (below).

## Server modes: shared board vs. per-session games

| Mode | Start with | Behaviour |
| ------ | ------------ | ----------- |
| **Shared** (default) | `taikyokushogi-server` | One `GameState` per process: every visitor sees the same board and history. Meant for local play/analysis. |
| **Sessions** | `taikyokushogi-server --sessions` or `TAIKYOKU_SESSIONS=1` | Every visitor that presents a session id gets an **isolated game** (own board, move log, undo history, AI). Requests without a valid id are handled according to `TAIKYOKU_ANON` (below); by default they get a **fresh session**, never the shared board. |

In session mode the id is looked up, in order, in the `X-Session-Id` header,
the `taikyoku_session` cookie and the `?session=` query parameter. Ids are
limited to `[A-Za-z0-9_-]{1,64}`. The bundled web frontend generates one id
per browser tab and always sends the header, so two tabs = two independent
games.

### Anonymous requests (`TAIKYOKU_ANON`)

What happens to `/api/*` requests that carry no valid session id while
session mode is on:

| `TAIKYOKU_ANON` | Behaviour |
| ----------------- | ----------- |
| `auto` *(default)* | The request is answered with a **brand-new session**: the response carries `Set-Cookie: taikyoku_session=…; Path=/; SameSite=Lax` and an `X-Session-Id` header, and the client keeps using that id for its next requests. The shared board is never touched. |
| `error` | The request is rejected with `400 {ok:false, error:…}`. Valid ids keep working as usual. Useful for deployments whose clients always send an id (e.g. script-driven, cookie-less callers). |
| `shared` | Legacy v1.0 behaviour: the request is served on the **shared board**. Only opt in when something else guarantees ids (e.g. an authenticating reverse proxy), or for v1.0 compatibility. |

Invalid ids (wrong charset/length) count as "no id" and follow the same
policy. Static assets (`/`, `/assets/*`) are never affected — the frontend
loads without minting cookies.

Settings (environment variables; the server binary also accepts `--sessions`,
`--anon auto|error|shared`, `--bind ADDR` and `--net PATH` flags):

| Variable | Default | Meaning |
| ---------- | --------- | --------- |
| `TAIKYOKU_SESSIONS` | unset | `1`/`true`/`on`/`yes` enables session mode |
| `TAIKYOKU_ANON` | `auto` | Session-mode policy for id-less `/api/*` requests: `auto` mints a fresh session (cookie + `X-Session-Id`), `error` rejects with 400, `shared` serves the legacy shared board |
| `TAIKYOKU_MAX_SESSIONS` | `64` | Live session games before the least recently used one is evicted (its next request simply starts a fresh game) |
| `TAIKYOKU_CORS_ORIGIN` | *(mirror)* | Comma-separated exact origins to allow; unset = the CORS layer mirrors the request origin (permissive, fine locally) |
| `TAIKYOKU_BIND` | `127.0.0.1:8000` | Bind address |
| `TAIKYOKU_NET` | *(none)* | NNUE network file, or `none` for the hand-crafted evaluator |
| `TAIKYOKU_DIST` | `web/dist` | Frontend directory |

For a **public deployment**, run in session mode *and* set
`TAIKYOKU_CORS_ORIGIN=https://your-frontend.example` (a comma-separated
allow-list) instead of the permissive default. With the default
`TAIKYOKU_ANON=auto` anonymous callers can never reach the shared board;
`TAIKYOKU_ANON=error` additionally refuses to mint sessions for them.

The release packages ship these modes preconfigured: the `-full` package
starts in shared mode, the `-monolithic` package includes
`start-server-monolithic.sh`/`.cmd` launchers that run the server with
`--sessions`, and the `-engine` package has no web frontend (its
`taikyokushogi-server` still serves the REST API; browsers get a
"Frontend not built" page unless `TAIKYOKU_DIST` points at a frontend you
build yourself). See the README's "Prebuilt binaries".

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
| ---------- | -------- | ------------- |
| `/api/health` | GET | Liveness, engine version, board size, piece-type count |
| `/api/state` | GET | Full board state + TSFEN, eval score, material, version counter |
| `/api/status` | GET | Compact summary (result, ply count, piece counts, TSFEN) |
| `/api/history` | GET | Move log + score history |
| `/api/moves` | GET | Legal moves from a square (`?r=<row>&c=<col>`) |
| `/api/legal-moves` | GET | Every legal move of the side to move (with origins) |
| `/api/engine-info` | GET | Version, backend, hash size, threads, search techniques |
| `/api/evaluate` | GET | Static evaluation and material balance |
| `/api/pieces` | GET | Full piece database (301 piece types) |
| `/api/piece-info/:abbrev` | GET | Piece metadata incl. Betza and royal flag (404 if unknown) |
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
