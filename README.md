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
| Banco           | PostgreSQL separado       | SQLite embarcado (sqlx)     |
| RAM em idle     | 200–600 MB                | < 50 MB (alvo)              |
| TLS             | Let's Encrypt via API     | rustls + ACME embutido      |
| Interface       | Web UI                    | GUI desktop nativa (glacier-ui) |

Um único binário (`rustployd`) substitui o PaaS inteiro. O cliente **`rustploy-gui`** conversa com ele via HTTP/JSON + SSE — não precisa rodar na mesma máquina do daemon.

## Funcionalidades

- Deploy de imagens de registry (`docker.io/user/image:tag`) ou de repositórios Git com Dockerfile
- Pipeline de deploy com zero-downtime: staging → healthcheck → swap → drain → promote
- Rollback automático em caso de falha de healthcheck
- Proxy reverso embutido com roteamento por domínio (atualização instantânea, sem reload)
- Logs em tempo real e gráficos de CPU/RAM por container na interface
- Secrets criptografados em repouso com `age`
- Recovery automático de deploys interrompidos ao reiniciar o daemon
- **Webhooks de CI/CD**: endpoint `POST /webhook/{service_id}/{token}` gerado automaticamente no primeiro deploy; compatível com GitHub, GitLab, Gitea e Docker Hub (veja [`docs/webhooks.md`](docs/webhooks.md))
- **Infra-as-Code**: exporte/importe todos os projetos e serviços como manifesto YAML (+ `.env`) direto pela tela de Settings do `rustploy-gui` — reconciliação declarativa e idempotente. O antigo CLI de arquivo (`rustploy apply -f`/`rustploy export`) vivia no TUI removido e não tem substituto de linha de comando hoje (veja [`docs/infra-as-code.md`](docs/infra-as-code.md))
- **Inventário Docker do host**: lista imagens, volumes e networks de todo o Docker Engine (não só recursos geridos pelo Rustploy), com indicação de uso/não uso e limpeza de não usados (`docker system df` como fonte, sem custo extra de round-trip)

## Cliente

**`rustploy-gui`** (binário `rustploy-gui`, crate `crates/rustploy-gui`) é o único cliente —
construído com o framework próprio `glacier-ui` (UI declarativa em XML → iced). Conecta ao
daemon via **HTTP/JSON + SSE**, não precisa rodar na mesma máquina do daemon.
`cargo run -p rustploy-gui` a partir da raiz do workspace.

Houve um cliente TUI (Ratatui, `crates/client`) via Unix Domain Socket — removido; o daemon
ainda expõe o listener UDS/postcard (ver `CLAUDE.md`), mas nada no repo se conecta a ele hoje.

## Não-objetivos

- Não é substituto do Kubernetes para centenas de containers
- Não gerencia clusters multi-host — foco em single-node
- Não tem Web UI — usa uma GUI desktop nativa (`rustploy-gui`)
- Não suporta builds sem Dockerfile

## Requisitos

