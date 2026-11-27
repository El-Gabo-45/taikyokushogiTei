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
| `TAIKYOKU_LOG_FILE` | *(none)* | Append structured JSON lines to this file (e.g. `logs/server.jsonl`); `none`/`off` disables |
| `TAIKYOKU_LOG_CAPACITY` | `256` | Entries kept in the in-memory ring served by `/api/logs` (1–4096) |
| `TAIKYOKU_SESSIONS` | unset | `1`/`true`/`on`/`yes` enables session mode |
| `TAIKYOKU_ANON` | `auto` | Session-mode policy for id-less `/api/*` requests: `auto` mints a fresh session (cookie + `X-Session-Id`), `error` rejects with 400, `shared` serves the legacy shared board |
| `TAIKYOKU_MAX_SESSIONS` | `64` | Live session games before the least recently used one is evicted (its next request simply starts a fresh game) |
| `TAIKYOKU_CORS_ORIGIN` | *(mirror)* | Comma-separated exact origins to allow (`scheme://host[:port]`); unset = the CORS layer mirrors the request origin (permissive, fine locally). A malformed value aborts startup instead of falling back to the permissive default |
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
| `/api/ready` | GET | Readiness: free search slots, `503` when saturated |
| `/api/config` | GET | Public runtime limits (body cap, search slots, session policy, log ring) |
| `/api/logs?limit=N` | GET | Most recent structured log entries (newest last) |
| `/api/state` | GET | Full board state + TSFEN, eval score, material, version counter |
| `/api/status` | GET | Compact summary (result, ply count, piece counts, TSFEN) |
| `/api/history` | GET | Move log + score history |
| `/api/moves` | GET | Legal moves from a square (`?r=<row>&c=<col>`) |
| `/api/legal-moves` | GET | Every legal move of the side to move (with origins) |
| `/api/engine-info` | GET | Version, backend, hash size, threads, search techniques |
| `/api/gpu` | GET | GPU/graphics adapters detected by `wgpu` (Vulkan/Metal/DX12/GL; needs `gpu-detect`) |
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

### Safety limits

- **Request body cap** — every request body is limited to
  `MAX_BODY_BYTES` (128 KiB, `src/server/mod.rs`), enforced by
  `tower_http::limit::RequestBodyLimitLayer`. A body larger than that is
  rejected with `413 Payload Too Large` **before** JSON parsing, so a client
  cannot exhaust server memory by streaming a huge payload to
  `/api/load` or `/api/position`. A full 36×36 TSFEN is ~15 KiB at most, so
  legitimate requests stay far below the cap. CORS is the outermost layer, so
  even 413 responses carry the configured CORS headers.
- **Session id validation** — ids are restricted to
  `[A-Za-z0-9_-]{1,64}`; anything else is treated as "no id" and follows
  `TAIKYOKU_ANON`. Session ids minted by the server are `anon-` + 128 random
  bits.
- **CORS validation** — a malformed `TAIKYOKU_CORS_ORIGIN` aborts startup
  instead of silently falling back to the permissive mirror policy.
- **Search bounds** — `depth` and `movetime` on `/api/ai-move` and `/api/search`
  are clamped to `MAX_DEPTH`/`MAX_TIME_MS`, and searches run on
  `spawn_blocking` so a long think never blocks the async executor.
- **Search concurrency cap** — at most `MAX_CONCURRENT_SEARCHES` (4) searches
  run at once across all games/sessions, guarded by a shared `tokio::Semaphore`.
  Callers wait up to 2 s for a slot; if all slots stay busy they get
  `429 Too Many Requests` instead of piling CPU-bound work onto the runtime
  (which would starve every other endpoint). This complements the session cap:
  64 visitors cannot launch 64 simultaneous 30-second searches.
- **Security headers** — every response (API, static assets, CORS preflights,
  413s, 429s, session-policy rejections) carries `X-Content-Type-Options:
  nosniff`, `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer` and a
  restrictive `Permissions-Policy` (camera/mic/geolocation/payment/USB off).
  The middleware is the outermost layer so no response can skip it.
- **Immutable-data caching** — the 301-type piece database (`/api/pieces`) is
  built once and cached in a `OnceLock`; `/api/engine-info` reuses the same
  cached list for its royal count. This avoids ~1000 string clones on every
  UI load/refresh for data that cannot change at runtime.
