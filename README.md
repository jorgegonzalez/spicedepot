# SpiceLens

A cross-platform desktop GUI for [SpiceDB](https://github.com/authzed/spicedb), built with Tauri 2 + Rust + React.

> Status: **v1 feature-complete, alpha quality**. All planned features ship; UX polish and signed release binaries are next.

## Install

Pre-built binaries land on the [Releases page](https://github.com/your-org/spicelens/releases) (macOS universal `.dmg`, Linux `.AppImage` / `.deb`, Windows `.msi`). Each release is built from the tagged commit by [`.github/workflows/release.yml`](.github/workflows/release.yml).

To build from source, see "Build from source" below.

## Features

- **Connection manager** — save named SpiceDB connections (endpoint, token, TLS on/off). Tokens stored in the OS keychain.
- **Schema editor** — view and write SpiceDB schemas with syntax highlighting via CodeMirror.
- **Permission checker** — interactive `CheckPermission` form with fully-consistent reads (the just-written relationship shows up immediately).
- **Lookup** — `LookupResources` ("which docs can alice view?") and `LookupSubjects` ("who can view doc1?") via a single page with a tab toggle. Results render as a sortable table.
- **Relationship browser** — filtered `ReadRelationships` table with per-row delete + an inline form to add single relationships (CREATE or TOUCH).
- **Watch viewer** — live `WatchService.Watch` stream rendered as a scrolling log of relationship changes (CREATE / TOUCH / DELETE) with optional object-type filter.
- **Insecure (plaintext) gRPC** — works correctly out of the box, useful for local-dev SpiceDB.
- TLS gRPC support via `rustls` (no native OpenSSL dependency).

Roadmap (post-v1 polish):

- Signed release binaries (Apple notarization, Windows code-signing)
- Caveat context editor in the permission checker
- "Load more" pagination for lookup / relationships once result sets get big
- Schema validation feedback before write

## Build from source

Requirements:

- Rust (stable, 1.80+)
- Node.js 20+
- pnpm 9+
- A C toolchain (Xcode CLT on macOS, MSVC on Windows, build-essential on Linux)
- WebKitGTK 4.1 + libssl + libsoup3 on Linux

```bash
pnpm install
pnpm tauri dev
```

To produce a release bundle:

```bash
pnpm tauri build
```

## Updating the SpiceDB proto definitions

Proto files are vendored under `src-tauri/proto/` for reproducible builds. To refresh them:

```bash
./scripts/sync-proto.sh
```

The script downloads pinned versions of the [authzed/api](https://github.com/authzed/api) protos and their transitive dependencies. Edit the `*_REF` variables at the top of the script to bump versions.

## CI

Every PR runs three jobs in [`.github/workflows/ci.yml`](.github/workflows/ci.yml):

1. **Rust** — `cargo build --all-targets`, `clippy --no-deps -- -D warnings`, `cargo test --lib`
2. **Frontend** — `pnpm typecheck` + `pnpm build`
3. **Integration** — spins up SpiceDB v1.52.0 as a service container and runs the `#[ignore]`'d gRPC tests against it (`cargo test --lib -- --ignored`)

The Rust job stubs out `dist/` because `tauri::generate_context!` insists the directory exist at compile time even though `cargo check` doesn't need Vite output.

## Support the project

SpiceLens is free and open source under the MIT license. If it saves you time, a few ways to keep it healthy:

- ♥ **[GitHub Sponsors](https://github.com/sponsors/REPLACE_ME)** — recurring support, listed in the repo
- ☕ **[Buy me a coffee](https://buymeacoffee.com/REPLACE_ME)** — one-off thanks
- **[Probe](https://probe.example.com/spicelens)** — paid commercial support, priority issues, and custom features

The same links live in the app's footer.

## License

MIT — see [LICENSE](LICENSE).

## Contributing

This is an early-stage project. Issues and PRs welcome. See [CLAUDE.md](CLAUDE.md) for the architecture overview.
