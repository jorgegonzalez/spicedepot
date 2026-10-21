# Dependency security investigation — October 14, 2026

**Remediation update: October 21, 2026.** The historical investigation below is superseded by the remediation section at the end.

Investigated GitHub Dependabot alerts for commit `1980128`. No dependency versions changed and no alerts dismissed.

## Summary

GitHub currently reports **34 open alerts: 9 high, 22 moderate, 3 low**, representing **19 distinct advisories**. The earlier push message reported 27; the live API is the current snapshot. Duplicate manifest entries and the separate marketing dependency tree inflate the alert count.

All high-severity entries are in development/build tooling. That does not make them harmless: malicious source maps, CSS, browser statistics, or requests to a vulnerable development server can affect the developer/build machine. These tools are not the production SpiceDB service and are not shipped as servers inside the packaged app.

## Runtime assessment

- **rustls 0.23.40** is in the desktop network stack through tonic -> tokio-rustls. Upgrade to at least 0.23.45 first. The advisory concerns TLS handshake encryption-level validation; its description says the authenticated transcript prevents a network-position attacker from changing or completing the handshake. The current Railway connection uses plaintext gRPC over an encrypted OpenSSH tunnel, so that specific connection does not use rustls. Direct TLS connections do.
- **React Router:** the app uses declarative HashRouter, static NavLink destinations, and a constant `/schema` navigation target. No SSR/hydration, data-router redirects, or attacker-controlled navigation destination was found. The SSR and data-router advisories therefore do not match current usage; the generic navigation advisories have no identified trigger in this app. Upgrade anyway to avoid depending on these constraints. GitHub lists 7.18.0 for two advisories, requiring a deliberate v6-to-v7 migration rather than a blind patch bump.
- **serde_with 3.20.0:** transitive through Tauri. No application KeyValueMap use found. Exploitability through Tauri's own uses was not fully proven or ruled out. Update to at least 3.21.0.
- **glib:** associated with the Linux GTK/WebKit dependency tree, absent from the resolved native macOS tree. No direct application use of VariantStrIter found. Fixed at 0.20.0, but Tauri's compatible GTK dependency family must be checked; overriding a Rust major/minor API family is not a safe automatic fix. Full Linux reverse-tree resolution was blocked by packages not present in the offline cache.

## Advisory inventory

