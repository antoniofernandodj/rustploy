# Rustploy

Rustploy is a self-hosted, single-node PaaS written in Rust. It is designed to be a lightweight, secure, and easy-to-use alternative to other PaaS solutions (Dokploy/Coolify), specifically tailored for private servers and homelabs.

## Project Overview

- **Core Purpose:** Automate zero-downtime deployment of containerized services (Git+Dockerfile or registry images), with a built-in reverse proxy, ACME/TLS, and an embedded Docker OCI registry.
- **Architecture:** Workspace with these crates:
  - `daemon`: The backend agent (`rustployd`) that executes deployments, manages Docker containers, ingress, and the embedded registry.
  - `shared`: Common models, protocols, and configuration logic used across the workspace.
  - `rustploy-gui`: The desktop client (`glacier-ui`: XML templates → iced), talking to the daemon over HTTP/JSON + SSE. Its business logic lives in Luau scripts (`views/scripts/`), not in Rust.
  - `fw-helper`: Privileged firewall helper (`rustployd-fw`), talks to the daemon over a separate root-owned socket.
  - `importer`: Data import from other platforms (e.g. Dokploy).
- **Technologies:** Rust, bollard (Docker Engine API), hyper (reverse proxy + embedded registry), SQLite via `sqlx`, rustls + ACME, systemd (service management), Luau (GUI logic).

There used to be a fourth crate, `client` — a Ratatui TUI — which has since been removed. `rustploy-gui` is the only client today.

## Key Architecture Components

- **Templates/Blueprints:** Located in `crates/shared/templates/blueprints/` (Dokploy-compatible format), compiled in via `build.rs`. Each app catalog entry defines its Docker Compose structure, generators, and required env vars.
- **Ingress & Proxy:** Managed by the daemon (`crates/daemon/src/ingress/`), handling TLS/ACME, domain routing (`arc-swap` route table), and reverse proxying to containers.
- **Deployment Engine:** `crates/daemon/src/deploy/executor.rs` drives the deploy state machine (clone/pull → build → stage → healthcheck → swap → drain → promote).
- **Embedded Registry:** `crates/daemon/src/registry/` implements OCI Distribution Spec v2 directly in Rust (no `registry:2` container) — push/pull, GC, Basic auth by token, optional public exposure via ingress/ACME.

## Development Workflow

### Building and Running

- **Build Workspace:** `make build` (builds daemon and gui in release mode).
- **Check Compilation:** `make check` (fast workspace check).
- **Development Mode:**
  - Run Daemon: `make dev-daemon` (or `cargo run -p daemon`)
  - Run GUI: `cargo run -p rustploy-gui`
- **Testing:** `make test` (runs all tests in the workspace).
- **Formatting:** `make fmt` (enforces project-wide Rust styling).

### Packaging and Installation

- **Generate DEB Packages:** `make deb-daemon` / `make deb-gui` (requires `cargo-deb`).
- **Install Packages:** `make install-daemon`, `make install-gui`.
- **Systemd Service:** `make start`, `make stop`, `make status` to manage the `rustployd` service.

## Development Conventions

- **`glacier-ui` dependency:** `rustploy-gui` consumes `glacier-ui` from crates.io (pinned version), never a local `path`/`[patch]` — changes to `glacier-ui` require publishing a new version there first.
- **Luau logic:** The reactive network/business layer lives in `crates/rustploy-gui/views/scripts/*.luau` (packages `fmt/`, `handlers/`, `net/`). Type-check with `luau-lsp analyze` before considering a change done.
- **Postcard wire safety:** `Command`/`Response`/`Event` enum variants (protocol.rs) are positional on the wire — always append new fields/variants at the end, never insert in the middle, never use `skip_serializing_if`/serde defaults.
- **Surgical Edits:** Use precise, minimal edits when modifying templates or UI components to maintain structural integrity.
- **Testing:** New features or bug fixes should be accompanied by verification via `make test` and, for GUI changes, manual testing in a running window (`cargo run -p rustploy-gui`) — a green test suite doesn't confirm a UI feature actually renders/behaves correctly.

## Documentation Reference

See `CLAUDE.md` for the actively maintained, detailed architecture reference. `AGENTS.md` is an older, largely superseded technical spec (predates the SQLite migration, the TUI removal, and several features) — kept for historical context.

- `docs/services.md`: Tracking of implemented and pending service templates.
- `docs/migration.md`: Instructions for importing data from other platforms (Dokploy).
- `docs/internal-networking.md`: Details on how containers communicate within the Rustploy ecosystem.
- `docs/ingress-proxy.md`: Documentation for the built-in reverse proxy and TLS management.
