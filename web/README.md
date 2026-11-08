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
helpers (flip, zoom clamp, score formatting/chart) and the kanji table.

Note: the compiled `web/dist/` is committed to the repository (see `web/.gitignore`) so the Rust server can serve it without a Node.js toolchain. If the directory is missing or stale, run `npm run build`; the CI workflow rebuilds it automatically.
