VERSION := $(shell grep '^version' crates/daemon/Cargo.toml | head -1 | cut -d'"' -f2)

DAEMON_BIN := target/release/rustployd
CLIENT_BIN := target/release/rustploy

BOLD  := \033[1m
RESET := \033[0m
GREEN := \033[32m
CYAN  := \033[36m

export CARGO_TERM_COLOR := never

.DEFAULT_GOAL := help

# ── Build ──────────────────────────────────────────────────────────────────────

.PHONY: build
build: ## Compila daemon e client em modo release
	cargo build --release --workspace

.PHONY: check
check: ## Verifica o workspace sem linkar (mais rápido)
	cargo check --workspace

.PHONY: dev-daemon
dev-daemon: ## Roda o daemon em modo debug (para desenvolvimento)
	cargo run -p daemon

.PHONY: dev-client
dev-client: ## Roda o TUI client em modo debug
	cargo run -p client

# ── Qualidade ─────────────────────────────────────────────────────────────────

.PHONY: test
test: ## Roda todos os testes do workspace
	cargo test --workspace

.PHONY: fmt
fmt: ## Formata todo o código com rustfmt
	cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Verifica formatação sem modificar arquivos
	cargo fmt --all -- --check

.PHONY: clippy
clippy: ## Roda o clippy em todo o workspace
	cargo clippy --workspace --all-targets -- -D warnings

.PHONY: lint
lint: fmt-check clippy ## fmt-check + clippy

# ── Packaging ─────────────────────────────────────────────────────────────────

.PHONY: deb-daemon
deb-daemon: ## Compila e gera apenas o .deb do daemon
	cargo build --release -p daemon
	cargo deb -p daemon --no-build
	@echo ""
	@echo "$(GREEN)Pacote gerado:$(RESET)"
	@ls -lh target/debian/rustployd_*.deb

.PHONY: deb
deb: build ## Gera os pacotes .deb (daemon + client + remote-client)
	cargo deb -p daemon --no-build
	cargo deb -p client --no-build
	cargo deb -p remote-client --no-build
	@echo ""
	@echo "$(GREEN)Pacotes gerados:$(RESET)"
	@ls -lh target/debian/*.deb

.PHONY: install-daemon
install-daemon: deb-daemon ## Compila, empacota e instala apenas o daemon
	sudo dpkg -i $$(ls target/debian/rustployd_*.deb | tail -1)

.PHONY: install
install: deb ## Instala os pacotes .deb via dpkg
	sudo dpkg -i $$(ls target/debian/rustployd_*.deb       | tail -1)
	sudo dpkg -i $$(ls target/debian/rustploy_*.deb        | tail -1)
	sudo dpkg -i $$(ls target/debian/rustploy-remote_*.deb | tail -1)

.PHONY: install-client
install-client: ## Instala apenas o client (requer make deb antes)
	sudo dpkg -i $$(ls target/debian/rustploy_*.deb | tail -1)

.PHONY: deb-remote-client
deb-remote-client: ## Compila e gera apenas o .deb do remote-client
	cargo build --release -p remote-client
	cargo deb -p remote-client --no-build
	@echo ""
	@echo "$(GREEN)Pacote gerado:$(RESET)"
	@ls -lh target/debian/rustploy-remote_*.deb

.PHONY: install-remote-client
install-remote-client: deb-remote-client ## Compila, empacota e instala o remote-client
	sudo dpkg -i $$(ls target/debian/rustploy-remote_*.deb | tail -1)

.PHONY: uninstall
uninstall: ## Remove os pacotes instalados
	sudo dpkg -r rustployd rustploy || true

.PHONY: reinstall
reinstall: uninstall install ## Remove e reinstala ambos os pacotes

# ── Serviço systemd ───────────────────────────────────────────────────────────

.PHONY: start
start: ## Inicia o serviço rustployd via systemctl
	sudo systemctl start rustployd

.PHONY: stop
stop: ## Para o serviço rustployd
	sudo systemctl stop rustployd

.PHONY: restart
restart: ## Reinicia o serviço rustployd
	sudo systemctl restart rustployd

.PHONY: status
status: ## Exibe o status do serviço rustployd
	systemctl status rustployd

.PHONY: enable
enable: ## Habilita o serviço rustployd para iniciar com o sistema
	sudo systemctl enable rustployd

.PHONY: disable
disable: ## Desabilita o serviço rustployd do boot
	sudo systemctl disable rustployd

.PHONY: logs
logs: ## Segue os logs do daemon via journalctl
	journalctl -fu rustployd

.PHONY: logs-boot
logs-boot: ## Exibe logs desde o último boot
	journalctl -u rustployd -b

# ── Setup ─────────────────────────────────────────────────────────────────────

.PHONY: setup
setup: ## Instala todas as dependências necessárias
	@echo "$(BOLD)==> Verificando rustup / cargo$(RESET)"
	@command -v cargo >/dev/null 2>&1 || \
		(echo "Instalando rustup..." && \
		curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable && \
		. "$$HOME/.cargo/env")
	@echo "$(GREEN)  cargo: $$(cargo --version)$(RESET)"

	@echo "$(BOLD)==> Verificando cargo-deb$(RESET)"
	@cargo deb --version >/dev/null 2>&1 || cargo install cargo-deb
	@echo "$(GREEN)  cargo-deb: $$(cargo deb --version)$(RESET)"

	@echo "$(BOLD)==> Verificando cargo-watch (opcional, para dev)$(RESET)"
	@cargo watch --version >/dev/null 2>&1 || cargo install cargo-watch || \
		echo "  cargo-watch não instalado (opcional)"

	@echo "$(BOLD)==> Verificando wl-clipboard (Wayland clipboard)$(RESET)"
	@command -v wl-copy >/dev/null 2>&1 || \
		(echo "  Instalando wl-clipboard..." && sudo apt-get install -y wl-clipboard)
	@echo "$(GREEN)  wl-copy: $$(which wl-copy)$(RESET)"

	@echo "$(BOLD)==> Verificando Docker$(RESET)"
	@command -v docker >/dev/null 2>&1 || \
		echo "  $(BOLD)AVISO:$(RESET) Docker não encontrado. Instale em https://docs.docker.com/engine/install/"
	@docker info >/dev/null 2>&1 && \
		echo "$(GREEN)  docker: $$(docker --version)$(RESET)" || \
		echo "  Docker instalado mas daemon não está rodando"

	@echo "$(BOLD)==> Verificando dpkg (para install)$(RESET)"
	@command -v dpkg >/dev/null 2>&1 && \
		echo "$(GREEN)  dpkg: OK$(RESET)" || \
		echo "  dpkg não encontrado (necessário para make install)"

	@echo ""
	@echo "$(GREEN)$(BOLD)Setup concluído!$(RESET) Próximos passos:"
	@echo "  make build    — compila em release"
	@echo "  make deb      — gera os .deb"
	@echo "  make install  — instala"

# ── Limpeza ───────────────────────────────────────────────────────────────────

.PHONY: clean
clean: ## Remove artefatos de build (target/)
	cargo clean

.PHONY: clean-deb
clean-deb: ## Remove apenas os .deb gerados
	rm -f target/debian/*.deb

# ── Info ──────────────────────────────────────────────────────────────────────

.PHONY: version
version: ## Exibe a versão atual do projeto
	@echo "$(VERSION)"

.PHONY: help
help: ## Lista todos os targets disponíveis
	@echo ""
	@echo "$(BOLD)Rustploy $(VERSION) — targets disponíveis$(RESET)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ { \
		printf "  $(CYAN)%-18s$(RESET) %s\n", $$1, $$2 \
	}' $(MAKEFILE_LIST)
	@echo ""
