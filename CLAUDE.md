# CLAUDE.md

Architecture and conventions for working on SpiceDepot. Read this before adding features.

## What this is

A cross-platform desktop GUI for [SpiceDB](https://github.com/authzed/spicedb), built as a Tauri 2 app. The Rust backend owns all gRPC communication; the React frontend never touches gRPC directly.

## Stack

| Layer    | Tools                                                                       |
| -------- | --------------------------------------------------------------------------- |
| Backend  | Rust, Tauri 2, tonic 0.12, prost 0.13, tokio                                |
| Frontend | React 18, TypeScript, Vite, Tailwind v3, TanStack Query, Zustand, CodeMirror 6 |
| Storage  | tauri-plugin-store (JSON, non-sensitive) + keyring (OS keychain, tokens)    |
| Proto    | Vendored `.proto` files compiled by `tonic-build` with `protoc-bin-vendored` |

## Repo layout

```
.
├── src/                    React frontend
│   ├── main.tsx            entry, mounts QueryClient + HashRouter
│   ├── App.tsx             route table
│   ├── components/
│   │   ├── AppShell.tsx    chrome (header, nav, active-connection picker)
│   │   └── ui.tsx          Button / Input / Field / Banner primitives
│   ├── pages/
│   │   ├── ConnectionsPage.tsx    CRUD + Test for connections
│   │   └── SchemaPage.tsx         CodeMirror editor for read/write schema
│   ├── lib/
│   │   ├── tauri.ts        typed `invoke<T>()` wrapper
│   │   ├── api.ts          TS mirrors of every Tauri command
│   │   ├── store.ts        Zustand: persisted active-connection id
│   │   └── spicedbLang.ts  CodeMirror StreamLanguage for SpiceDB schema
│   └── styles.css          Tailwind base + CodeMirror tweaks
├── src-tauri/              Rust backend
│   ├── Cargo.toml
│   ├── build.rs            runs tonic-build + tauri-build
│   ├── tauri.conf.json     Tauri config
│   ├── capabilities/       Tauri 2 capability ACLs
│   ├── icons/              app icons
│   ├── proto/              vendored .proto files (see "Updating protos" below)
│   └── src/
│       ├── main.rs         calls `spicedepot_lib::run()`
│       ├── lib.rs          Tauri builder, plugins, command registration
│       ├── error.rs        AppError + AppResult — single error type for commands
│       ├── connections.rs  ConnectionStore (store + keyring)
│       ├── spicedb/
│       │   ├── mod.rs
│       │   ├── proto.rs    `tonic::include_proto!` mounts for generated code
│       │   └── client.rs   SpiceDbClient: builds Channel, attaches Bearer auth
│       └── commands/
│           ├── mod.rs       `client_for(state, id)` shared helper
│           ├── connections.rs
│           └── schema.rs
├── scripts/sync-proto.sh   re-vendor proto files from upstream (pinned)
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Build & dev

```bash
pnpm install            # once
pnpm tauri dev          # spawns Vite + opens the app
pnpm build              # frontend only
pnpm typecheck          # tsc --noEmit
cd src-tauri && cargo check
cd src-tauri && cargo test --lib
```

A vendored `protoc` ships via `protoc-bin-vendored`, so contributors don't need to install protobuf themselves. Set `PROTOC=/path/to/protoc` to override.

## Architecture: end-to-end command flow

The frontend never speaks gRPC. Every backend operation is a Tauri command. The pattern is:

1. **TS side** — call `api.foo(args)` from `src/lib/api.ts`. That wraps the raw `invoke()` and gives you a typed promise.
2. **Boundary** — Tauri serializes args as JSON, dispatches to the Rust handler.
3. **Command** — `src-tauri/src/commands/*.rs` function annotated `#[tauri::command]`. Pulls state, may call `client_for(&state, &connection_id).await?` to get a `SpiceDbClient`, then dispatches to it.
4. **Client** — `SpiceDbClient` holds a `tonic::Channel` and attaches `authorization: Bearer <token>` metadata to every request.
5. **Result** — `AppResult<T>` flows back; `AppError` serializes to a string the TS side throws as an `Error`. React Query surfaces it.

### Adding a new Tauri command

1. Add the function in `src-tauri/src/commands/<area>.rs` with `#[tauri::command]` and `AppResult<T>` return.
2. Wrap the body's result in `super::log_err(...)` so failures land in the Rust logs (the frontend only sees the error string).
3. Register it in `src-tauri/src/lib.rs` inside `tauri::generate_handler![...]`.
4. Add a typed wrapper in `src/lib/api.ts`.
5. Use it from React via `useQuery` / `useMutation` against `api.foo`.

### Argument naming convention

Tauri converts Rust `snake_case` parameters to `camelCase` for the JS side automatically. From TS, pass `{ connectionId: "..." }` for a Rust param `connection_id: String`. The `invoke` wrapper passes args verbatim — match what Tauri expects.

## gRPC client

`SpiceDbClient::connect(DialConfig)` does the work of dialing. Key behaviors:

- **Insecure (plaintext) mode** — when `insecure: true`, builds an `http://` URI and skips TLS config. This is the "known gap" mode that must keep working — `localhost:50051` against a dev SpiceDB should always succeed. Covered by `host_port_insecure_becomes_http` and `scheme_flag_mismatch_errors` unit tests.
- **TLS mode** — when `insecure: false`, builds an `https://` URI and applies `ClientTlsConfig::new().with_native_roots()` (rustls + native CA store via the `tls-roots` feature on tonic).
- **Endpoint normalization** — accepts `host:port`, `http(s)://host:port`, or `grpc(s)://host:port`. The scheme must match the `insecure` flag or `build_uri` returns an `InvalidEndpoint` error before we even try to dial.
- **Auth** — every request gets `authorization: Bearer <token>` metadata. SpiceDB's pre-shared-key auth is just a bearer header.
- **Schema empty case** — `read_schema` translates `gRPC NOT_FOUND` to "no schema written yet" (empty string), so the UI can render a blank editor without throwing.

Channels are built per call today. If/when streaming features (Watch, lookup pagination) land, switch to a per-connection-id channel pool keyed in `AppState`.

## Connections store

Two stores, one logical record:

- **JSON via `tauri-plugin-store`** (`connections.json` in the OS app data dir): `id`, `name`, `endpoint`, `insecure`, timestamps. Safe to back up.
- **OS keychain via `keyring`** (service `dev.spicelens.app`, user = connection UUID): the bearer token. Never written to disk by us.

`ConnectionInput.token == ""` on update means "leave token unchanged" — important UX detail.

### Keychain calls must use `spawn_blocking`

The `keyring` crate is **synchronous** — it blocks the calling thread for the duration of the keychain operation, and on macOS those operations can stall waiting for the user to dismiss an authorization prompt. Calling it directly from a `#[tauri::command] async fn` would block a Tokio worker thread (and on macOS, potentially deadlock the runtime).

Every keychain call in `connections.rs` is wrapped in `tokio::task::spawn_blocking`. The helper `set_token_blocking` shows the pattern; reuse it (or model new keychain code on it) — never call `keyring::Entry::*` directly from an async context.

### Linux: requires Secret Service for token persistence

We use the `linux-native-sync-persistent` keyring feature, which layers the kernel keyutils session keyring (fast, in-process) over Secret Service (persistent, D-Bus). On a typical Linux desktop (GNOME/KDE/Cinnamon/etc.) tokens persist across reboots; on a headless Linux box without a running `secret-service` provider, tokens behave like `linux-native` alone — they survive within a session but vanish on reboot. Document this in the connection-creation flow once we expose it to the user.

## Proto code generation

Proto files live under `src-tauri/proto/` (vendored, committed). `build.rs` invokes `tonic-build` to emit Rust into `OUT_DIR`. Two non-obvious details:

1. **Sibling package mounting** — generated paths inside nested message modules use `super::super::super::super::google::rpc::Status` to reach cross-package types. Because of how `tonic::include_proto!` flattens content, both `authzed` and `google` must be mounted as siblings inside `spicedb/proto.rs`. Don't relocate one without the other or builds break with `could not find google in super`.
2. **Well-known types** — `google.protobuf.*` (Struct, Timestamp, Duration, Any) come from `prost_types` automatically. `google.rpc.Status` is **not** well-known; we compile it ourselves and mount it under `proto::google::rpc`.

### Updating proto definitions

```bash
./scripts/sync-proto.sh
```

Bump the `*_REF` variables at the top of the script to upgrade. Commit the resulting changes. Re-run `cargo build` afterwards (the `cargo:rerun-if-changed=proto` directive in `build.rs` will pick up the diff).

## State management (frontend)

- **Server state** (everything the Rust backend owns): TanStack Query. Query keys are `["connections"]` and `["schema", connectionId]`. Invalidate on mutation success.
- **Client state** (UI-only, transient): plain `useState`. The one exception is `useActiveConnection` (Zustand + persist) which survives reloads.

Don't reach for Zustand for things that belong in React Query, and don't put a server response into Zustand.

## Adding a new feature page

The next features (permission checker, relationship browser, lookup) follow the same pattern as `SchemaPage`:

1. Add Rust command(s) in `src-tauri/src/commands/<feature>.rs`.
2. Register in `lib.rs`.
3. Mirror in `src/lib/api.ts`.
4. New `src/pages/<Feature>Page.tsx` that reads `useActiveConnection().activeConnectionId` and uses `useQuery`/`useMutation`.
5. Add a route in `src/App.tsx` and a nav entry in `AppShell.tsx`.

## Conventions

- Errors: every Tauri command returns `AppResult<T>`. Map upstream errors via `From` impls in `error.rs` rather than ad-hoc `.map_err`.
- gRPC: don't expose raw `tonic::Status` to the frontend; the `From<tonic::Status>` impl in `AppError` already produces a clean `grpc <code>: <message>` string.
- Frontend imports: use the `@/` alias (e.g. `@/lib/api`) — configured in both `tsconfig.json` and `vite.config.ts`.
- Keep `dist/` out of git (gitignored). Tauri's `generate_context!` insists the path *exists* at compile time, but the build pipeline rebuilds it.

## Roadmap (per priority)

1. ✅ Connection manager
2. ✅ Schema read/write
3. ✅ Permission checker (`CheckPermission`)
4. ✅ Relationship browser (`ReadRelationships` + single-row `WriteRelationships` create/touch/delete)
5. ✅ Lookup (`LookupResources`, `LookupSubjects`)
6. ✅ Watch stream viewer (`WatchService.Watch`)
7. ☐ Bulk `DeleteRelationships` (delete-by-filter) — scoped out of #4 for v1

Each gets its own page + commands; the gRPC client exposes `permissions()` for everything in the authz family. Most streaming RPCs (lookup, read-relationships) drain into a Vec inside Rust before returning — fine for the SpiceDB-default 1000-result cap. The Watch viewer is the exception: its stream is unbounded, so it uses `tauri::ipc::Channel<WatchEvent>` (see "Watch streaming" below).

## CheckPermission and Lookup: consistency choice

Every authz read (`CheckPermission`, `LookupResources`, `LookupSubjects`) sends `Consistency = FullyConsistent(true)`. SpiceDB's default (`MinimizeLatency`) reads from any snapshot and can return a stale result — surprising in an interactive tool where the user has just written a relationship and is testing the effect. The `check_permission_with_relationship` and `lookup_round_trip` integration tests both exercise this exact ordering and would fail without `FullyConsistent`. If a future command needs different consistency semantics, set it per-command rather than relaxing this default.

## Server-streaming lookups

`PermissionsService.LookupResources` and `LookupSubjects` are server-streaming RPCs. The current implementation drains the stream into a `Vec` inside `SpiceDbClient::{lookup_resources, lookup_subjects}` before returning, so the Tauri command is a single round-trip from the frontend's perspective. SpiceDB caps server-side response counts (default 1000) and our optional `limit` field can request fewer.

If/when we want progressive rendering (large result sets, "load more" pagination, etc.), swap the `Vec` for a `tauri::ipc::Channel<T>`: spawn the streaming RPC in a Tokio task and send each item as it arrives. The frontend `useMutation` would become a subscription pattern. Not worth the complexity for these unless the result-cap becomes a real problem.

## Watch streaming

`WatchService.Watch` is the one feature where the Vec-drain pattern doesn't fit — the stream is unbounded by design. The plumbing:

1. **Rust side** — `SpiceDbClient::watch_stream` returns the raw `tonic::Streaming<WatchResponse>` rather than draining it. The `commands::watch::watch_start` command accepts a `tauri::ipc::Channel<WatchEvent>` parameter, spawns a Tokio task that drives the stream and forwards each response (one event per `RelationshipUpdate`) to the channel.
2. **Lifecycle** — `watch_start` returns a fresh UUID, and stores the spawned task's `AbortHandle` in `AppState.watches: Mutex<HashMap<String, AbortHandle>>`. `watch_stop` looks up the handle and calls `.abort()`, which drops the stream and cancels the gRPC call. The task also self-removes its entry on terminal end / error so completed handles don't accumulate.
3. **Frontend side** — `WatchPage` constructs a `new Channel<WatchEvent>()` via `@tauri-apps/api/core`, attaches `channel.onmessage`, and passes the channel as the `onEvent` arg to `invoke('watch_start', ...)`. Unmounting the page (or switching connections) calls `watch_stop`.
4. **Event shape** — `WatchEvent` is a tagged union (`#[serde(tag = "kind")]`) with `update | schema_changed | ended | error` variants. The `update` variant flattens the proto `Relationship` into the same `RelationshipRow` the relationship browser uses, so the same TS type covers both pages.

The integration test (`watch_receives_updates`) exercises the full path against a live SpiceDB: open the stream, write a relationship from a second task, assert the write lands in the stream within 5 seconds, then drop the stream to verify cancellation.

This is the template for any future server-streaming feature where the stream is conceptually unbounded.
