# Rustploy — Especificação Técnica Completa

> PaaS de baixo consumo escrito em Rust, sem orquestrador externo.  
> Alternativa ao Dokploy/Coolify com footprint de memória de serviço < 50 MB.

---

## 1. Visão e Motivação

### 1.1 O Problema

Plataformas PaaS auto-hospedadas existentes (Dokploy, Coolify, CapRover) compartilham um problema estrutural: são construídas sobre Node.js/PHP e dependem de Docker Swarm ou Kubernetes para orquestração. Para projetos de baixo tráfego em VPS modestas (1-2 vCPU, 1-4 GB RAM), o overhead do próprio PaaS consome uma fatia desproporcional dos recursos disponíveis antes de qualquer aplicação do usuário sequer subir.

### 1.2 A Solução

Rustploy é um daemon único que:

- Compila para um binário estático < 15 MB
- Consome < 30 MB de RAM em idle
- Gerencia o ciclo completo de deploy sem processos externos além do `dockerd`
- Expõe um proxy reverso embutido (Pingora) que se atualiza em tempo real sem reload de arquivos
- Persiste estado em SurrealDB embarcado (zero processo de banco separado)

### 1.3 Posicionamento

| Dimensão           | Dokploy/Coolify        | Rustploy               |
|--------------------|------------------------|------------------------|
| Runtime            | Node.js / PHP          | Rust (binário nativo)  |
| Orquestrador       | Docker Swarm / K8s     | Daemon próprio         |
| Proxy              | Traefik (processo Go)  | Pingora (lib Rust)     |
| Banco              | PostgreSQL separado    | SurrealDB embarcado    |
| RAM em idle        | 200–600 MB             | < 50 MB (alvo)         |
| TLS                | Let's Encrypt via API  | rustls + ACME embutido |
| Interface          | Web UI                 | TUI (Ratatui)          |

### 1.4 Não-Objetivos (explícitos)

- **Não é um substituto do Kubernetes** para workloads com centenas de containers
- **Não gerencia clusters multi-host** — foco em single-node
- **Não tem Web UI** no escopo inicial — o TUI é a interface primária
- **Não suporta build de imagens arbitrárias** — apenas repositórios Git com Dockerfile; imagens pré-construídas em registry também são suportadas
- **Não implementa service mesh** — isolamento de rede via bridge Docker é suficiente

---

## 2. Arquitetura do Sistema

### 2.1 Visão Geral de Componentes

