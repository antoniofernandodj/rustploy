# Rustploy

Rustploy is a self-hosted platform for deploying and managing applications using Docker Compose. It is designed to be a lightweight, secure, and easy-to-use alternative to other PaaS solutions, specifically tailored for private servers and homelabs.

## Project Overview

- **Core Purpose:** Automate the deployment of containerized services with pre-configured templates.
- **Architecture:** Split into a workspace with three main crates:
  - `client`: A TUI (Terminal User Interface) application for managing deployments and services.
  - `daemon`: The backend agent (`rustployd`) that executes deployments, manages Docker containers, and handles networking/ingress.
  - `shared`: Common models, protocols, and configuration logic used by both client and daemon.
- **Technologies:** Rust, Docker Compose, Ratatui (for the TUI), Axum (API server), SQLite (database), and systemd (service management).

## Key Architecture Components

- **Templates:** Located in `crates/client/src/templates/`. Each service (e.g., WordPress, Ghost) has its own module in `entries/` defining its Docker Compose structure and required variables.
- **Ingress & Proxy:** Managed by the daemon (`crates/daemon/src/ingress/`), handling TLS, routing, and reverse proxying to containers.
- **Deployment Engine:** Located in `crates/daemon/src/deploy/`, handles git operations and execution of Docker Compose commands.

## Development Workflow

### Building and Running

- **Build Workspace:** `make build` (builds both client and daemon in release mode).
- **Check Compilation:** `make check` (fast workspace check).
- **Development Mode:**
  - Run Daemon: `make dev-daemon`
  - Run Client: `make dev-client`
- **Testing:** `make test` (runs all tests in the workspace).
- **Formatting:** `make fmt` (enforces project-wide Rust styling).

### Packaging and Installation

- **Generate DEB Packages:** `make deb` (requires `cargo-deb`).
- **Install Packages:** `make install` (installs both daemon and client via `dpkg`).
- **Systemd Service:** `make start`, `make stop`, `make status` to manage the `rustployd` service.

## Development Conventions

- **Template Style:** `TemplateVar` declarations must be multi-line for readability and consistency with `rustfmt`.
- **Surgical Edits:** Use precise `replace` calls when modifying templates or UI components to maintain structural integrity.
- **Testing:** New features or bug fixes should be accompanied by verification via `make test` or manual TUI testing.
- **Style:** Adhere to the established Ratatui UI patterns in `crates/client/src/ui/`.

## Documentation Reference

- `docs/services.md`: Tracking of implemented and pending service templates.
- `docs/internal-networking.md`: Details on how containers communicate within the Rustploy ecosystem.
- `docs/ingress-proxy.md`: Documentation for the built-in reverse proxy and TLS management.
