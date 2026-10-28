# Frontend (web/) — development

Requirements: Node.js 20.19+ or 22.12+ (Vite 8 requirement) and `npm`.

Install and development:

```bash
cd web
npm install
npm run dev   # development mode (vite)

# or production build
npm run build
```

The Rust server serves the `web/dist` content when built; during development use Vite's dev server and configure a proxy for calls to the Rust backend if needed.

Note: `web/dist/` is gitignored (see `web/.gitignore`). It must be built with `npm run build` before running the Rust server, or the server will fall back to a "frontend not built" page. The CI workflow builds it automatically.