```
┌─────────────────────────────────────────────────────────────────────┐
│  Host Linux                                                         │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  rustployd  (binário único)                                 │    │
│  │                                                             │    │
│  │  ┌──────────────┐   ┌──────────────┐   ┌──────────────────┐ │    │
│  │  │   Pingora    │   │    Daemon    │   │   SurrealDB      │ │    │
│  │  │   Ingress    │◄─►│    Core      │◄─►│   (embarcado)    │ │    │
│  │  │  :80 / :443  │   │              │   │   RocksDB/SpeeDB │ │    │
│  │  └──────────────┘   └──────┬───────┘   └──────────────────┘ │    │
│  │                            │                                │    │
│  │              Unix Domain Socket                             │    │
│  │              /run/rustploy/rustploy.sock                    │    │
│  └────────────────────────────┼────────────────────────────────┘    │
│                               │                                     │
│  ┌────────────────────────────▼──────────────────────────────────┐  │
│  │  rustploy (TUI client)                                        │  │
│  │  - Dashboard de projetos / serviços                           │  │
│  │  - Stream de logs em tempo real                               │  │
│  │  - Gráficos de CPU/RAM por container                          │  │
│  └───────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  Docker Engine  /var/run/docker.sock                          │  │
│  │  Containers gerenciados por projeto                           │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Estrutura do Workspace Rust

```
rustploy/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── shared/                 # tipos compartilhados, protocolo
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs     # Command, Event, Response enums
│   │       ├── models.rs       # Project, Service, Deployment, etc.
│   │       └── config.rs       # RustployConfig struct
│   │
│   ├── daemon/                 # rustployd — processo principal
│   │   └── src/
│   │       ├── main.rs
│   │       ├── api/            # handlers Axum (UDS)
│   │       │   ├── mod.rs
│   │       │   ├── projects.rs
│   │       │   ├── services.rs
│   │       │   ├── deployments.rs
│   │       │   └── stream.rs   # SSE-over-UDS para eventos
│   │       ├── db/             # camada SurrealDB
│   │       │   ├── mod.rs
│   │       │   ├── projects.rs
│   │       │   ├── services.rs
│   │       │   └── deployments.rs
│   │       ├── docker/         # wrapper bollard
│   │       │   ├── mod.rs
│   │       │   ├── images.rs
│   │       │   ├── containers.rs
│   │       │   ├── networks.rs
│   │       │   └── events.rs
│   │       ├── deploy/         # máquina de estados de deploy
│   │       │   ├── mod.rs
│   │       │   ├── state.rs    # enum DeployState
│   │       │   ├── executor.rs # lógica de transições
│   │       │   └── recovery.rs # recuperação após crash do daemon
│   │       ├── ingress/        # integração Pingora
│   │       │   ├── mod.rs
│   │       │   ├── router.rs   # tabela de rotas em memória
│   │       │   ├── tls.rs      # gestão ACME / rustls
│   │       │   └── proxy.rs    # ProxyHttp impl
│   │       └── metrics.rs      # coleta CPU/RAM dos containers
│   │
│   └── client/                 # rustploy — TUI
│       └── src/
│           ├── main.rs
│           ├── transport.rs    # cliente UDS + Bincode
│           ├── app.rs          # estado global da TUI
│           ├── events.rs       # loop de eventos (crossterm + UDS stream)
│           └── ui/
│               ├── mod.rs
│               ├── dashboard.rs
│               ├── service_detail.rs
│               ├── deploy_log.rs
│               └── metrics.rs
```

---

## 3. Crate `shared` — Protocolo e Modelos

### 3.1 Protocolo de Comunicação

O canal de comunicação entre `client` e `daemon` é um Unix Domain Socket em `/run/rustploy/rustploy.sock`. Dois padrões de uso:

1. **Request/Response** — comandos imperativos, serializados em Bincode sobre HTTP/1.1 via Axum
2. **Event Stream** — eventos push do daemon para o client, via chunked transfer encoding (um evento Bincode por chunk)

#### Framing de eventos (stream)

```
[4 bytes: tamanho do payload u32 LE][payload: Bincode<Event>]
```

O client lê o tamanho, aloca exatamente aquele buffer, desserializa. Isso evita parsing de linha e mantém CPU mínimo.

### 3.2 Comandos (client → daemon)

Os comandos são agrupados por domínio:

**Projetos:** `ProjectCreate` (nome + descrição opcional), `ProjectDelete` (por id), `ProjectList`.

**Serviços:** `ServiceCreate` (recebe uma `ServiceSpec` completa), `ServiceUpdate` (id + nova spec), `ServiceDelete` (por id), `ServiceList` (filtrado por projeto).

**Deployments:** `DeployStart` (por service_id), `DeployAbort` (por deployment_id), `DeployRollback` (por service_id, volta à versão anterior), `DeployHistory` (por service_id, com limite de resultados).

**Observabilidade:** `LogsSubscribe`/`LogsUnsubscribe` (por service_id, com quantidade de linhas retroativas), `MetricsSubscribe`/`MetricsUnsubscribe` (por service_id).

**Infraestrutura:** `Ping` (verifica se o daemon responde), `DaemonStatus` (informações gerais do daemon).

### 3.3 Eventos (daemon → client, stream)

**`DeployStateChanged`** — emitido a cada transição de estado; carrega o `deployment_id`, `service_id`, novo estado, timestamp e mensagem opcional.

**`DeployProgress`** — granularidade fina dentro de estados longos (ex: progresso por camada durante `PullingImage`); carrega a fase, percentual (0–100) e descrição textual.

**`LogLine`** — uma linha capturada do container (stdout ou stderr), com timestamp e identificação do serviço e container.

**`ContainerMetrics`** — snapshot de CPU%, memória usada e limite, bytes de rede recebidos/transmitidos e timestamp; emitido a cada ciclo de coleta.

**`ServiceStatusChanged`** — mudança de alto nível no status de um serviço (Stopped, Deploying, Running, Degraded, Error).

**`DaemonReady`** — emitido após o daemon inicializar completamente, com a versão do binário.

**`Error`** — erros assíncronos com código estruturado e mensagem descritiva.

### 3.4 Respostas (daemon → client, request/response)

As respostas são um tipo union que cobre todos os casos possíveis: `Ok` (confirmação sem dado), `Project`, `Projects`, `Service`, `Services`, `Deployment`, `Deployments`, `DaemonStatus` (informações gerais), `Pong` (uptime em segundos) e `Err` (erro estruturado com código e mensagem).

### 3.5 Modelos de Dados

Todos os identificadores são ULIDs — identificadores lexicograficamente ordenados que garantem unicidade sem coordenação central.

**Project** — campos: `id`, `name` (único), `description` (opcional), `created_at`.

**ServiceSpec** — especificação imutável de um serviço: `name`, `project_id`, `source` (origem da imagem — veja abaixo), `port` (porta interna do container), `domain` (domínio público), `env_vars`, `volumes`, `healthcheck`, `replicas` (fixo em 1 na v1), `resources`.

**ServiceSource** — define como a imagem do serviço é obtida; duas variantes mutuamente exclusivas:

- `Registry { image }` — imagem já publicada em um registry (ex: `ghcr.io/user/app:latest`); o daemon faz pull diretamente
- `Git { url, branch, dockerfile_path, build_context, credentials }` — repositório Git com Dockerfile; o daemon clona o repositório, constrói a imagem localmente via API do Docker Engine e a usa para o deploy

**GitSource** — campos de uma origem Git: `url` (HTTPS ou SSH), `branch` (ou commit SHA), `dockerfile_path` (caminho do Dockerfile dentro do repo, padrão `Dockerfile`), `build_context` (caminho do contexto de build dentro do repo, padrão `.`), `credentials` (referência a um secret do projeto com o token de acesso ou chave SSH, opcional para repositórios públicos).

**Service** — agrega uma `ServiceSpec` com estado operacional: `id`, `spec`, `status`, `live_container_id` (ID do container ativo no Docker), `created_at`, `updated_at`.

**ServiceStatus** — enum de estado: `Stopped`, `Deploying`, `Running`, `Degraded`, `Error(mensagem)`.

**Deployment** — representa uma tentativa de deploy: `id`, `service_id`, `image`, `state` (estado atual na máquina de estados), `states_log` (histórico completo de transições com timestamps), `started_at`, `finished_at` (opcional).

**StateTransition** — um registro no log: `from`, `to`, `at` (timestamp), `message` (opcional).

**EnvVar** — par chave + valor, onde o valor pode ser `Plain(texto)` ou `Secret(nome_do_secret)` — neste caso, o daemon resolve e decriptografa na hora de criar o container.

**VolumeMount** — `host_path`, `container_path`, `read_only`.

**Healthcheck** — `kind` (HTTP, TCP ou DockerNative), `interval_secs`, `timeout_secs`, `retries`, `start_period_secs`.

**HealthcheckKind** — `Http` (path + status HTTP esperado), `Tcp` (apenas verifica conexão na porta), `DockerNative` (delega ao HEALTHCHECK da imagem).

**ResourceLimits** — `cpu_shares` (relativo; 1024 = 1 CPU inteiro), `mem_limit_bytes` (0 = sem limite).

---

## 4. Máquina de Estados do Deploy

### 4.1 Estados e Transições

```
                         ┌─────────┐
                    ─────► Pending │
                         └────┬────┘
                              │ dependências OK
                    ┌─────────▼──────────┐
                    │   ResolvingDeps    │
                    └──┬────────────┬────┘
                       │            │ rede OK, secrets OK
             source=Registry   source=Git
                       │            │
              ┌────────▼───┐  ┌──────▼──────────┐
              │PullingImage│  │  CloningRepo    │◄── progresso via DeployProgress
              └────────┬───┘  └──────┬──────────┘
                       │             │ repo clonado
                       │     ┌───────▼──────────┐
                       │     │  BuildingImage   │◄── log de build via LogLine
                       │     └───────┬──────────┘
                       │             │ imagem construída
                       └───────┬─────┘
                               │ imagem disponível localmente
                    ┌──────────▼─────────┐
                    │     Staging        │  cria container N+1 (sem tráfego)
                    └─────────┬──────────┘
                              │ container criado e iniciado
                    ┌─────────▼──────────────┐
                    │ HealthcheckPolling     │  loop até pass ou timeout
                    └────┬───────────────────┘
                    pass │          │ fail / timeout
               ┌─────────▼───┐   ┌──▼──────────┐
               │ SwappingIn  │   │ RollingBack │
               └─────────┬───┘   └──┬──────────┘
                         │          │ tráfego devolvido ao container antigo
               ┌─────────▼───┐   ┌──▼──────────┐
               │  Draining   │   │   Failed    │◄── estado terminal
               └─────────┬───┘   └─────────────┘
                         │ drain_secs decorridos
               ┌─────────▼───┐
               │  Promoting  │  renomeia container, atualiza SurrealDB
               └─────────┬───┘
                         │
               ┌─────────▼──┐
               │    Live    │◄── estado terminal (sucesso)
               └────────────┘
                         │ próximo deploy iniciado
               ┌─────────▼───┐
               │   Pruning   │  remove container antigo e imagens órfãs
               └─────────────┘
