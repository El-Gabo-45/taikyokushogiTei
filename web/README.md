# Frontend (web/) — development

Requirements: Node.js 20.19+ or 22.12+ (Vite 8 requirement) and `npm`.

Install and development:

```bash
cd web
npm install
npm run dev   # development mode (vite)

# production build
npm run build
```

The Rust server serves the `web/dist` content when built; during development use Vite's dev server and configure a proxy for calls to the Rust backend if needed.

Tests (vitest, no browser needed):

```bash
npm test          # run once
npm run test:watch
```

The suite covers the API client (URLs, payloads, error mapping), the game
state machine (init/play/undo/AI/analysis/auto-play guard), the pure view
helpers (flip, zoom clamp, score formatting/chart), the kanji table and the
game-file exporters (JSON shape/CLI compatibility and PGN tags/move text).

## Game export

The **Export JSON** button below the move log downloads the current game
record. The top-level `tsfen`, `moves`, `side` (`"b"`/`"w"`) and
`move_number` fields match the CLI's `GameFile`, so the file can be loaded
with:

```text
taikyokushogi-cli
> load taikyoku-game-20261110-153012-move-42.json
```

The exporter also adds informational fields (`format`, `version`,
`exported_at`, `app`, `engine`, `result`, piece counts, no-progress plies).
The CLI ignores unknown JSON fields, so they do not affect loading.

**Export PGN** writes a PGN-style record with conventional tags (`Event`,
`Date`, `Variant`, `Result`, `Engine`, …) and one token per move, e.g.
`1. BG.F34-B30 BG.e3-F30 *`. It is intended for reading and sharing:
Taikyoku's 36×36 board and 301 piece types are not covered by the PGN
standard, so no chess GUI can replay it as a normal game. Use JSON to resume
a game in the CLI.

Both buttons use the timestamped name
`taikyoku-game-YYYYMMDD-HHMMSS-move-N.{json,pgn}`.

Note: the compiled `web/dist/` is committed to the repository (see `web/.gitignore`) so the Rust server can serve it without a Node.js toolchain. If the directory is missing or stale, run `npm run build`; the CI workflow rebuilds it automatically.
