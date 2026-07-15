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

# Run a specific test
cargo test -p daemon test_name
cargo test -p shared

# Check all crates without linking
cargo check --workspace
```

The daemon binary is `rustployd`.

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
| `shared` | — | Models, protocol types, config structs used by the daemon and the GUI |
| `daemon` | `rustployd` | Long-running server: API, DB, Docker, ingress, deploy engine |
| `rustploy-gui` | `rustploy-gui` | glacier-ui (XML→iced) desktop client — the only client, since the TUI (`crates/client`) was removed. Toda a rede/lógica de negócio vive em Luau (`views/scripts/`, pacotes `fmt/`/`handlers/`/`net/` — ver `docs/luau-modularizacao-pacotes.md`), falando com o daemon pela **API HTTP/JSON + SSE** (`crates/daemon/src/api/http_api.rs`), não pelo UDS local. |
| `fw-helper` | `rustployd-fw` | Helper privilegiado de firewall (roda como root via socket activation em `/run/rustploy/fw.sock`). O daemon pede allow/deny de portas externas (`daemon/src/firewall.rs`); o helper só aceita portas dentro da faixa `[external_ports]` da config e só fala com o ufw. Sem dependência da crate `shared`, de propósito. Ver `docs/relatorio-porta-externa-automatica.md`. |

### IPC protocol

`rustploy-gui` is the only client, and speaks plain **HTTP/JSON + SSE** (`crates/daemon/src/api/http_api.rs`: `POST /api/rpc`, `GET /api/events`, `GET /api/health`) — its logic runs in Luau (`fetch`/`sse`), which has no UDS access. `Command`/`Response`/`Event` are defined in `crates/shared/src/protocol.rs` and dispatched via `dispatch()` regardless of transport.

The daemon still exposes a raw **postcard-encoded** Unix Domain Socket listener (`api/server.rs`) — every frame is `[u32 LE length][postcard bytes]`, a `ClientFrame::Rpc(Command)` gets one `Response` frame back, `Subscribe { service_id }` turns the connection into a stream of `Event` frames. This existed for a TUI client (`crates/client`, Ratatui) that was **removed** — nothing in this repo currently connects to the UDS socket. It's left in place (harmless, no upkeep cost) rather than ripped out; touch it only if asked to actually remove UDS support, not as part of routine daemon work.

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

### rustploy-gui internals (`crates/rustploy-gui/src/`)

UI declared in XML templates (`views/*.xml`) and rendered by the published `glacier-ui` crate (see the rule above — never a local `path`/`[patch]` dependency). Every network/business-logic responsibility (login, the SSE consumer, navigation, every mutation) lives in Luau (`views/scripts/`), **not** in this Rust source — `src/` here is only the `iced::daemon` runtime, window chrome, and local persistence (on disk, not the rustploy backend daemon). Run from the workspace root (`cargo run -p rustploy-gui`) or from a packaged layout (see `assets.rs`) — template/script paths are relative to the CWD `glacier-ui` resolves them against, not necessarily the launch directory.

- **`main.rs`** — thin entry point: calls `assets::locate_and_chdir()`, then `app::run()` (the `iced::daemon` runtime). Since glacier-ui 0.36 the app runs on **`iced::daemon`** (multi-window), not `iced::application`.
- **`assets.rs`** — locates the asset base directory at startup and `chdir`s into it, so every CWD-relative reference (Rust-side and inside the XML/Luau themselves) resolves the same way regardless of how the app was launched. Resolution order: `$RUSTPLOY_UI_ASSETS` override → the executable's own directory (portable/Windows `.zip` layout) → `/usr/share/rustploy` (Debian package layout) → current directory as-is (dev run from the workspace root). Probes for `crates/rustploy-gui/views/app.xml` to confirm a candidate base is valid.
- **`app/mod.rs`** — desde glacier-ui **0.38**, apenas **configuração do `GlacierDaemon`**. O runner da lib cuida do loop `iced::daemon`, do motor-por-janela, das janelas-filhas (`open_window(...)` na Luau), dos broadcasts entre elas, dos listeners globais (drag-end, Tab, `@media`) e das ações `window:*` da titlebar borderless (tratadas contra o `Id` da janela em roteamento, e não via `window::latest()` — no Wayland o round-trip perde o pointer-grab serial e `window:drag` vira no-op silencioso). O que é específico do rustploy entra por ganchos do builder: `.font()`/`.default_font()` (JetBrains Mono embutida), `.main_window()` (borderless, ícone, `min_size`, geometria restaurada, `exit_on_close_request: false`), `.child_window()` (as filhas também são borderless), `.main()` (registra `app.xml`, semeia as `Prefs`, define a tela), `.on_message()` (persiste o login lembrado — a camada Luau não tem I/O de arquivo, então o script grava no contexto e o Rust lê o contexto e escreve no disco) e `.on_close()` (persiste a geometria). A geometria chega ao gancho **consultada na hora** pelo runner, não rastreada de eventos `Resized`/`Moved`: no handshake do xdg-shell no Wayland chega um `Resized` espúrio com o `min_size`, e um valor rastreado nasce envenenado com o mínimo. `window::position` é sempre `None` no Wayland (o protocolo não a expõe ao cliente — não é contornável), então só o tamanho é restaurado lá.

  **Histórico:** até a 0.37 este arquivo era um runtime `iced::daemon` **inteiro reimplementado à mão** (~250 linhas duplicando roteamento por janela, listeners globais e abertura de filhas), porque o builder do `GlacierDaemon` não expunha nada disso. Esse buraco de API foi fechado no glacier 0.38 e o runtime local foi removido; se algo aqui parecer faltar, o lugar de consertar é o builder do glacier, não um runtime paralelo aqui.
- **`app/store.rs`** — local JSON persistence under `shared::fallback_data_dir()`: `Prefs` (remembered login URL/token) and `WindowState` (remembered size/position, see above).
- **`views/`** — `app.xml` (titlebar + resize handles, switches on `screen`), `login.xml`, `shell.xml` (sidebar + topbar, switches on `view`; topbar keeps only the search box, daemon status, Stop All and Disconnect), `home.xml` (Deployments/Projects/Monitoring/Ingress/Docker/Settings views — Docker has Containers/Images/Volumes/Networks sub-tabs, the last three with a "clear unused" button each), `service.xml` (service detail, its own sub-tabs), `new_service.xml` (wizard), `new_project_form.xml` (janela **separada** de "Novo projeto" — ver abaixo), `components/*.xml` (reusable fragments). Styled by `views/styles/app.gss` (linked globally from `app.xml` via `<link rel="stylesheet">` — global by design since glacier-ui 0.23, since it holds the classes shared across templates; janelas separadas como `new_project_form.xml` precisam relinká-lo, pois cada janela é um motor isolado).

  **Fluxo multi-janela "Novo projeto"** (glacier-ui 0.37+ IPC entre janelas): o botão "+ Novo projeto" (`shell.xml`, view projects) chama `open_new_project_window` (`handlers/projects.luau`), que faz `open_window({ file = "…/new_project_form.xml", data = { api_url, api_token } })` — a janela nova é um motor Glacier próprio, que recebe a conexão via `data`. Seu script (`views/scripts/new_project_window.luau`) chama `ProjectCreate` e, no sucesso, `broadcast("project_created", {…})` + `close_window()`. O runner do glacier (`GlacierDaemon`) entrega o broadcast à janela principal, cujo `on_broadcast` (`handlers/projects.luau`) faz `Stream.refresh_now()` (o card novo aparece na hora) e um toast. O antigo formulário inline foi removido; `create_project` continua no handler (reutilizável), sem call-site na UI.

**Luau tooling (`luau-lsp`)**: the reactive network/logic layer lives in `views/scripts/*.luau` (packages `fmt/`, `handlers/`, `net/` — see `docs/luau-modularizacao-pacotes.md` for the full module-organization writeup, the `require`-resolution investigation that shaped it, and troubleshooting). Type-check any change with `luau-lsp analyze --base-luaurc=.luaurc --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau <file(s)>` before considering it done — it's not a substitute for `cargo test -p rustploy-gui --test templates_render` (the real `mlua` runtime), but catches module-path/type mistakes in seconds. Install the CLI binary and, for VS Code, the `johnnymorganz.luau-lsp` extension (config already checked in via `.luaurc` + `.vscode/settings.json`) — see `docs/luau-modularizacao-pacotes.md` for exact commands and for why `glacier.d.luau` needs the `--definitions=`/`luau-lsp.types.definitionFiles` treatment (skipping it makes the editor misparse the file as a plain script and raise ~40 false errors).

**Convenção de sintaxe (Luau)**: quando o **único** argumento de uma chamada é um table literal, use a forma sem parênteses — `f{ ... }`, não `f({ ... })` (idem `toast{...}`, `api:rpc_checked{...}`, `open_window{...}`, `json.array{}`, `os.time{...}`, `ipairs{...}`). Isso vale **só** para o único-argumento-table: chamadas com mais de um argumento (`prune({...}, "msg")`, `setmetatable({...}, mt)`) mantêm os parênteses. String literal única também poderia dispensar parênteses, mas **não** adotamos essa forma — só a de table.

### Deploy pipeline detail

Git-sourced deploys: clone repo (`git2` in `spawn_blocking` because `!Send`) → build Docker image (tar build context, stream output as `LogLine` events) → create staging container → healthcheck poll (TCP/HTTP/DockerNative) → swap ingress route → drain old container → rename staging → `Live`.  
Registry-sourced deploys skip clone/build and go straight to pull → staging.

Containers are named `rp_<service_name>_live` (production) and `rp_<service_name>_<deploy_id[:8]>_staging` (in-flight). Build artifacts live at `<db_path>/builds/<deployment_id>/` and are deleted on promotion or rollback.
