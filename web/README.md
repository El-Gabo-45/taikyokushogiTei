# Frontend (web/) — development

Requirements: Node 16+ and `npm`.

Install and development:

```bash
cd web
npm install
npm run dev   # development mode (vite)

# or production build
npm run build
```

The Rust server serves the `web/dist` content when built; during development use Vite's dev server and configure a proxy for calls to the Rust backend if needed.
