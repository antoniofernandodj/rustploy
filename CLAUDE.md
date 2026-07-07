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

## glacier-ui (dependência da crate rustploy-gui)

A crate `rustploy-gui` consome `glacier-ui` **do crates.io** (versão fixada no `Cargo.toml`), não o código-fonte local em `~/Development/rust/glacier-ui`.

**Regra (sempre):** quando uma mudança no `glacier-ui` for necessária (renomear um item público, corrigir bug, adicionar recurso), o fluxo é **sempre publicar uma nova versão e subir a dependência** — nunca usar `[patch.crates-io]` ou dependência por `path` para contornar:

1. Aplicar a mudança em `~/Development/rust/glacier-ui`.
2. Bump da versão em `glacier-ui/Cargo.toml` (ex.: `0.3.1` → `0.3.2`).
3. `cargo publish` (validar antes com `cargo publish --dry-run`).
4. Subir a versão de `glacier-ui` no `crates/rustploy-gui/Cargo.toml` para a recém-publicada.
5. `cargo check -p rustploy-gui` para confirmar.

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
| `rustploy-gui` | `rustploy-gui` | glacier-ui (XML→iced) desktop client. Toda a rede/lógica de negócio vive em Luau (`views/scripts/`, pacotes `fmt/`/`handlers/`/`net/` — ver `docs/luau-modularizacao-pacotes.md`), falando com o daemon pela **API HTTP/JSON + SSE** (`crates/daemon/src/api/http_api.rs`), não pelo UDS local. |

### IPC protocol

Raw **postcard-encoded** frames over the Unix Domain Socket — no HTTP involved. Every frame is `[u32 LE length][postcard bytes]`. The client opens a connection and sends a `ClientFrame`: `Rpc(Command)` gets a single `Response` frame back; `Subscribe { service_id }` turns the connection into a stream of `Event` frames (logs, metrics, deploy progress).  
All message types — `ClientFrame`, `Command`, `Response`, `Event` — are defined in `crates/shared/src/protocol.rs`.  
Postcard uses varint encoding: small integers and short strings produce fewer bytes than bincode, with no schema overhead.

This UDS/postcard scheme is what the **TUI client** (`crates/client/`) speaks. `rustploy-gui` speaks a separate, parallel channel instead — plain **HTTP/JSON + SSE** (`crates/daemon/src/api/http_api.rs`: `POST /api/rpc`, `GET /api/events`, `GET /api/health`) — because its logic runs in Luau (`fetch`/`sse`), which has no UDS access. Both channels reuse the same `dispatch()`/`Command`/`Response` types; only the wire framing differs.

### Daemon internals (`crates/daemon/src/`)

- **`api/server.rs`** — UDS listener; decodes each `ClientFrame` and routes `Rpc` to `dispatch()`, `Subscribe` to the event stream.
- **`api/routes.rs`** — `dispatch()` matches every `Command` variant to a handler module.
- **`api/handlers/`** — one file per command (e.g. `deploy_start.rs`, `project_create.rs`).
- **`db/`** — SQLite (via `sqlx`) wrappers for projects, services, deployments.
- **`deploy/executor.rs`** — `DeployExecutor` drives the deploy state machine in a `tokio::spawn`. States: `Pending → ResolvingDeps → PullingImage | CloningRepo → BuildingImage → Staging → HealthcheckPolling → SwappingIn → Draining → Promoting → Live`. Rollback lands at `Failed`.
- **`docker/`** — bollard wrappers: `images` (pull/build), `containers` (create/start/stop/rename/remove), `networks` (per-project bridge networks named `rp_<project_id_prefix>`). No `volumes.rs` — rustploy never creates named volumes, only bind mounts (`ServiceSpec.volumes`).
- **`api/handlers/docker_inventory.rs`** — host-wide Docker listing for the Docker tab (`Command::DockerImages/Volumes/Networks`), not just rustploy-managed resources. Images/volumes come from a single `docker system df` call (the only Docker Engine endpoint that computes in-use/reference counts for free); networks are cross-referenced against `list_containers(all: true)` by hand (list-networks never populates its own `Containers` field). Project/service attribution is best-effort: images by tag (`rp_<safe_name>:...` for Git builds, exact string match for registry images), networks by the `rp_net_<project_id_short>` naming convention; volumes get none (no label to key off). Also has `stop_all_managed` (`Command::StopAllManaged`) — stops every rustploy service by replaying `service_stop::handle` for each one regardless of what the DB's status column currently says, so state drift can't leave a container running.
- **`api/handlers/docker_prune.rs`** — removes unused images/volumes/networks/containers/build cache (`Command::PruneImages/Volumes/Networks/Containers/BuildCache`, all through `Response::PruneResult`).
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

### rustploy-gui internals (`crates/rustploy-gui/src/`)

