VERSION := $(shell grep '^version' crates/daemon/Cargo.toml | head -1 | cut -d'"' -f2)

DAEMON_BIN := target/release/rustployd
CLIENT_BIN := target/release/rustploy

BOLD  := \033[1m
RESET := \033[0m
GREEN := \033[32m
CYAN  := \033[36m
RED   := \033[31m

export CARGO_TERM_COLOR := never

.DEFAULT_GOAL := help

# ── Build ──────────────────────────────────────────────────────────────────────

.PHONY: build
build: ## Compila daemon, tui e gui em modo release
	cargo build --release --workspace

.PHONY: check
check: ## Verifica o workspace sem linkar (mais rápido)
	cargo check --workspace

.PHONY: dev-daemon
dev-daemon: ## Roda o daemon em modo debug (para desenvolvimento)
	cargo run -p daemon

.PHONY: dev-tui
dev-tui: ## Roda o TUI tui em modo debug
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

# ── Cross-compile (Windows) ───────────────────────────────────────────────────

WIN_TARGET   := x86_64-pc-windows-msvc
WIN_BIN      := target/$(WIN_TARGET)/release/rustploy-gui.exe

.PHONY: rustploy-gui-windows
rustploy-gui-windows: ## Compila o rustploy-gui para Windows (.exe) via cargo-xwin
	@command -v cargo-xwin >/dev/null 2>&1 || \
		(echo "$(BOLD)Instalando cargo-xwin...$(RESET)" && cargo install cargo-xwin)
	@rustup target list --installed | grep -q '^$(WIN_TARGET)$$' || \
		rustup target add $(WIN_TARGET)
	cargo xwin build --release -p rustploy-gui --target $(WIN_TARGET)
	@echo ""
	@echo "$(GREEN)Executável Windows gerado:$(RESET)"
	@ls -lh $(WIN_BIN)

WIN_DIST_DIR := dist/rustploy-gui-windows
WIN_DIST_ZIP := dist/rustploy-gui-windows.zip

.PHONY: rustploy-gui-windows-dist
rustploy-gui-windows-dist: ## Pacote .zip do rustploy-gui p/ Windows (apaga dist e regera)
	@command -v cargo-xwin >/dev/null 2>&1 || \
		(echo "$(BOLD)Instalando cargo-xwin...$(RESET)" && cargo install cargo-xwin)
	@rustup target list --installed | grep -q '^$(WIN_TARGET)$$' || \
		rustup target add $(WIN_TARGET)
	@command -v zip >/dev/null 2>&1 || \
		(echo "$(BOLD)Instale 'zip' (sudo apt install zip)$(RESET)" && exit 1)
	# CRT estática (+crt-static) para não depender do Visual C++ Redistributable
	# na máquina Windows de destino. O ícone do .exe é embutido pelo build.rs.
	RUSTFLAGS="-C target-feature=+crt-static" \
		cargo xwin build --release -p rustploy-gui --target $(WIN_TARGET)
	@echo "$(BOLD)Apagando dist/ e montando $(WIN_DIST_DIR)...$(RESET)"
	@rm -rf dist
	@mkdir -p $(WIN_DIST_DIR)/crates/rustploy-gui $(WIN_DIST_DIR)/crates/shared/templates
	@cp $(WIN_BIN) $(WIN_DIST_DIR)/
	# Assets lidos em runtime por caminho relativo ao CWD (o exe faz chdir p/ a
	# própria pasta no startup — ver src/assets.rs): mesma estrutura de pastas.
	# `views/` é copiada INTEIRA (templates .xml, views/styles/*.gss+*.json,
	# components/, e TODA a camada Luau — views/scripts/{app.luau,state.luau,
	# helpers.luau,glacier.d.luau,fmt.luau,fmt/,handlers/,net/}): copiar por
	# sub-pasta faz esse target esquecer um pacote novo silenciosamente (foi o
	# bug real corrigido aqui — a versão anterior copiava
	# `crates/rustploy-gui/templates`, renomeada p/ `views/` faz tempo, e nunca
	# pegava `views/scripts/`). Não existe mais um `crates/rustploy-gui/styles/`
	# separado — foi movido para dentro de `views/`.
	@cp -r crates/rustploy-gui/views  $(WIN_DIST_DIR)/crates/rustploy-gui/
	@mkdir -p $(WIN_DIST_DIR)/crates/rustploy-gui/assets
	@cp -r crates/rustploy-gui/assets/icons $(WIN_DIST_DIR)/crates/rustploy-gui/assets/
	@cp -r crates/shared/templates/blueprints $(WIN_DIST_DIR)/crates/shared/templates/
	@printf 'Descompacte e rode rustploy-gui.exe (duplo-clique).\r\n' \
		> $(WIN_DIST_DIR)/LEIA-ME.txt
	@echo "$(BOLD)Conferindo se a camada Luau foi empacotada...$(RESET)"
	@test -f $(WIN_DIST_DIR)/crates/rustploy-gui/views/scripts/app.luau || \
		(echo "$(BOLD)ERRO: views/scripts/app.luau não foi copiado — pacote incompleto$(RESET)" && exit 1)
	@echo "  $$(find $(WIN_DIST_DIR)/crates/rustploy-gui/views/scripts -name '*.luau' | wc -l) arquivos .luau empacotados"
	@test -f $(WIN_DIST_DIR)/crates/rustploy-gui/views/styles/app.gss || \
		(echo "$(BOLD)ERRO: views/styles/app.gss não foi copiado — pacote incompleto$(RESET)" && exit 1)
	@cd dist && zip -qr rustploy-gui-windows.zip rustploy-gui-windows
	@echo ""
	@echo "$(GREEN)Pacote Windows gerado:$(RESET)"
	@ls -lh $(WIN_DIST_ZIP)

# ── Assinatura Windows (Authenticode) ─────────────────────────────────────────
# IMPORTANTE sobre o teto disto: o Smart App Control (Win 11) NÃO é resolvido
# por auto-assinatura. Um cert self-signed só remove avisos em máquinas que
# importarem o .cer no "Trusted Root" + "Trusted Publishers" (cenário de
# time/interno). Para o público, o caminho grátis de verdade é o SignPath
# (tier OSS) rodando no CI — ver .github/workflows/release.yml e
# docs/windows-code-signing.md. Este alvo serve para testes locais/internos.

WIN_SIGN_CERT   := dist/rustploy-selfsign.pfx
WIN_SIGN_CER    := dist/rustploy-selfsign.cer
WIN_SIGN_PASS   := rustploy
WIN_SIGN_TS_URL := http://timestamp.digicert.com

.PHONY: win-selfsign-cert
win-selfsign-cert: ## Gera um certificado self-signed p/ testes (dist/*.pfx e *.cer)
	@command -v openssl >/dev/null 2>&1 || \
		(echo "$(BOLD)Instale openssl$(RESET)" && exit 1)
	@mkdir -p dist
	@if [ -f $(WIN_SIGN_CERT) ]; then \
		echo "$(WIN_SIGN_CERT) já existe — apague-o para regerar."; \
	else \
		echo "$(BOLD)Gerando cert self-signed (code signing)...$(RESET)"; \
		openssl req -x509 -newkey rsa:3072 -keyout dist/_key.pem -out dist/_crt.pem \
			-days 1095 -nodes -subj "/CN=Chiquitos/O=Chiquitos" \
			-addext "extendedKeyUsage=codeSigning" 2>/dev/null; \
		openssl pkcs12 -export -out $(WIN_SIGN_CERT) -inkey dist/_key.pem \
			-in dist/_crt.pem -passout pass:$(WIN_SIGN_PASS); \
		openssl x509 -in dist/_crt.pem -outform DER -out $(WIN_SIGN_CER); \
		rm -f dist/_key.pem dist/_crt.pem; \
		echo "$(GREEN)  $(WIN_SIGN_CERT) (senha: $(WIN_SIGN_PASS))$(RESET)"; \
		echo "$(GREEN)  $(WIN_SIGN_CER)  — importe no Windows p/ o app ser aceito$(RESET)"; \
	fi

.PHONY: rustploy-gui-windows-sign
rustploy-gui-windows-sign: win-selfsign-cert ## Assina o .exe (self-signed) com osslsigncode + timestamp
	@command -v osslsigncode >/dev/null 2>&1 || \
		(echo "$(BOLD)Instale osslsigncode (sudo apt install osslsigncode)$(RESET)" && exit 1)
	@test -f $(WIN_BIN) || \
		(echo "$(BOLD)$(WIN_BIN) não existe — rode 'make rustploy-gui-windows' antes$(RESET)" && exit 1)
	@echo "$(BOLD)Assinando $(WIN_BIN)...$(RESET)"
	osslsigncode sign \
		-pkcs12 $(WIN_SIGN_CERT) -pass $(WIN_SIGN_PASS) \
		-h sha256 -t $(WIN_SIGN_TS_URL) \
		-n "Rustploy GUI" -i https://github.com/antoniofernandodj/rustploy \
		-in  $(WIN_BIN) \
		-out $(WIN_BIN).signed
	@mv $(WIN_BIN).signed $(WIN_BIN)
	@echo "$(GREEN)Assinado. Verificando...$(RESET)"
	@osslsigncode verify $(WIN_BIN) 2>&1 | grep -E 'Signature|Timestamp|CN' || true

.PHONY: deb-gui
deb-gui: ## Pacote .deb do remote-gui p/ Linux (apaga dist e regera)
	@command -v cargo-deb >/dev/null 2>&1 || \
		(echo "$(BOLD)Instalando cargo-deb...$(RESET)" && cargo install cargo-deb)
	@echo "$(BOLD)Apagando dist/ e gerando .deb do rustploy-gui...$(RESET)"
	@rm -rf dist
	@mkdir -p dist
	# --separate-debug-symbols mantém o binário enxuto; assets vão p/
	# /usr/share/rustploy e o .desktop/ícones p/ o desktop (ver Cargo.toml).
	cargo deb -p rustploy-gui -o dist/
	@echo ""
	@echo "$(GREEN)Pacote Linux (.deb) gerado:$(RESET)"
	@ls -lh dist/*.deb

# ── Packaging ─────────────────────────────────────────────────────────────────

.PHONY: deb-daemon
deb-daemon: ## Compila e gera apenas o .deb do daemon
	cargo build --release -p daemon -p fw-helper
	cargo deb -p daemon --no-build
	@echo ""
	@echo "$(GREEN)Pacote gerado:$(RESET)"
	@ls -lh target/debian/rustployd_*.deb


.PHONY: install-daemon
install-daemon: deb-daemon ## Compila, empacota e instala apenas o daemon
	sudo dpkg -i $$(ls target/debian/rustployd_*.deb | tail -1)
	sudo systemctl daemon-reload
	# Explícito (não só via postinst, que silencia falhas com `|| true`):
	# helper de firewall — allow/deny de porta externa via /run/rustploy/fw.sock.
	sudo systemctl enable --now rustployd-fw.socket
	sudo systemctl restart rustployd
	sudo journalctl --rotate
	sudo journalctl --vacuum-time=1s --unit=rustployd
	@echo ""
	@echo "$(BOLD)Status do helper de firewall:$(RESET)"
	@sudo systemctl status rustployd-fw.socket --no-pager -l || true
	@test -S /run/rustploy/fw.sock \
		&& echo "$(GREEN)/run/rustploy/fw.sock ok$(RESET)" \
		|| echo "$(RED)AVISO: /run/rustploy/fw.sock não existe — liberação automática de porta externa vai falhar$(RESET)"

.PHONY: install
install: deb ## Instala os pacotes .deb via dpkg
	sudo dpkg -i $$(ls target/debian/rustployd_*.deb       | tail -1)
	sudo dpkg -i $$(ls target/debian/rustploy_*.deb        | tail -1)
	sudo dpkg -i $$(ls target/debian/rustploy-remote_*.deb | tail -1)
	@command -v update-desktop-database >/dev/null 2>&1 && sudo update-desktop-database /usr/share/applications || true

.PHONY: install-tui
install-tui: ## Instala apenas o tui (requer make deb antes)
	sudo dpkg -i $$(ls target/debian/rustploy_*.deb | tail -1)

.PHONY: install-gui
install-gui: deb-gui ## Compila, empacota e instala o rustploy-gui
	sudo dpkg -i $$(ls ./dist/rustploy-gui_*.deb | tail -1)
	sudo rm -f $$(ls ./dist/rustploy-gui_*.deb | tail -1)
	@command -v update-desktop-database >/dev/null 2>&1 && sudo update-desktop-database /usr/share/applications || true

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
