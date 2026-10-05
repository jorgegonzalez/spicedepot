# SpiceDepot

A cross-platform desktop GUI for [SpiceDB](https://github.com/authzed/spicedb), built with Tauri 2 + Rust + React.

> Status: **v1 feature-complete, alpha quality**. All planned features ship; UX polish and signed release binaries are next.

## Install

### Homebrew

Install the desktop app with Homebrew after the first SpiceDepot release:

```bash
brew install --cask jorgegonzalez/tap/spicedepot
```

### Pre-built binaries

The existing releases were published under the former SpiceLens name. They include:

- macOS universal `.dmg` (Apple Silicon + Intel)
- Linux `.AppImage` + `.deb` (x64)
- Windows `.msi` + `setup.exe` (x64)

The binaries aren't notarized yet, so macOS may show a Gatekeeper warning on first launch (`right-click → Open → Open` to bypass). Notarization is on the roadmap.

### From source

See "Build from source" below.

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

## CI status

GitHub Actions runs the continuous integration and release workflows in this repository. Tagged releases build desktop packages for macOS, Linux, and Windows; the release workflow also updates the Homebrew tap.

## Support the project

SpiceDepot is free and open source under the Apache 2.0 license. If it saves you time, a couple of ways to keep it healthy:

- ♥ **[GitHub Sponsors](https://github.com/sponsors/jorgegonzalez)** — recurring support, listed in the repo
- ☕ **[Buy me a coffee](https://buymeacoffee.com/jorgegonzalez)** — one-off thanks

The same links live in the app's footer.

## License

**Apache 2.0** — see [LICENSE](LICENSE) for the full text and [NOTICE](NOTICE) for the copyright stanza. Same license SpiceDB itself uses, no surprises.

## Contributing

This is an early-stage project. Issues and PRs welcome. See [CLAUDE.md](CLAUDE.md) for the architecture overview.
