# practice-hub

Guitar + pad-rudiment practice trainer. Keyboard simulator: F = left/down, J = right/up, Space = start/pause.

- Run: `npm install && npm run dev`
- Test: `npm test`
- Build: `npm run build` (output in `dist/`)
- Deploy: push to `main`; `.github/workflows/deploy.yml` builds and publishes to GitHub Pages.
- One-time: repo Settings → Pages → Source: "GitHub Actions".
- Data stays in your browser (IndexedDB); use Settings → Export/Import JSON to back up.
