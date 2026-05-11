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

## Deploy (Vercel)

This subdirectory is configured to deploy on Vercel — see [vercel.json](./vercel.json).

**One-time setup:**

1. Go to <https://vercel.com/new>
2. Import `jorgegonzalez/spicelens`
3. **Root Directory** → click "Edit" → set to `marketing`
4. Framework Preset: Vite (auto-detected once Root Directory is set)
5. Deploy

Vercel will redeploy on every push to `main`. Preview deployments fire on PRs against any other branch.

**Custom domain** (once registered):

In the Vercel project → Settings → Domains → add `spicelens.app`. Vercel will give you the DNS records to set at your registrar. Apex domain via `A` records to Vercel's IP; `www` via `CNAME` to `cname.vercel-dns.com`. HTTPS provisions automatically.

## Alternative hosts

`dist/` is a fully static bundle — Cloudflare Pages, Netlify, S3, etc. all work too. The `vercel.json` headers (asset cache-control) are Vercel-specific but other hosts will just ignore them.

## Edit

- **Copy** — all text lives in [index.html](./index.html). It's one file by design.
- **Styles** — Tailwind via [`src/style.css`](./src/style.css); custom utilities defined in `@layer components`.
- **Icon** — same `assets/icon.png` the desktop app uses, copied into this dir at scaffold time.
- **Sponsor / support links** — match what's in the desktop app's [`src/lib/support.ts`](../src/lib/support.ts) and [`.github/FUNDING.yml`](../.github/FUNDING.yml). When you update one, update all three.