UI declared in XML templates (`views/*.xml`) and rendered by the published `glacier-ui` crate (see the rule above — never a local `path`/`[patch]` dependency). Every network/business-logic responsibility (login, the SSE consumer, navigation, every mutation) lives in Luau (`views/scripts/`), **not** in this Rust source — `src/` here is only the iced bootstrap, window chrome, and local (non-daemon) persistence. Run from the workspace root (`cargo run -p rustploy-gui`) or from a packaged layout (see `assets.rs`) — template/script paths are relative to the CWD `glacier-ui` resolves them against, not necessarily the launch directory.

- **`main.rs`** — just the `iced::application(...)` bootstrap (fonts, window settings); calls `assets::locate_and_chdir()` before anything else loads.
- **`assets.rs`** — locates the asset base directory at startup and `chdir`s into it, so every CWD-relative reference (Rust-side and inside the XML/Luau themselves) resolves the same way regardless of how the app was launched. Resolution order: `$RUSTPLOY_UI_ASSETS` override → the executable's own directory (portable/Windows `.zip` layout) → `/usr/share/rustploy` (Debian package layout) → current directory as-is (dev run from the workspace root). Probes for `crates/rustploy-gui/views/app.xml` to confirm a candidate base is valid.
- **`app/mod.rs`** — the iced `App`/`Message` types, the window chrome (the borderless custom titlebar's `window:*` actions — drag/resize/minimize/maximize/close — and window-geometry persistence), and the bridge into glacier-ui: registers the single component (`register_component("app", "crates/rustploy-gui/views/app.xml")`) and seeds/persists `Prefs` (remembered login URL/token — the Luau layer has no file I/O of its own, so this round-trips through Rust). Geometry is **queried fresh** (`window::size`/`window::position`) at the moment of closing (`close_and_save`, chained via `Task::then`), not tracked from `Event::Resized`/`Moved` — an earlier event-tracking version reliably saved the window's `min_size` instead of its real size, because an early spurious `Resized` event during the Wayland xdg-shell configure handshake got cached and never overwritten. `window::position` is always `None` on Wayland (the protocol never exposes it to clients — not fixable client-side); size persistence works. `exit_on_close_request(false)` + `Command::CloseRequested` handling means both the titlebar's own close button and an OS/WM-level close request save before actually closing.
- **`app/store.rs`** — local JSON persistence under `shared::fallback_data_dir()`: `Prefs` (remembered login URL/token) and `WindowState` (remembered size/position, see above).
- **`views/`** — `app.xml` (titlebar + resize handles, switches on `screen`), `login.xml`, `shell.xml` (sidebar + topbar, switches on `view`; topbar keeps only the search box, daemon status, Stop All and Disconnect), `home.xml` (Deployments/Projects/Monitoring/Ingress/Docker/Settings views — Docker has Containers/Images/Volumes/Networks sub-tabs, the last three with a "clear unused" button each), `service.xml` (service detail, its own sub-tabs), `new_service.xml` (wizard), `components/*.xml` (reusable fragments). Styled by `views/styles/app.gss` (linked globally from `app.xml` via `<link rel="stylesheet">` — global by design since glacier-ui 0.23, since it holds the classes shared across templates).

**Luau tooling (`luau-lsp`)**: the reactive network/logic layer lives in `views/scripts/*.luau` (packages `fmt/`, `handlers/`, `net/` — see `docs/luau-modularizacao-pacotes.md` for the full module-organization writeup, the `require`-resolution investigation that shaped it, and troubleshooting). Type-check any change with `luau-lsp analyze --base-luaurc=.luaurc --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau <file(s)>` before considering it done — it's not a substitute for `cargo test -p rustploy-gui --test templates_render` (the real `mlua` runtime), but catches module-path/type mistakes in seconds. Install the CLI binary and, for VS Code, the `johnnymorganz.luau-lsp` extension (config already checked in via `.luaurc` + `.vscode/settings.json`) — see `docs/luau-modularizacao-pacotes.md` for exact commands and for why `glacier.d.luau` needs the `--definitions=`/`luau-lsp.types.definitionFiles` treatment (skipping it makes the editor misparse the file as a plain script and raise ~40 false errors).

### Deploy pipeline detail

Git-sourced deploys: clone repo (`git2` in `spawn_blocking` because `!Send`) → build Docker image (tar build context, stream output as `LogLine` events) → create staging container → healthcheck poll (TCP/HTTP/DockerNative) → swap ingress route → drain old container → rename staging → `Live`.  
Registry-sourced deploys skip clone/build and go straight to pull → staging.

Containers are named `rp_<service_name>_live` (production) and `rp_<service_name>_<deploy_id[:8]>_staging` (in-flight). Build artifacts live at `<db_path>/builds/<deployment_id>/` and are deleted on promotion or rollback.
