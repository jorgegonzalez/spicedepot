# SpiceLens

A cross-platform desktop GUI for [SpiceDB](https://github.com/authzed/spicedb), built with Tauri 2 + Rust + React.

> Status: **alpha**. Connection manager and schema editor are functional. Permission checking, relationship browsing, and lookup are on the roadmap.

## Features

- **Connection manager** — save named SpiceDB connections (endpoint, token, TLS on/off). Tokens stored in the OS keychain.
- **Schema editor** — view and write SpiceDB schemas with syntax highlighting via CodeMirror.
- **Permission checker** — interactive `CheckPermission` form with fully-consistent reads (the just-written relationship shows up immediately).
- **Lookup** — `LookupResources` ("which docs can alice view?") and `LookupSubjects` ("who can view doc1?") via a single page with a tab toggle. Results render as a sortable table.
- **Relationship browser** — filtered `ReadRelationships` table with per-row delete + an inline form to add single relationships (CREATE or TOUCH).
- **Watch viewer** — live `WatchService.Watch` stream rendered as a scrolling log of relationship changes (CREATE / TOUCH / DELETE) with optional object-type filter.
- **Insecure (plaintext) gRPC** — works correctly out of the box, useful for local-dev SpiceDB.
- TLS gRPC support via `rustls` (no native OpenSSL dependency).

Roadmap:

- Bulk `DeleteRelationships` (delete-by-filter)

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

## License

MIT — see [LICENSE](LICENSE).

## Contributing

This is an early-stage project. Issues and PRs welcome. See [CLAUDE.md](CLAUDE.md) for the architecture overview.