```

### 4.2 Persistência de Estado

Cada transição de estado é uma transação ACID no SurrealDB. Ao criar um deployment, o banco registra `id`, `service_id`, `image`, estado inicial `Pending`, log de transições vazio e `started_at`. A cada transição, o campo `state` é atualizado e um objeto com `{from, to, at, message}` é anexado ao array `states_log`.

**Invariante de recuperação**: ao iniciar, o daemon executa uma query por todos os deployments cujo estado não seja `Live`, `Failed`, `Pruning`. Para cada um, a lógica de recovery é chamada e o deploy é retomado ou abortado com rollback, dependendo do estado encontrado.

### 4.3 Lógica do Executor

O executor de deploy opera em loop: lê o estado atual do deployment no banco, executa a ação correspondente ao estado, persiste a transição para o próximo estado e repete até atingir um estado terminal (`Live` ou `Failed`). Qualquer erro em qualquer step dispara automaticamente a transição para `RollingBack`.

O mapeamento de estado para ação é:

| Estado atual         | Ação executada                                                        |
|----------------------|-----------------------------------------------------------------------|
| `Pending`            | Verificar dependências (rede, secrets, credenciais Git se aplicável)  |
| `ResolvingDeps`      | Ramificar: `PullingImage` (Registry) ou `CloningRepo` (Git)          |
| `PullingImage`       | Criar e iniciar container de staging                                  |
| `CloningRepo`        | Iniciar build da imagem (`BuildingImage`)                             |
| `BuildingImage`      | Criar e iniciar container de staging                                  |
| `Staging`            | Iniciar loop de healthcheck                                           |
| `HealthcheckPolling` | Sinalizar Pingora para iniciar o swap                                 |
| `SwappingIn`         | Aguardar `drain_secs` com container antigo sem tráfego                |
| `Draining`           | Renomear container e atualizar banco                                  |
| `Promoting`          | Marcar como `Live`                                                    |
| `RollingBack`        | Reverter tráfego e destruir container de staging                      |

---

## 5. Crate `daemon` — Subsistemas

### 5.1 Subsistema Docker (`crates/daemon/src/docker/`)

Wrapper sobre a biblioteca de acesso à API do Docker Engine que encapsula todas as interações com o `dockerd`:

#### 5.1.1 Gestão de Imagens

O gerenciador de imagens expõe operações para os dois caminhos de deploy:

**Caminho Registry:**
- **pull** — faz o download da imagem em streaming, emitindo um evento de progresso por camada recebida via EventBus; permite ao TUI mostrar progresso real de download
- **exists** — verifica se a imagem já está disponível localmente antes de tentar o pull

**Caminho Git:**
- **clone_repo** — clona o repositório Git no diretório temporário de trabalho do daemon; suporta HTTPS (com token) e SSH (com chave privada referenciada via secret); emite eventos de progresso via EventBus
- **build_image** — invoca a API de build do Docker Engine apontando para o diretório clonado, usando o `dockerfile_path` e `build_context` configurados; a saída do build (stdout do `docker build`) é capturada linha a linha e emitida como eventos `LogLine` para o TUI em tempo real; ao terminar, a imagem é tagueada com `rp_{service_name}:{deployment_id_short}`

**Compartilhadas:**
- **prune_unused** — remove imagens que não são referenciadas por nenhum container gerenciado pelo Rustploy, respeitando a configuração de `image_cache` (número de versões antigas a manter)

#### 5.1.2 Gestão de Containers

Convenção de nomenclatura:
- Container ativo: `rp_{service_name}`
- Container em staging: `rp_{service_name}_staging_{deployment_id_short}`

O gerenciador de containers expõe: `create_staging` (cria o container N+1 com configurações completas e retorna o container_id), `start`, `stop_graceful` (SIGTERM com timeout antes de SIGKILL), `rename`, `remove` e `inspect`.

A criação do container de staging sempre inclui:
- `network_mode`: a rede bridge do projeto (`rp_net_{project_id_short}`)
- `labels`: `rustploy.managed=true`, `rustploy.service_id={id}`, `rustploy.deployment_id={id}`
- `restart_policy`: `none` durante staging (o daemon controla o ciclo de vida)
- `host_config.memory`: do `ResourceLimits`
- `host_config.cpu_shares`: do `ResourceLimits`

#### 5.1.3 Gestão de Redes

Cada projeto tem uma rede bridge isolada. Containers do mesmo projeto se veem pelo nome (`rp_{service_name}`), mas o mundo externo só os acessa via Pingora.

O gerenciador de redes expõe: `ensure_project_network` (cria a rede se não existir e retorna o network_id), `remove_project_network`, `connect_container` e `disconnect_container`.

#### 5.1.4 Healthcheck Polling

O daemon implementa seu próprio healthcheck polling em vez de depender do healthcheck nativo do Docker, porque:

1. O healthcheck do Docker tem resolução de intervalo grosseira
2. Precisamos detectar o "ready" em tempo real para minimizar o downtime da janela de swap

O polling opera em loop até o número máximo de tentativas configurado. A cada tentativa:

1. Inspeciona o estado do container no Docker Engine — se o container tiver parado, aborta imediatamente com erro
2. Executa a verificação conforme o modo configurado:
   - **HTTP**: resolve o IP do container na rede do projeto, faz uma requisição GET ao path configurado e compara o status HTTP retornado com o esperado
   - **TCP**: tenta estabelecer uma conexão TCP no IP e porta do container
   - **DockerNative**: lê o campo `health.status` da inspeção do container e verifica se é `"healthy"`
3. Se passou, retorna sucesso; caso contrário, aguarda `interval_secs` e tenta novamente

### 5.2 Integração com Repositórios Git

#### 5.2.1 Provedores Suportados

O daemon suporta qualquer repositório Git acessível via HTTPS ou SSH, o que inclui nativamente:

| Provedor | HTTPS | SSH | Autenticação               |
|----------|-------|-----|----------------------------|
| GitHub   | Sim   | Sim | Personal Access Token / Deploy Key |
| GitLab   | Sim   | Sim | Project Access Token / Deploy Key  |
| Gitea    | Sim   | Sim | API Token / Deploy Key             |
| Git puro | Sim   | Sim | Credencial HTTP / Chave SSH        |

Para repositórios **públicos**, nenhuma credencial é necessária. Para repositórios **privados**, o usuário cadastra o token ou a chave SSH como um secret do projeto, e a `GitSource` referencia esse secret pelo nome.

#### 5.2.2 Fluxo de Clone e Build

1. **Clone** — o daemon cria um diretório temporário em `{db_path}/builds/{deployment_id}`, executa o clone do `url` no `branch` configurado e, em seguida, faz checkout do commit exato para garantir reprodutibilidade. Progresso (contagem de objetos, compressão, recebimento) é emitido como `DeployProgress`.

2. **Build** — o daemon chama a API de build do Docker Engine apontando para `{clone_dir}/{build_context}` como contexto e `{clone_dir}/{dockerfile_path}` como Dockerfile. Cada linha de saída do build (`Step 1/8`, `RUN apt-get install`, etc.) é emitida como evento `LogLine` para o TUI exibir em tempo real.

3. **Tag** — ao concluir o build, a imagem recebe a tag `rp_{service_name}:{deployment_id_short}` para rastreamento. O caminho segue então para `Staging` identicamente ao fluxo de registry.

4. **Limpeza** — o diretório temporário de clone é removido após o build (com ou sem sucesso).

#### 5.2.3 Auto-deploy por Webhook (v2)

Na v2, o daemon poderá expor endpoints de webhook por serviço:

```
POST /webhooks/{service_id}/github
POST /webhooks/{service_id}/gitlab
POST /webhooks/{service_id}/gitea
```

Ao receber um evento de push no branch configurado, o daemon dispara automaticamente um novo deploy. A verificação de assinatura HMAC do payload (com secret configurável) garante que apenas o provedor legítimo pode acionar o webhook. Na v1, re-deploy é sempre iniciado manualmente via TUI.

### 5.3 Subsistema de Ingress — Pingora (`crates/daemon/src/ingress/`)

#### 5.3.1 Tabela de Rotas

A tabela de rotas é um mapa de domínio para entrada de roteamento, mantida em memória com acesso de leitura lock-free via ponteiro atômico (`ArcSwap`). Isso garante que o hot path de cada requisição HTTP nunca bloqueia para adquirir um lock, independentemente da frequência de atualizações de deploy.

Cada entrada de roteamento contém: `domain`, `backend_addr` (IP interno do container + porta, ex: `172.20.0.3:8080`), `service_id` e `tls_cert` (opcional).

O `IngressController` expõe duas operações atômicas:
- **upsert_route** — chamado pelo executor após o estado `Promoting`; substitui ou insere a entrada de roteamento para o domínio de forma imediatamente visível para novas requisições
- **remove_route** — chamado ao remover um serviço

#### 5.3.2 Lógica de Proxy

A cada requisição recebida pelo Pingora, o proxy extrai o header `Host`, consulta a tabela de rotas pelo domínio e encaminha a requisição para o `backend_addr` correspondente. Se não houver rota para o domínio, retorna HTTP 404. Toda essa lógica é executada sem locks, usando apenas a leitura atômica do ponteiro da tabela.

#### 5.3.3 TLS e ACME

Pingora usa `rustls` nativamente. O gerenciamento de certificados segue este fluxo:

1. **Primeiro deploy de um domínio**: daemon inicia desafio ACME HTTP-01 via `instant-acme`
2. O Pingora expõe o endpoint `/.well-known/acme-challenge/` temporariamente via rota especial
3. Após validação, o certificado é armazenado no SurrealDB (serializado como PEM)
4. O `IngressController` carrega o certificado no `RouteEntry`
5. Renovação automática via cron interno (verifica expiração a cada 12h, renova com > 30 dias de antecedência)

O `TlsManager` expõe duas operações: `ensure_cert` (obtém certificado via ACME se não existir ou estiver expirado) e `renew_expiring` (varre o banco por certificados próximos do vencimento e os renova, retornando os domínios renovados).

### 5.4 EventBus — Canal de Eventos Internos

O `EventBus` é o mecanismo de desacoplamento interno do daemon. Qualquer subsistema publica eventos sem saber quem os consumirá. Internamente usa um canal de broadcast: múltiplos subscribers (um por conexão de client TUI) recebem todos os eventos e filtram pelo `service_id` relevante antes de encaminhar.

As operações são: `publish` (envia um evento para todos os subscribers; se o canal estiver cheio, o evento é descartado silenciosamente — jamais bloqueia o produtor) e `subscribe` (retorna um receiver independente para um novo client).

O handler de stream da API cria um subscriber por conexão, filtra eventos pelo `service_id` solicitado (ou encaminha todos se `service_id` for nulo) e serializa cada evento com o framing `[u32 LE tamanho][payload Bincode]` antes de escrever no socket.

### 5.5 Coleta de Métricas

Uma task assíncrona em background consulta a API de estatísticas do Docker Engine periodicamente (padrão: a cada 2 segundos) para cada container em estado `Running`. Para cada container, coleta:

- **CPU%** — calculado a partir dos contadores de ciclos do cgroup delta entre duas leituras consecutivas
- **Memória** — bytes usados e limite configurado
- **Rede** — bytes recebidos e transmitidos acumulados na interface de rede do container

Cada snapshot é publicado no EventBus como evento `ContainerMetrics` com o `service_id` e timestamp correspondentes.

---

## 6. Banco de Dados — SurrealDB Embarcado

### 6.1 Modo de Operação

SurrealDB é iniciado em modo embarcado com backend RocksDB. Isso significa zero processo externo — o banco vive dentro do mesmo processo do daemon, acessado diretamente pela memória. O namespace é `rustploy` e o banco é `main`. Um sistema de migrations garante que o schema evolui de forma controlada entre versões do daemon.

### 6.2 Schema Completo

**Tabela `project`** — campos: `id` (string ULID), `name` (string, único), `description` (string opcional), `created_at` (datetime). Índice único em `name`.

**Tabela `service`** — campos: `id`, `name`, `project_id`, `image`, `port` (inteiro), `domain` (string, único), `env_vars` (array), `volumes` (array), `healthcheck` (objeto), `resources` (objeto), `status` (string, default `'Stopped'`), `live_container_id` (string opcional), `created_at`, `updated_at`. Índice único em `domain`.

**Tabela `deployment`** — campos: `id`, `service_id`, `image`, `state` (string com o nome do estado atual), `states_log` (array de objetos `{from, to, at, message}`), `started_at`, `finished_at` (datetime opcional).

**Tabela `secret`** — campos: `id`, `project_id`, `key`, `value` (string criptografada com age). Índice único em `(project_id, key)`.

**Tabela `tls_cert`** — campos: `id`, `domain` (único), `cert_pem`, `key_pem`, `expires_at`. Índice único em `domain`.

**Relações de grafo:** `has` (Project → Service) e `deploys` (Service → Deployment).

### 6.3 Queries Críticas

- **Recovery ao iniciar** — selecionar todos os deployments cujo `state` não pertença ao conjunto de estados terminais `['Live', 'Failed', 'Pruning']`
- **Serviços de um projeto** — navegar a relação de grafo `project → [has] → service` a partir do `project_id`
- **Último deployment de cada serviço** — agrupar por `service_id`, ordenar por `started_at` decrescente, retornar o primeiro de cada grupo
- **Rotas iniciais do Pingora** — selecionar todos os serviços com `status = 'Running'` e `live_container_id` preenchido para reconstituir a tabela de rotas ao iniciar o daemon

---

## 7. API do Daemon (Axum sobre UDS)

### 7.1 Rotas HTTP

```
POST   /projects              → Command::ProjectCreate
GET    /projects              → Command::ProjectList
DELETE /projects/:id          → Command::ProjectDelete