- Linux com Docker Engine (`dockerd`) acessível em `/var/run/docker.sock`
- Rust toolchain (edição 2024 — `rustup update stable`)
- (Opcional, só para mexer na camada Luau do `rustploy-gui` em `crates/rustploy-gui/views/scripts/`) `luau-lsp`, para type-check dos `.luau`:
  ```bash
  curl -L https://github.com/JohnnyMorganz/luau-lsp/releases/latest/download/luau-lsp-linux-x86_64.zip -o /tmp/luau-lsp.zip
  unzip -o /tmp/luau-lsp.zip -d ~/.local/bin/
  luau-lsp --version   # ex.: 1.68.1
  ```
  (troque `linux-x86_64` pelo asset certo em [releases](https://github.com/JohnnyMorganz/luau-lsp/releases/latest) para macOS/Windows). Para VS Code, instale também a extensão `johnnymorganz.luau-lsp` — o `.luaurc` e o `.vscode/settings.json` do repo já configuram tudo (incluindo o `glacier.d.luau`, o *definitions file* dos globais injetados pelo motor glacier-ui). Detalhes, comandos de validação e a investigação completa por trás da organização em pacotes (`fmt/`, `handlers/`, `net/`) em `docs/luau-modularizacao-pacotes.md`.

Permissões de sistema não são obrigatórias para desenvolvimento. O daemon detecta se consegue escrever nos paths configurados e faz fallback automático para `~/.local/share/rustploy/` quando necessário.

## Build

```bash
cargo build --release
```

Gera:
- `target/release/rustployd` — o daemon
- `target/release/rustploy-gui` — o cliente desktop (ver [Cliente](#cliente)); em modo dev, `cargo run -p rustploy-gui` a partir da raiz basta — os assets (templates XML, scripts Luau, estilos) são lidos com caminho relativo ao CWD.

Para empacotar o `rustploy-gui` distribuível (binário + assets no mesmo pacote, sem depender do checkout do repo) use os alvos do `Makefile`:

```bash
make deb-gui                    # .deb para Linux (cargo-deb) — dist/*.deb
make rustploy-gui-windows-dist   # .zip portável para Windows (cross via cargo-xwin) — dist/rustploy-gui-windows.zip
```

Os dois embarcam a árvore `views/` inteira (templates + a camada Luau em `views/scripts/`, pacotes `fmt/`/`handlers/`/`net/` — ver `docs/luau-modularizacao-pacotes.md`), `styles/`, ícones e os blueprints de template (`crates/shared/templates/blueprints/`). O release automático (`.github/workflows/release.yml`, disparado por tag `v*`) gera os pacotes (daemon Linux, `.deb` do GUI, `.zip` Windows do GUI).

## Execução

**Produção (com root ou permissões no socket/dir system):**
```bash
./rustployd    # socket em /run/rustploy/rustploy.sock, db em /var/lib/rustploy/db
```

**Desenvolvimento (sem root):**
```bash
./rustployd    # fallback automático para ~/.local/share/rustploy/
```

O daemon tenta o path configurado primeiro; se não tiver permissão de escrita, avisa no log (`WARN socket path not writable, using fallback`) e usa `~/.local/share/rustploy/rustploy.sock`. O banco segue o mesmo critério. Depois de subir o daemon, conecte com `rustploy-gui` (URL + token da API HTTP).

## Configuração

Arquivo TOML carregado de `$RUSTPLOY_CONFIG`, depois `/etc/rustploy/config.toml`, depois `~/.config/rustploy/config.toml`. Se nenhum existir, os defaults abaixo são usados.

```toml
[daemon]
socket_path  = "/run/rustploy/rustploy.sock"
db_path      = "/var/lib/rustploy/db"
log_level    = "info"

[api]                 # HTTP/JSON + SSE (canal do GUI). Serve TAMBÉM, sem token,
port         = 9797   # os webhooks de CI/CD (/webhook/…) e o callback OAuth
bind_address = "127.0.0.1"   # não-loopback exige token
# token      = "…"
# domain     = "rustploy.meusite.com"   # HTTPS automático (ACME) nesta porta

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

Cada transição é persistida no SQLite. Ao reiniciar, deploys interrompidos são retomados ou revertidos conforme o estado encontrado no banco.

## Arquitetura

```tree
crates/
├── shared/     # Command, Event, Response, modelos de domínio, RustployConfig
├── daemon/     # rustployd — API UDS+HTTP, SQLite (sqlx), Docker, ingress, deploy engine
└── rustploy-gui/  # rustploy-gui — único cliente (glacier-ui/XML→iced), fala HTTP
```

`rustploy-gui` fala **HTTP/JSON + SSE** com o daemon (`crates/daemon/src/api/http_api.rs`), porque sua lógica roda em Luau (`fetch`/`sse`), sem acesso a UDS: `POST /api/rpc` (um `Command` por requisição), `GET /api/events` (snapshot completo a cada 2s + eventos do bus, como Server-Sent Events), `GET /api/health`.

O daemon também expõe um listener UDS com payload **postcard** (framing `[u32 LE len][bytes]`, `Rpc(Command)`/`Subscribe`→`Event`) — reaproveita o mesmo `dispatch()`/`Command`/`Response`, só o framing muda. Existia para um cliente TUI (Ratatui) que foi removido; nada no repo se conecta a ele hoje.

## Status

| Fase | Descrição | Status |
|------|-----------|--------|
| 0 | Scaffold do workspace, UDS + Axum + Postcard, TUI base | Concluído |
| 1 | CRUD de projetos/serviços, SQLite, Docker, EventBus | Concluído |
| 2 | Máquina de estados de deploy, healthcheck, recovery | Concluído |
| 3 | IngressController com roteamento por domínio | Concluído |
| 4 | TUI completo (sidebar, projetos, detalhe de serviço, logs, métricas, settings, status do daemon) | Concluído |
| 5 | ACME/TLS automático, gestão de secrets via protocolo | Em andamento¹ |
| 6 | Testes de integração, systemd unit, benchmark de memória | Concluído |

¹ Infraestrutura de criptografia (`age`) implementada em `secrets.rs`; comandos `SecretSet/Get` e integração ACME ainda não expostos no protocolo.

² O cliente TUI (Ratatui, `crates/client`, fases 0 e 4 acima) foi removido depois. `rustploy-gui` é hoje o único cliente.
