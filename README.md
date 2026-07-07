# Rustploy

PaaS single-node escrito em Rust. Alternativa ao Dokploy/Coolify com footprint de memória < 50 MB.

## O problema

Plataformas PaaS auto-hospedadas (Dokploy, Coolify, CapRover) são construídas sobre Node.js/PHP e dependem de Docker Swarm ou Kubernetes. Em VPS modestas (1–2 vCPU, 1–4 GB RAM), o overhead do próprio PaaS consome uma fatia desproporcional dos recursos antes de qualquer aplicação subir.

## A solução

| Dimensão        | Dokploy / Coolify         | Rustploy                    |
|-----------------|---------------------------|-----------------------------|
| Runtime         | Node.js / PHP             | Rust — binário nativo       |
| Orquestrador    | Docker Swarm / K8s        | Daemon próprio              |
| Proxy           | Traefik (processo Go)     | Proxy hyper embutido        |
| Banco           | PostgreSQL separado       | SurrealDB embarcado         |
| RAM em idle     | 200–600 MB                | < 50 MB (alvo)              |
| TLS             | Let's Encrypt via API     | rustls + ACME embutido      |
| Interface       | Web UI                    | TUI (Ratatui)               |

Um único binário (`rustployd`) substitui o PaaS inteiro. O cliente TUI (`rustploy`) conecta via Unix Domain Socket.

## Funcionalidades

- Deploy de imagens de registry (`docker.io/user/image:tag`) ou de repositórios Git com Dockerfile
- Pipeline de deploy com zero-downtime: staging → healthcheck → swap → drain → promote
- Rollback automático em caso de falha de healthcheck
- Proxy reverso embutido com roteamento por domínio (atualização instantânea, sem reload)
- Logs em tempo real e gráficos de CPU/RAM por container no TUI
- Secrets criptografados em repouso com `age`
- Recovery automático de deploys interrompidos ao reiniciar o daemon
- **Webhooks de CI/CD**: endpoint `POST /webhook/{service_id}/{token}` gerado automaticamente no primeiro deploy; compatível com GitHub, GitLab, Gitea e Docker Hub (veja [`docs/webhooks.md`](docs/webhooks.md))
- **Infra-as-Code**: descreva projetos e serviços em `rustploy.yml` versionável e aplique de forma declarativa e idempotente com `rustploy apply -f`; exporte projetos existentes com `rustploy export` (veja [`docs/infra-as-code.md`](docs/infra-as-code.md))
- **Inventário Docker do host**: lista imagens, volumes e networks de todo o Docker Engine (não só recursos geridos pelo Rustploy), com indicação de uso/não uso e limpeza de não usados (`docker system df` como fonte, sem custo extra de round-trip)

## Clientes

Além do TUI (`rustploy`, interface primária), existe um cliente desktop GUI opcional,
**`rustploy-gui`** (binário `rustploy-gui`, crate `crates/rustploy-gui`), construído com
o framework próprio `glacier-ui` (UI declarativa em XML → iced). Conecta ao daemon via
**HTTP** em vez do UDS local — não precisa rodar na mesma máquina do daemon. `cargo run -p rustploy-gui` a partir da raiz do
workspace.

## Não-objetivos

- Não é substituto do Kubernetes para centenas de containers
- Não gerencia clusters multi-host — foco em single-node
- Não tem Web UI — o TUI é a interface primária
- Não suporta builds sem Dockerfile

## Requisitos

- Linux com Docker Engine (`dockerd`) acessível em `/var/run/docker.sock`
- Rust toolchain (edição 2024 — `rustup update stable`)
- Para copiar URLs de webhook no TUI: `wl-clipboard` (Wayland) ou `xclip`/`xsel` (X11)
  ```bash
  # Wayland (Ubuntu/Debian)
  sudo apt install wl-clipboard

  # X11 (Ubuntu/Debian)
  sudo apt install xclip
  ```