| Package | Severity | Fixed version reported by GitHub | Area | Advisory |
| --- | --- | --- | --- | --- |
| rustls | medium | 0.23.45 | Desktop TLS runtime | [GHSA-2mjx-qc3c-rqvc](https://github.com/advisories/GHSA-2mjx-qc3c-rqvc) |
| source-map-js | high | 1.2.2 | Build/dev tooling | [GHSA-68fv-2mgg-jv7q](https://github.com/advisories/GHSA-68fv-2mgg-jv7q) |
| postcss-selector-parser | medium | 7.1.6 | Build/dev tooling | [GHSA-rj75-hqrm-r3gf](https://github.com/advisories/GHSA-rj75-hqrm-r3gf) |
| serde_with | medium | 3.21.0 | Tauri serialization dependency | [GHSA-7gcf-g7xr-8hxj](https://github.com/advisories/GHSA-7gcf-g7xr-8hxj) |
| baseline-browser-mapping | medium | 2.11.0 | Build/dev tooling | [GHSA-w5vr-8v7q-w6rv](https://github.com/advisories/GHSA-w5vr-8v7q-w6rv) |
| react-router-dom | medium | 6.30.6 | Frontend routing; current usage mitigates | [GHSA-jjmj-jmhj-qwj2](https://github.com/advisories/GHSA-jjmj-jmhj-qwj2) |
| browserslist | high | 4.28.7 | Build/dev tooling | [GHSA-73wf-gq98-2v4g](https://github.com/advisories/GHSA-73wf-gq98-2v4g) |
| postcss-selector-parser | low | 6.1.3 | Build/dev tooling | [GHSA-w9m9-85wc-3x92](https://github.com/advisories/GHSA-w9m9-85wc-3x92) |
| postcss | medium | 8.5.23 | Build/dev tooling | [GHSA-fxqj-rqcc-2cmp](https://github.com/advisories/GHSA-fxqj-rqcc-2cmp) |
| @remix-run/router | medium | 1.23.3 | Frontend routing; current usage mitigates | [GHSA-2j2x-hqr9-3h42](https://github.com/advisories/GHSA-2j2x-hqr9-3h42) |
| postcss | high | 8.5.18 | Build/dev tooling | [GHSA-r28c-9q8g-f849](https://github.com/advisories/GHSA-r28c-9q8g-f849) |
| react-router | medium | 7.18.0 | Frontend routing; current usage mitigates | [GHSA-wrjc-x8rr-h8h6](https://github.com/advisories/GHSA-wrjc-x8rr-h8h6) |
| react-router | medium | 7.18.0 | Frontend routing; current usage mitigates | [GHSA-337j-9hxr-rhxg](https://github.com/advisories/GHSA-337j-9hxr-rhxg) |
| vite | medium | 6.4.3 | Build/dev tooling | [GHSA-v6wh-96g9-6wx3](https://github.com/advisories/GHSA-v6wh-96g9-6wx3) |
| @babel/core | low | 7.29.6 | Build/dev tooling | [GHSA-4x5r-pxfx-6jf8](https://github.com/advisories/GHSA-4x5r-pxfx-6jf8) |
| vite | high | 6.4.3 | Build/dev tooling | [GHSA-fx2h-pf6j-xcff](https://github.com/advisories/GHSA-fx2h-pf6j-xcff) |
| glib | medium | 0.20.0 | Linux native runtime dependency | [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) |
| vite | medium | 6.4.2 | Build/dev tooling | [GHSA-4w7w-66w2-5vf9](https://github.com/advisories/GHSA-4w7w-66w2-5vf9) |
| esbuild | medium | 0.25.0 | Build/dev tooling | [GHSA-67mh-4wv8-2f99](https://github.com/advisories/GHSA-67mh-4wv8-2f99) |

## Remediation order

1. Update rustls and serde_with in Cargo.lock within compatible dependency constraints.
2. Upgrade Vite in both desktop and marketing manifests to a supported patched release (GitHub minimum 6.4.3); ensure esbuild is at least 0.25.0. Check the React plugin peer requirements.
3. Refresh both JavaScript lockfiles for patched PostCSS (8.5.23+), source-map-js (1.2.2+), browserslist (4.28.7+), baseline-browser-mapping (2.11.0+), Babel (7.29.6+), and selector parser. Selector parser has advisories across v6 and v7; inspect parent constraints before using overrides.
4. Migrate React Router to a version fixing all listed advisories, checking HashRouter behavior.
5. Resolve the Linux glib dependency through a compatible upstream Tauri/GTK upgrade or a reviewed backport. Document any unresolved Linux-only exception explicitly.
6. Rebuild desktop and marketing assets, run tests, reopen the desktop app, verify read-only MCP connectivity, and recheck GitHub alerts after pushing. Do not dismiss alerts merely to make the count zero.

This investigation does not demonstrate credential theft or compromise. It is a dependency/reachability review, not an exploit test or a complete transitive source audit. No tests or rebuild were needed because no implementation changed.


## Remediation — October 21, 2026

Updated desktop and marketing tooling to Vite 7.3.7 and Tailwind 4.3.3, with the dedicated Tailwind PostCSS integration. Updated React Router to 7.18.4 and refreshed compatible dependencies. Updated Tauri 2 native/frontend packages together. Documented Node >=22.12 and Rust >=1.90 and aligned CI Node versions.

Updated rustls to 0.23.46 and serde_with to 3.24.0. A fresh RustSec scan also found additional issues, now fixed with h2 0.4.20, rustls-webpki 0.103.16, quick-xml 0.42.0 (via plist 1.10.3), anyhow 1.0.104, and event-listener 5.4.2.

GLib 0.18.5 is vendored under src-tauri/vendor/glib with the exact upstream mutable-out-pointer fix from gtk-rs/gtk-rs-core#1343. Comparison to the official crate confirmed only src/variant_iter.rs changed; upstream license retained. Cargo resolves the local patch, not the vulnerable registry package. Its version is intentionally unchanged. Audit tools may skip local packages, so the zero audit count alone does not validate this backport. Linux runtime validation remains outstanding.

### Verification

- Desktop pnpm audit: zero known vulnerabilities.
- Marketing npm audit: zero known vulnerabilities.
- Rust cargo audit: zero vulnerabilities and zero soundness warnings; five unmaintained warnings remain (proc-macro-error, rustls-pemfile, zbus_names, zvariant, zvariant_derive).
- Desktop TypeScript/Vite build and packaged debug macOS build passed.
- Marketing build passed; local preview visually inspected.
- Rebuilt desktop opened; connection layout visually inspected after CSS migration.
- Rust unit tests: five passed, seven isolated integration tests ignored.
- Strict Clippy checks passed with warnings treated as errors.
- Local MCP list_connections succeeded. Production schema/check requests could not complete: Railway SSH reports connection timeout. No production writes performed.

No security alerts were dismissed or suppressed. Remaining maintenance warnings require upstream migrations rather than merely increasing version numbers. This is not a full security certification or cross-platform runtime verification.