- **Cache policy** — `/api/*` responses carry `Cache-Control: no-store,
  no-cache, must-revalidate` (game state changes on every move), while Vite's
  content-hashed `/assets/*` files get `public, max-age=31536000, immutable`.
  CORS preflights are cached for 600 s via `Access-Control-Max-Age`.
- **`Retry-After`** — a 429 (full search queue) includes `Retry-After: 2`,
  matching the 2 s permit wait, so clients know when to retry.
- **Input validation** — `mid_sq` on `/api/move` must be a flat square index
  in `0..1296`; out-of-range values get a 400 naming the field instead of a
  misleading "Illegal move".
- **Graceful shutdown** — SIGINT (Ctrl+C) and SIGTERM trigger
  `with_graceful_shutdown`, draining in-flight requests (including searches)
  before exit instead of dropping connections. Important for containers.
- **Request correlation** — every response carries `X-Request-Id`. A valid
  client-supplied id (`[A-Za-z0-9_-]{1,64}`) is preserved; otherwise one is
  generated (`req-<64-bit hex>`). Each request is logged as
  `[http] <method> <path> -> <status> (<ms>ms, id=<id>)` so a report can be
  traced through proxy and server logs.
- **Liveness vs readiness** — `GET /api/health` answers identity data
  (liveness), while `GET /api/ready` answers `200` with the number of free
  search slots, or `503` when all slots are busy (a load balancer should route
  elsewhere). Both bypass the anonymous-request policy, so probes work even
  with `TAIKYOKU_ANON=error` and no session id.
- **Server CLI** — `taikyokushogi-server --help` / `--version` (also `-h`/`-V`).

## Logging & error capture (the easy way)

Every request is recorded automatically — no manual `log()` calls needed in
handlers:

1. `request_context` (outermost middleware) assigns a correlation id, then
   records one structured entry with method, path, status and duration.
   Levels are assigned from the status: **2xx → info**, **4xx → warn**,
   **5xx → error**. So a client error or a failed search shows up as a WARN /
   ERROR entry with the exact `X-Request-Id` the client saw.
2. Entries go to **stderr**, to a bounded **in-memory ring** (256 entries by
   default, `TAIKYOKU_LOG_CAPACITY`, max 4096), and optionally to an
   append-only **JSONL file** via `TAIKYOKU_LOG_FILE=/path/to/server.jsonl`.
3. `GET /api/logs?limit=N` (1–200, default 50) returns the newest entries as
   JSON: `{ total, returned, entries: [{ ts_ms, level, request_id, target, message }] }`.
   No session id needed (like health/ready).
4. `GET /api/config` returns the public runtime limits (body cap, search slot
   count, session cap, anon policy, log capacity) so clients/tests can read
   them instead of hard-coding.
5. The frontend has `api.fetchLogs(limit)` and `api.fetchConfig()` with Zod
   validation. The **Developer Tools panel** (press **D**) includes a
   *Diagnostics* section:
   - newest log lines with level filter (`ERROR`/`WARN`/`INFO`) and an
     auto-refresh toggle (5 s),
   - **Export logs** as a JSON file, **Copy config** as JSON to the clipboard,
   - the public limits (body cap, search slots, session policy, log ring),
   - GPU/graphics adapters from `GET /api/gpu` (Vulkan/Metal/DX12/GL — works
     for AMD/Intel cards without CUDA/ROCm), with the full report in a tooltip;
   - a **search-telemetry chart** (`<canvas>`) plotting score, nodes/s and TT
     `hashfull` over the last 60 searches, plus a one-line summary of the most
     recent one. `hashfull` travels from `Tt::hashfull()` through `IterInfo`,
     the server's `SearchStats` and the Zod-validated client response.

Background tasks (e.g. a `spawn_blocking` join failure in the search) use
`logs.error("search", …)` with request id `-`, so they land in the same ring.

Responses are also gzip-compressed when the client sends
`Accept-Encoding: gzip` (`tower_http::compression`), which shrinks the large
`/api/pieces` payload considerably.

### Frontend safety belt

- **Request timeout** — every `fetch` in `web/src/api/client.ts` gets an
  `AbortController` that aborts after `REQUEST_TIMEOUT_MS` (60 s, comfortably
  above the server's search cap). An aborted request becomes an `ApiError`
  with HTTP `408` naming the endpoint, so a hung server or dropped proxy
  response can never leave the UI waiting forever. The timer is cleared in a
  `finally` block on every path.