- (Opcional, só para mexer na camada Luau do `rustploy-gui` em `crates/rustploy-gui/views/scripts/`) `luau-lsp`, para type-check dos `.luau`:
  ```bash
  curl -L https://github.com/JohnnyMorganz/luau-lsp/releases/latest/download/luau-lsp-linux-x86_64.zip -o /tmp/luau-lsp.zip
  unzip -o /tmp/luau-lsp.zip -d ~/.local/bin/
  luau-lsp --version   # ex.: 1.68.1
  ```
  (troque `linux-x86_64` pelo asset certo em [releases](https://github.com/JohnnyMorganz/luau-lsp/releases/latest) para macOS/Windows). Para VS Code, instale também a extensão `johnnymorganz.luau-lsp` — o `.luaurc` e o `.vscode/settings.json` do repo já configuram tudo (incluindo o `glacier.d.luau`, o *definitions file* dos globais injetados pelo motor glacier-ui). Detalhes, comandos de validação e a investigação completa por trás da organização em pacotes (`fmt/`, `handlers/`, `net/`) em `docs/luau-modularizacao-pacotes.md`.

Permissões de sistema não são obrigatórias para desenvolvimento. O daemon detecta se consegue escrever nos paths configurados e faz fallback automático para `~/.local/share/rustploy/` quando necessário. O cliente segue a mesma lógica ao localizar o socket.

## Build

```bash
cargo build --release
```

Gera:
- `target/release/rustployd` — o daemon
- `target/release/rustploy` — o cliente TUI

## Execução

**Produção (com root ou permissões no socket/dir system):**
```bash
./rustployd    # socket em /run/rustploy/rustploy.sock, db em /var/lib/rustploy/db
./rustploy     # conecta automaticamente ao socket acima
```

**Desenvolvimento (sem root):**
```bash
./rustployd    # fallback automático para ~/.local/share/rustploy/
./rustploy     # detecta o socket via ping, usa o mesmo fallback
```

O daemon tenta o path configurado primeiro; se não tiver permissão de escrita, avisa no log (`WARN socket path not writable, using fallback`) e usa `~/.local/share/rustploy/rustploy.sock`. O banco segue o mesmo critério.

## Configuração

Arquivo TOML carregado de `$RUSTPLOY_CONFIG`, depois `/etc/rustploy/config.toml`, depois `~/.config/rustploy/config.toml`. Se nenhum existir, os defaults abaixo são usados.

```toml
[daemon]
socket_path  = "/run/rustploy/rustploy.sock"
db_path      = "/var/lib/rustploy/db"
log_level    = "info"
webhook_port = 8788   # porta do servidor de webhook de CI/CD + callback OAuth

[ingress]
http_port    = 80
https_port   = 443
bind_address = "0.0.0.0"

[ingress.acme]
enabled   = true
email     = "you@example.com"
directory = "https://acme-v02.api.letsencrypt.org/directory"

[docker]
socket_path = "/var/run/docker.sock"

[deploy]
drain_secs  = 10   # segundos de drenagem antes de destruir o container antigo
image_cache = 2    # versões de imagem antigas mantidas por serviço

[metrics]
interval_secs  = 2
history_points = 60

[secrets]
master_key_path = "/etc/rustploy/master.key"
```

Overrides via variável de ambiente: `RUSTPLOY_SOCKET_PATH`, `RUSTPLOY_DB_PATH`, `RUSTPLOY_LOG_LEVEL`.  
Verbosidade de tracing: `RUST_LOG=daemon=debug`.

## Pipeline de deploy

```
Pending → ResolvingDeps ┬→ PullingImage ──────────────┐
                        └→ CloningRepo → BuildingImage ┘
                                                        ↓
                                                    Staging
                                                        ↓
                                             HealthcheckPolling
                                              ↙            ↘
                                        SwappingIn      RollingBack → Failed
                                            ↓
                                         Draining
                                            ↓
                                         Promoting
                                            ↓
                                           Live
```

Cada transição é persistida no SurrealDB. Ao reiniciar, deploys interrompidos são retomados ou revertidos conforme o estado encontrado no banco.

## Arquitetura

```tree
crates/
├── shared/     # Command, Event, Response, modelos de domínio, RustployConfig
├── daemon/     # rustployd — API Axum/UDS, SQLite (sqlx), Docker, ingress, deploy engine
├── client/     # rustploy — TUI Ratatui (interface primária)
└── rustploy-gui/  # rustploy-gui — cliente GUI opcional (glacier-ui/KDL→iced), fala HTTP
```

Comunicação: HTTP sobre Unix Domain Socket com payload postcard (serialização binária compacta via varint).  
- `POST /rpc` — comandos imperativos (`Command` → `Response`)  
- `GET /stream` — eventos push em tempo real (`Event`, chunked, framing `[u32 LE len][postcard bytes]`)

## Status

| Fase | Descrição | Status |
|------|-----------|--------|
| 0 | Scaffold do workspace, UDS + Axum + Postcard, TUI base | Concluído |
| 1 | CRUD de projetos/serviços, SurrealDB, Docker, EventBus | Concluído |
| 2 | Máquina de estados de deploy, healthcheck, recovery | Concluído |
| 3 | IngressController com roteamento por domínio | Concluído |
| 4 | TUI completo (sidebar, projetos, detalhe de serviço, logs, métricas, settings, status do daemon) | Concluído |
| 5 | ACME/TLS automático, gestão de secrets via protocolo | Em andamento¹ |
| 6 | Testes de integração, systemd unit, benchmark de memória | Concluído |

¹ Infraestrutura de criptografia (`age`) implementada em `secrets.rs`; comandos `SecretSet/Get` e integração ACME ainda não expostos no protocolo.