POST   /services              → Command::ServiceCreate
GET    /services?project=:id  → Command::ServiceList
GET    /services/:id          → (retorna Service completo)
PUT    /services/:id          → Command::ServiceUpdate
DELETE /services/:id          → Command::ServiceDelete

POST   /deployments                  → Command::DeployStart { service_id }
DELETE /deployments/:id              → Command::DeployAbort
POST   /deployments/:id/rollback     → Command::DeployRollback

GET    /stream?service=:id    → Event stream (chunked Bincode)
GET    /health                → { "ok": true, "version": "..." }
```

### 7.2 Autenticação

Na v1, a autenticação é baseada em **socket permissions**: apenas processos rodando como o mesmo usuário (ou root) podem conectar ao UDS. O daemon verifica o peer UID via `SO_PEERCRED` ao aceitar cada conexão.

Para deploys remotos futuros (v2), o plano é expor uma API HTTPS com autenticação via API token armazenado no SurrealDB (hash bcrypt).

---

## 8. Crate `client` — TUI

### 8.1 Layout das Telas

#### Dashboard Principal

```
┌─ Rustploy v0.1.0 ───────────────────────────────────────────── [q]uit ─┐
│ PROJETOS                    │ SERVIÇOS                                 │
│ ► my-app-project            │ ► api-service       [RUNNING]  ↑512M 12% │
│   blog-project              │   worker-service    [RUNNING]  ↑128M  3% │
│   staging-env               │   cache-service    [STOPPED]             │
│                             │                                          │
├─────────────────────────────┴──────────────────────────────────────────┤
│ ÚLTIMO DEPLOY: api-service                                             │
│  ✓ Pending → ResolvingDeps → PullingImage → Staging →                  │
│    HealthcheckPolling → SwappingIn → Draining → Promoting → Live       │
│  Duração total: 47s    Concluído: 2025-05-14 22:13:08                  │
├────────────────────────────────────────────────────────────────────────┤
│ [d]eploy  [l]ogs  [m]étrics  [e]nv vars  [r]ollback  [↑↓] navegar      │
└────────────────────────────────────────────────────────────────────────┘
```

#### Tela de Deploy em Progresso

```
┌─ Deploy: api-service ──────────────────────────────────────────────────┐
│ Imagem: ghcr.io/user/api:main-abc123f                                  │
│                                                                        │
│ [████████████░░░░░░░░░░░░] 48%  PullingImage                           │
│                                                                        │
│ Camadas:                                                               │
│  ✓ sha256:a1b2c3... Pull complete     (done)                           │
│  ↓ sha256:g7h8i9... Downloading       35.2 MB / 72.0 MB                │
│  ◌ sha256:j1k2l3... Waiting                                            │
│                                                                        │
├────────────────────────────────────────────────────────────────────────┤
│ Eventos recentes:                                                      │
│  22:14:01 → ResolvingDeps   Rede rp_net_abc123 OK                      │
│  22:14:01 → PullingImage    Iniciando pull de 4 camadas                │
│                                                                        │
│ [a]bortar deploy                                                       │
└────────────────────────────────────────────────────────────────────────┘
```

#### Tela de Logs

```
┌─ Logs: api-service ────────────────────────── [f]ilter [w]rap [↑↓] ────┐
│ 22:14:53.121 [INFO]  Server listening on 0.0.0.0:8080                  │
│ 22:14:55.340 [INFO]  Database connection established                   │
│ 22:15:01.002 [INFO]  GET /health 200 2ms                               │
│ 22:15:10.441 [WARN]  Slow query detected: 210ms                        │
│ 22:15:11.882 [INFO]  POST /api/users 201 44ms                          │
│ ▄ (streaming...)                                                       │
└────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Estado Global do TUI

