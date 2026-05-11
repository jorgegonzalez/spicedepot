# SpiceLens marketing site

Static site for [spicelens.app](https://spicelens.app) (or wherever this ends up deployed). Plain HTML + Tailwind v3 + Vite. No framework — the FAQ uses native `<details>`/`<summary>` and the rest is pure layout.

## Develop

```bash
cd marketing
npm install
npm run dev      # → http://localhost:4321
```

> **Why npm instead of pnpm?** pnpm 11.0.8 has a deps-status check that fails on esbuild's (cosmetic) ignored postinstall warning, blocking `pnpm build` even after a successful install. The desktop app at the repo root uses pnpm and is unaffected; only this subdirectory needs npm. If a future pnpm release fixes the check, switch back.

## Build for production

```bash
npm run build    # → dist/ (deployable as-is to any static host)
npm run preview  # serve dist/ locally on :4173
```

## Deploy

`dist/` is a fully static bundle — drop it on:

- **Cloudflare Pages** — point at this directory, build command `npm run build`, output `dist`
- **Vercel / Netlify** — same as above
- **GitHub Pages** — push `dist/` to a `gh-pages` branch
- **Any S3-compatible** — `aws s3 sync dist/ s3://bucket --delete`

## Edit

- **Copy** — all text lives in [index.html](./index.html). It's one file by design.
- **Styles** — Tailwind via [`src/style.css`](./src/style.css); custom utilities defined in `@layer components`.
- **Icon** — same `assets/icon.png` the desktop app uses, copied into this dir at scaffold time.
- **Sponsor / support links** — match what's in the desktop app's [`src/lib/support.ts`](../src/lib/support.ts) and [`.github/FUNDING.yml`](../.github/FUNDING.yml). When you update one, update all three.
