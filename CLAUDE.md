# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

## Build & Run

```bash
# Build everything
cargo build

# Build release
cargo build --release

# Run the daemon (requires Docker socket and write access to /run/rustploy/)
cargo run -p daemon

# Run the TUI client
cargo run -p client

# Run a specific test
cargo test -p daemon test_name
cargo test -p shared

# Check all crates without linking
cargo check --workspace
```

The daemon binary is `rustployd`; the client binary is `rustploy`.

## glacier-ui (dependência da crate remote-ui)

A crate `remote-ui` consome `glacier-ui` **do crates.io** (versão fixada no `Cargo.toml`), não o código-fonte local em `~/Development/rust/glacier-ui`.

**Regra (sempre):** quando uma mudança no `glacier-ui` for necessária (renomear um item público, corrigir bug, adicionar recurso), o fluxo é **sempre publicar uma nova versão e subir a dependência** — nunca usar `[patch.crates-io]` ou dependência por `path` para contornar:

1. Aplicar a mudança em `~/Development/rust/glacier-ui`.
2. Bump da versão em `glacier-ui/Cargo.toml` (ex.: `0.3.1` → `0.3.2`).
3. `cargo publish` (validar antes com `cargo publish --dry-run`).
4. Subir a versão de `glacier-ui` no `crates/remote-ui/Cargo.toml` para a recém-publicada.
5. `cargo check -p remote-ui` para confirmar.

## Configuration

Config is loaded from `$RUSTPLOY_CONFIG`, then `/etc/rustploy/config.toml`, then `~/.config/rustploy/config.toml`. If none exist, defaults are used.

Key env-var overrides: `RUSTPLOY_SOCKET_PATH`, `RUSTPLOY_DB_PATH`, `RUSTPLOY_LOG_LEVEL`.  
Daemon logs structured JSON; control verbosity with `RUST_LOG=<level>` or `RUSTPLOY_LOG_LEVEL`.

Default socket: `/run/rustploy/rustploy.sock`  
Default DB path: `/var/lib/rustploy/db`  
Default master key: `/etc/rustploy/master.key`

## Architecture

### Crate layout

| Crate | Binary | Role |
|-------|--------|------|
| `shared` | — | Models, protocol types, config structs shared by both sides |
| `daemon` | `rustployd` | Long-running server: API, DB, Docker, ingress, deploy engine |
| `client` | `rustploy` | Ratatui TUI that talks to the daemon |

### IPC protocol

Raw **postcard-encoded** frames over the Unix Domain Socket — no HTTP involved. Every frame is `[u32 LE length][postcard bytes]`. The client opens a connection and sends a `ClientFrame`: `Rpc(Command)` gets a single `Response` frame back; `Subscribe { service_id }` turns the connection into a stream of `Event` frames (logs, metrics, deploy progress).  
All message types — `ClientFrame`, `Command`, `Response`, `Event` — are defined in `crates/shared/src/protocol.rs`.  
Postcard uses varint encoding: small integers and short strings produce fewer bytes than bincode, with no schema overhead.

### Daemon internals (`crates/daemon/src/`)

- **`api/server.rs`** — UDS listener; decodes each `ClientFrame` and routes `Rpc` to `dispatch()`, `Subscribe` to the event stream.
- **`api/routes.rs`** — `dispatch()` matches every `Command` variant to a handler module.
- **`api/handlers/`** — one file per command (e.g. `deploy_start.rs`, `project_create.rs`).
- **`db/`** — SQLite (via `sqlx`) wrappers for projects, services, deployments.
- **`deploy/executor.rs`** — `DeployExecutor` drives the deploy state machine in a `tokio::spawn`. States: `Pending → ResolvingDeps → PullingImage | CloningRepo → BuildingImage → Staging → HealthcheckPolling → SwappingIn → Draining → Promoting → Live`. Rollback lands at `Failed`.
- **`docker/`** — bollard wrappers: `images` (pull/build), `containers` (create/start/stop/rename/remove), `networks` (per-project bridge networks named `rp_<project_id_prefix>`).
- **`ingress/proxy.rs`** — hyper-based reverse proxy; route table is an `arc-swap`-protected `HashMap<domain, upstream>` updated live by the deploy executor.
- **`event_bus.rs`** — in-process broadcast channel; daemon modules publish `Event` values; `/stream` handler fans them out to connected clients.
- **`secrets.rs`** — `age`-based encryption; secrets stored by name, referenced in `ServiceSpec.env_vars` as `EnvVarValue::Secret(name)`.
- **`metrics.rs`** — background loop that polls Docker stats and publishes `ContainerMetrics` events.

### Client internals (`crates/client/src/`)

- **`app.rs`** — `App` struct (all UI state), `View`/`SidebarItem` enums, `Command`/`CmdContext` pairing. `App::apply_event()` handles incoming daemon events; `App::handle_response()` handles RPC responses.
- **`events.rs`** — input event loop; maps key presses to mutations on `App`.
- **`transport.rs`** — sync UDS client (`std::os::unix::net::UnixStream`, no tokio); exposes `send(Command) → Response` and a blocking stream subscription (run on a dedicated thread).
- **`ui/mod.rs`** — top-level render dispatcher; delegates to sub-modules by current `View`.
- **`ui/sidebar.rs`**, **`ui/projects.rs`**, **`ui/service_detail.rs`**, **`ui/deploy_log.rs`**, **`ui/metrics.rs`**, **`ui/settings.rs`** — individual screen renderers.

### Deploy pipeline detail

Git-sourced deploys: clone repo (`git2` in `spawn_blocking` because `!Send`) → build Docker image (tar build context, stream output as `LogLine` events) → create staging container → healthcheck poll (TCP/HTTP/DockerNative) → swap ingress route → drain old container → rename staging → `Live`.  
Registry-sourced deploys skip clone/build and go straight to pull → staging.

Containers are named `rp_<service_name>_live` (production) and `rp_<service_name>_<deploy_id[:8]>_staging` (in-flight). Build artifacts live at `<db_path>/builds/<deployment_id>/` and are deleted on promotion or rollback.