O estado da aplicação TUI mantém em memória:

- `screen` — a tela atualmente ativa (Dashboard, ServiceDetail, DeployProgress, Logs, Metrics, EnvVars, Confirm)
- `projects` e `services` — listas obtidas do daemon ao iniciar e atualizadas por eventos
- `selected_project` / `selected_service` — índices de navegação
- `deploy_progress` — mapa de deployment_id para dados de progresso em curso
- `logs` — mapa de service_id para buffer circular de linhas (máximo 2000 por serviço)
- `metrics` — mapa de service_id para fila circular de pontos de métricas (últimos 60 pontos)
- `notification` — mensagem de notificação temporária com tipo e timestamp de expiração

### 8.3 Loop de Eventos

O client multiplexa três fontes de eventos de forma assíncrona:

1. **Input do teclado** — eventos do terminal processados para navegação e ações
2. **Stream do daemon** — eventos recebidos e aplicados ao estado local (progresso, logs, métricas)
3. **Tick de animação** — dispara a cada 100ms para redesenhar a interface e processar timers internos (expiração de notificações, animações de loading)

Em cada iteração do loop, a tela é redesenhada primeiro e depois o próximo evento de qualquer das três fontes é aguardado de forma concorrente sem bloquear as outras.

---

## 9. Configuração

