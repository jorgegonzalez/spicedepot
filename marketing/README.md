# SpiceDepot marketing site

Marketing site for [spicedepot.app](https://spicedepot.app), built with plain HTML, Tailwind CSS, and Vite. The FAQ uses native `<details>` and `<summary>` elements.

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
2. Import the GitHub repository `jorgegonzalez/spicedepot`
3. **Root Directory** → click "Edit" → set to `marketing`
4. Framework Preset: Vite (auto-detected once Root Directory is set)
5. Deploy

Vercel will redeploy on every push to `main`. Preview deployments fire on PRs against any other branch.

**Custom domain**:

The apex (`spicedepot.app`) and `www` domains are attached to the Vercel project. In Cloudflare DNS, both use `A` records pointing to `76.76.21.21` with proxying disabled. Vercel provisions HTTPS after DNS verification.

## Alternative hosts

`dist/` is a fully static bundle — Cloudflare Pages, Netlify, S3, etc. all work too. The `vercel.json` headers (asset cache-control) are Vercel-specific but other hosts will just ignore them.

## Edit

- **Copy** — all text lives in [index.html](./index.html). It's one file by design.
- **Styles** — Tailwind via [`src/style.css`](./src/style.css); custom utilities defined in `@layer components`.
- **Icon** — same `assets/icon.png` the desktop app uses, copied into this dir at scaffold time.
- **Sponsor / support links** — match what's in the desktop app's [`src/lib/support.ts`](../src/lib/support.ts) and [`.github/FUNDING.yml`](../.github/FUNDING.yml). When you update one, update all three.