### 9.1 Arquivo de Configuração

Localização padrão: `/etc/rustploy/config.toml` (ou `~/.config/rustploy/config.toml` para instalação de usuário).

| Seção            | Chave              | Padrão                                     | Descrição                                              |
|------------------|--------------------|--------------------------------------------|--------------------------------------------------------|
| `[daemon]`       | `socket_path`      | `/run/rustploy/rustploy.sock`              | Caminho do Unix Domain Socket                          |
| `[daemon]`       | `db_path`          | `/var/lib/rustploy/db`                     | Diretório dos dados do SurrealDB                       |
| `[daemon]`       | `log_level`        | `info`                                     | Verbosidade dos logs (trace/debug/info/warn/error)     |
| `[ingress]`      | `http_port`        | `80`                                       | Porta HTTP do Pingora                                  |
| `[ingress]`      | `https_port`       | `443`                                      | Porta HTTPS do Pingora                                 |
| `[ingress]`      | `bind_address`     | `0.0.0.0`                                  | Interface de rede para bind                            |
| `[ingress.acme]` | `enabled`          | `true`                                     | Ativar/desativar ACME automático                       |
| `[ingress.acme]` | `email`            | —                                          | E-mail para registro na autoridade certificadora       |
| `[ingress.acme]` | `directory`        | URL de produção do Let's Encrypt           | URL do diretório ACME (trocar por staging para testes) |
| `[docker]`       | `socket_path`      | `/var/run/docker.sock`                     | Caminho do socket do Docker Engine                     |
| `[deploy]`       | `drain_secs`       | `10`                                       | Segundos de drenagem antes de destruir container antigo|
| `[deploy]`       | `image_cache`      | `2`                                        | Versões de imagem antigas a manter por serviço         |
| `[metrics]`      | `interval_secs`    | `2`                                        | Intervalo de coleta de métricas dos containers         |
| `[metrics]`      | `history_points`   | `60`                                       | Pontos históricos em memória por serviço               |
| `[secrets]`      | `master_key_path`  | `/etc/rustploy/master.key`                 | Caminho da chave mestra de criptografia                |

### 9.2 Variáveis de Ambiente

Todas as configurações podem ser sobrescritas via env com prefixo `RUSTPLOY_`:
- `RUSTPLOY_DB_PATH`
- `RUSTPLOY_SOCKET_PATH`
- `RUSTPLOY_LOG_LEVEL`
- `RUSTPLOY_MASTER_KEY`

---

## 10. Segurança

### 10.1 Isolamento de Containers

- Cada projeto tem uma rede Docker bridge dedicada (`rp_net_{project_id_short}`)
- Containers de projetos diferentes não se enxergam pela rede
- O Pingora é o único ponto de entrada externo
- Containers não têm `--privileged` nem capabilities extras por padrão

### 10.2 Gestão de Secrets

Secrets são criptografados em repouso usando `age`:

1. O daemon gera uma chave mestra `age` no primeiro start (ou lê de arquivo configurado)
2. Ao criar um secret, o daemon criptografa o valor e armazena o ciphertext no SurrealDB
3. Ao criar o container, os secrets são decriptografados em memória e injetados como variáveis de ambiente
4. O valor plaintext **nunca** é gravado em disco nem transmitido via UDS

### 10.3 Permissões do Socket

```
/run/rustploy/rustploy.sock → owner: rustploy:rustploy, mode: 0660
```

Apenas membros do grupo `rustploy` podem conectar. Root sempre tem acesso.

---

## 11. Tratamento de Erros e Resiliência

### 11.1 Categorias de Erro

**Erros do cliente (input inválido):**
- `NotFound` — recurso não encontrado
- `InvalidSpec` — especificação de serviço inválida
- `DomainConflict` — outro serviço já usa o mesmo domínio
- `ServiceAlreadyDeploying` — deploy já em andamento para este serviço

**Erros do servidor (falha interna):**
- `DockerUnreachable` — não foi possível conectar ao Docker Engine
- `DatabaseError` — falha de leitura ou escrita no SurrealDB
- `HealthcheckFailed` — container não passou no healthcheck após esgotar tentativas
- `ImagePullFailed` — falha no download da imagem do registry
- `GitCloneFailed` — falha ao clonar o repositório (credenciais inválidas, repo não encontrado, timeout)
- `ImageBuildFailed` — falha durante o `docker build` (erro no Dockerfile, dependência indisponível, etc.)
- `IngressError` — erro ao atualizar rotas no Pingora

### 11.2 Estratégia de Retry

| Operação          | Retry? | Backoff            | Max tentativas |
|-------------------|--------|--------------------|----------------|
| Docker pull       | Sim    | Exponencial 1s→30s | 3              |
| Healthcheck poll  | Sim    | Fixo (configurável)| Configurável   |
| ACME challenge    | Sim    | Exponencial 5s→60s | 5              |
| DB write          | Sim    | Exponencial 50ms   | 5              |
| Ping container    | Não    | —                  | 1              |

### 11.3 Recovery ao Reiniciar o Daemon

Ao iniciar, o daemon consulta o banco por todos os deployments em estados não-terminais e os processa conforme o estado encontrado:

- **Estados pré-swap** (`Pending`, `ResolvingDeps`, `PullingImage`, `CloningRepo`, `BuildingImage`, `Staging`, `HealthcheckPolling`) — o container antigo ainda está vivo; rollback seguro: container de staging e diretório de clone são destruídos e deployment é marcado como `Failed`
- **Swap em curso** (`SwappingIn`, `Draining`) — inspecionar quais containers existem no Docker Engine e decidir se promove ou reverte baseado no que está vivo
- **`Promoting`** — concluir a renomeação do container e atualizar o banco
- **`RollingBack`** — concluir o rollback e marcar como `Failed`

---

## 12. Observabilidade

### 12.1 Logs Estruturados

O daemon emite logs em formato JSON estruturado em produção. Cada entrada inclui `timestamp`, `level`, `target` (módulo de origem) e campos contextuais como `service_id` e `deployment_id` quando aplicável. Exemplo de entrada:

```
{"timestamp":"2025-05-14T22:14:01Z","level":"INFO","target":"daemon::deploy","service_id":"01HZ...","deployment_id":"01HZ...","message":"transitioning state","from":"Staging","to":"HealthcheckPolling"}
```

Isso permite filtrar e agregar logs com qualquer ferramenta de análise (jq, Loki, etc.) sem parsing ad-hoc.

### 12.2 Métricas (v2: Prometheus)

Na v1, métricas vão apenas ao TUI via event stream. Na v2, endpoint `/metrics` em formato Prometheus:

```
rustploy_service_cpu_percent{service="api",project="my-app"} 12.3
rustploy_service_memory_bytes{service="api",project="my-app"} 536870912
rustploy_deployments_total{service="api",result="success"} 14
rustploy_deploy_duration_seconds{service="api"} 47.2
```

---

## 13. Dependências Principais

| Crate                | Versão | Finalidade                                                 |
|----------------------|--------|------------------------------------------------------------|
| `tokio`              | 1      | Runtime assíncrono                                         |
| `axum`               | 0.7    | Framework HTTP para a API sobre UDS                        |
| `hyper-util`         | 0.1    | Utilitários HTTP/1.1 para UDS                              |
| `serde` + `bincode`  | 1      | Serialização binária do protocolo                          |
| `surrealdb`          | 2      | Banco de dados embarcado (feature `kv-rocksdb`)            |
| `bollard`            | 0.17   | Cliente da API do Docker Engine                            |
| `pingora`            | 0.4    | Biblioteca de proxy reverso HTTP                           |
| `pingora-proxy`      | 0.4    | Trait `ProxyHttp` e peer management                        |
| `ratatui`            | 0.28   | Framework de TUI                                           |
| `crossterm`          | 0.28   | Backend de terminal e stream de eventos de teclado         |
| `instant-acme`       | 0.7    | Protocolo ACME para obtenção de certificados TLS           |
| `rustls`             | 0.23   | TLS puro em Rust (sem OpenSSL)                             |
| `age`                | 0.10   | Criptografia de secrets em repouso                         |
| `ulid`               | 1      | Geração de IDs ordenáveis                                  |
| `arc-swap`           | 1      | Ponteiro atômico para leitura lock-free da tabela de rotas |
| `anyhow`             | 1      | Gestão de erros contextuais                                |
| `thiserror`          | 2      | Derivação de tipos de erro estruturados                    |
| `tracing`            | 0.1    | Instrumentação e logs estruturados                         |
| `chrono`             | 0.4    | Timestamps e manipulação de datas                          |
| `reqwest`            | 0.12   | Requisições HTTP para healthcheck (feature `rustls-tls`)   |
| `git2`               | 0.19   | Clone e checkout de repositórios Git (bindings libgit2)    |
| `async-stream`       | 0.3    | Macro para criar streams assíncronos (event stream)        |

---

## 14. Desafios Técnicos Conhecidos

### 14.1 Volumes e Persistência

O maior desafio de zero-downtime com volumes é a janela de escrita dupla: durante o Draining, o container antigo ainda pode escrever no volume enquanto o novo já leu o estado. Estratégia:

- Para bancos de dados: o healthcheck deve confirmar que a aplicação está pronta *após* aplicar migrations
- Para volumes de arquivo: documentar que é responsabilidade da aplicação tolerar acesso concorrente
- Na v2: suporte opcional a snapshots de volume via LVM ou Btrfs antes de cada deploy

### 14.2 Pingora como Biblioteca Embarcada

Pingora não foi projetada para rodar embutida dentro de outro servidor. Os desafios:

- O Pingora tem seu próprio signal handling — integrar com cuidado com o signal handler do daemon
- A solução é rodar o Pingora em uma thread OS separada (não tokio) e usar channels para comunicação com o daemon

### 14.3 Detecção de IP do Container na Rede Correta

Containers conectados a múltiplas redes têm múltiplos IPs. O healthcheck HTTP deve usar sempre o IP do container na rede isolada do projeto — não na rede padrão do Docker Engine. A lógica de lookup filtra explicitamente pelo `network_id` da rede do projeto na estrutura de inspeção do container retornada pelo Docker Engine.

### 14.4 SurrealDB Embarcado com RocksDB

O RocksDB em modo embarcado pode apresentar write amplification elevada sob escrita contínua. Mitigações:

- Ajustar parâmetros de compaction conforme o padrão de escrita do daemon
- Alternativa de fallback: SpeeDB (fork mais leve do RocksDB, também suportado pelo SurrealDB)
- Implementar endpoint de backup que dispara um export do SurrealDB para arquivo

---

## 15. Roteiro de Implementação

### Fase 0 — Infraestrutura (concluída)
- [x] Workspace Cargo com crates `daemon`, `client`, `shared`
- [x] UDS + Axum + Bincode funcionando (echo server)
- [x] TUI Ratatui com input e display de respostas

### Fase 1 — Core do Daemon
- [ ] Definir todos os tipos em `shared`: Command, Event, Response, modelos de domínio
- [ ] Integrar SurrealDB embarcado com schema inicial e sistema de migrations
- [ ] CRUD de projetos e serviços via API UDS
- [ ] Integração com Docker Engine: pull de imagem, criação de container, gestão de redes
- [ ] EventBus funcional com broadcast para múltiplos subscribers

### Fase 2 — Máquina de Estados de Deploy
- [ ] Enum de estados completo com todos os dados por estado
- [ ] Executor com lógica de transição para cada estado
- [ ] Healthcheck polling nos três modos (HTTP, TCP, DockerNative)
- [ ] Persistência de cada transição no SurrealDB
- [ ] Recovery ao reiniciar o daemon

### Fase 3 — Ingress
- [ ] Integrar Pingora como biblioteca dentro do daemon
- [ ] IngressController com tabela de rotas em leitura lock-free
- [ ] Roteamento por Host header com lookup de domínio
- [ ] Carregamento das rotas existentes do banco ao iniciar o daemon
- [ ] TLS com certificados manuais (sem ACME ainda)

### Fase 4 — TUI Completo
- [ ] Dashboard com lista de projetos/serviços e métricas inline
- [ ] Tela de progresso de deploy com barra por camada de imagem
- [ ] Streaming de logs em tempo real com buffer circular
- [ ] Gráficos sparkline de CPU e memória
- [ ] Formulário de criação e edição de serviço

### Fase 5 — ACME e Secrets
- [ ] Integração com protocolo ACME para obtenção automática de certificados Let's Encrypt
- [ ] Renovação automática em background
- [ ] Gestão de secrets com criptografia em repouso

### Fase 6 — Produção
- [ ] Testes de integração com Docker Engine real
- [ ] Systemd unit file e script de instalação
- [ ] Documentação de usuário
- [ ] Benchmark de footprint de memória com alvo de menos de 50 MB em idle
