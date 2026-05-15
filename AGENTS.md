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
- **Não suporta build de imagens** na v1 — trabalha com imagens já publicadas em registry
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

### 3.2 Enum `Command` (client → daemon)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command {
    // Projetos
    ProjectCreate { name: String, description: Option<String> },
    ProjectDelete { id: ProjectId },
    ProjectList,

    // Serviços
    ServiceCreate(ServiceSpec),
    ServiceUpdate { id: ServiceId, spec: ServiceSpec },
    ServiceDelete { id: ServiceId },
    ServiceList { project_id: ProjectId },

    // Deployments
    DeployStart { service_id: ServiceId },
    DeployAbort { deployment_id: DeploymentId },
    DeployRollback { service_id: ServiceId },
    DeployHistory { service_id: ServiceId, limit: u32 },

    // Observability
    LogsSubscribe { service_id: ServiceId, lines_back: u32 },
    LogsUnsubscribe { service_id: ServiceId },
    MetricsSubscribe { service_id: ServiceId },
    MetricsUnsubscribe { service_id: ServiceId },

    // Infra
    Ping,
    DaemonStatus,
}
```

### 3.3 Enum `Event` (daemon → client, stream)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    // Estado de deploy
    DeployStateChanged {
        deployment_id: DeploymentId,
        service_id: ServiceId,
        state: DeployState,
        timestamp: i64,
        message: Option<String>,
    },
    DeployProgress {
        deployment_id: DeploymentId,
        phase: DeployPhase,
        percent: u8,
        detail: String,
    },

    // Logs
    LogLine {
        service_id: ServiceId,
        container_id: String,
        timestamp: i64,
        stream: LogStream,  // Stdout | Stderr
        line: String,
    },

    // Métricas em tempo real
    ContainerMetrics {
        service_id: ServiceId,
        container_id: String,
        cpu_percent: f32,
        mem_bytes: u64,
        mem_limit_bytes: u64,
        net_rx_bytes: u64,
        net_tx_bytes: u64,
        timestamp: i64,
    },

    // Notificações gerais
    ServiceStatusChanged { service_id: ServiceId, status: ServiceStatus },
    DaemonReady { version: String },
    Error { code: ErrorCode, message: String },
}
```

### 3.4 Enum `Response` (daemon → client, request/response)

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Response {
    Ok,
    Project(Project),
    Projects(Vec<Project>),
    Service(Service),
    Services(Vec<Service>),
    Deployment(Deployment),
    Deployments(Vec<Deployment>),
    DaemonStatus(DaemonStatusInfo),
    Pong { uptime_secs: u64 },
    Err(ApiError),
}
```

### 3.5 Modelos de Dados

```rust
pub type ProjectId    = ulid::Ulid;
pub type ServiceId    = ulid::Ulid;
pub type DeploymentId = ulid::Ulid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceSpec {
    pub name: String,
    pub project_id: ProjectId,
    pub image: String,           // ex: "ghcr.io/user/app:latest"
    pub port: u16,               // porta interna do container
    pub domain: String,          // ex: "app.example.com"
    pub env_vars: Vec<EnvVar>,
    pub volumes: Vec<VolumeMount>,
    pub healthcheck: Healthcheck,
    pub replicas: u8,            // sempre 1 na v1
    pub resources: ResourceLimits,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    pub id: ServiceId,
    pub spec: ServiceSpec,
    pub status: ServiceStatus,
    pub live_container_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServiceStatus {
    Stopped,
    Deploying,
    Running,
    Degraded,
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Deployment {
    pub id: DeploymentId,
    pub service_id: ServiceId,
    pub image: String,
    pub state: DeployState,
    pub states_log: Vec<StateTransition>,  // histórico completo
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateTransition {
    pub from: DeployState,
    pub to: DeployState,
    pub at: i64,
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: EnvVarValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EnvVarValue {
    Plain(String),
    Secret(String),  // referência a um secret, não o valor
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Healthcheck {
    pub kind: HealthcheckKind,
    pub interval_secs: u32,
    pub timeout_secs: u32,
    pub retries: u32,
    pub start_period_secs: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HealthcheckKind {
    Http { path: String, expected_status: u16 },
    Tcp,
    DockerNative,  // usa o HEALTHCHECK da imagem
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResourceLimits {
    pub cpu_shares: u64,         // relativo, ex: 512 = metade de 1024
    pub mem_limit_bytes: u64,    // 0 = sem limite
}
```

---

## 4. Máquina de Estados do Deploy

### 4.1 Estados e Transições

```
                    ┌─────────┐
               ─────► Pending │
                    └────┬────┘
                         │ dependências OK
                    ┌────▼──────────────┐
                    │ ResolvingDeps      │
                    └────┬──────────────┘
                         │ rede OK, secrets OK
                    ┌────▼──────────────┐
                    │ PullingImage       │◄── progresso via Event::DeployProgress
                    └────┬──────────────┘
                         │ imagem disponível localmente
                    ┌────▼──────────────┐
                    │ Staging            │  cria container N+1 (sem tráfego)
                    └────┬──────────────┘
                         │ container criado e iniciado
                    ┌────▼──────────────┐
                    │ HealthcheckPolling │  loop até pass ou timeout
                    └────┬──────────────┘
              pass │          │ fail / timeout
         ┌─────────▼──┐   ┌──▼──────────┐
         │ SwappingIn  │   │  RollingBack │
         └─────────┬───┘   └──┬──────────┘
                   │          │ tráfego devolvido ao container antigo
         ┌─────────▼──┐   ┌──▼──────────┐
         │  Draining   │   │   Failed    │◄── estado terminal
         └─────────┬───┘   └─────────────┘
                   │ drain_secs decorridos
         ┌─────────▼──┐
         │  Promoting  │  renomeia container, atualiza SurrealDB
         └─────────┬───┘
                   │
         ┌─────────▼──┐
         │    Live     │◄── estado terminal (sucesso)
         └─────────────┘
                   │ próximo deploy iniciado
         ┌─────────▼──┐
         │   Pruning   │  remove container antigo e imagens órfãs
         └─────────────┘
```

### 4.2 Persistência de Estado

Cada transição de estado é uma transação ACID no SurrealDB. O formato no banco:

```surql
CREATE deployment SET
    id = $id,
    service_id = $service_id,
    image = $image,
    state = 'Pending',
    states_log = [],
    started_at = time::now();

-- Em cada transição:
UPDATE deployment:$id SET
    state = $new_state,
    states_log += [{
        from: $old_state,
        to: $new_state,
        at: time::now(),
        message: $message
    }];
```

**Invariante de recuperação**: ao iniciar, o daemon executa uma query por todos os deployments cujo estado não seja `Live`, `Failed`, `Pruning`. Para cada um, a função `recovery::resume()` é chamada e o deploy é retomado ou abortado com rollback, dependendo do estado encontrado.

### 4.3 Implementação da State Machine

```rust
// crates/daemon/src/deploy/state.rs

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeployState {
    Pending,
    ResolvingDeps,
    PullingImage { layer_count: u32, layers_done: u32 },
    Staging,
    HealthcheckPolling { attempt: u32, max_attempts: u32 },
    SwappingIn,
    Draining { deadline: i64 },
    Promoting,
    Live,
    RollingBack { reason: String },
    Failed { reason: String },
    Pruning,
}

// crates/daemon/src/deploy/executor.rs

pub struct DeployExecutor {
    db: Arc<SurrealClient>,
    docker: Arc<DockerClient>,
    ingress: Arc<IngressController>,
    event_bus: Arc<EventBus>,
}

impl DeployExecutor {
    pub async fn run(&self, deployment_id: DeploymentId) -> Result<()> {
        let mut deployment = self.db.get_deployment(deployment_id).await?;
        let service = self.db.get_service(deployment.service_id).await?;

        loop {
            let next = self.step(&deployment, &service).await;
            match next {
                Ok(DeployState::Live) | Ok(DeployState::Failed { .. }) => {
                    self.transition(&mut deployment, next?, None).await?;
                    break;
                }
                Ok(next_state) => {
                    self.transition(&mut deployment, next_state, None).await?;
                }
                Err(e) => {
                    self.transition(
                        &mut deployment,
                        DeployState::RollingBack { reason: e.to_string() },
                        Some(e.to_string()),
                    ).await?;
                }
            }
        }
        Ok(())
    }

    async fn step(&self, dep: &Deployment, svc: &Service) -> Result<DeployState> {
        match &dep.state {
            DeployState::Pending          => self.resolve_deps(svc).await,
            DeployState::ResolvingDeps    => self.pull_image(dep, svc).await,
            DeployState::PullingImage {..} => self.stage_container(dep, svc).await,
            DeployState::Staging          => self.poll_healthcheck(dep, svc).await,
            DeployState::HealthcheckPolling {..} => self.swap_in(dep, svc).await,
            DeployState::SwappingIn       => self.drain(dep, svc).await,
            DeployState::Draining {..}    => self.promote(dep, svc).await,
            DeployState::Promoting        => Ok(DeployState::Live),
            DeployState::RollingBack {..} => self.rollback(dep, svc).await,
            _ => Err(anyhow!("estado terminal atingido inesperadamente")),
        }
    }
}
```

---

## 5. Crate `daemon` — Subsistemas

### 5.1 Subsistema Docker (`crates/daemon/src/docker/`)

Wrapper sobre `bollard` que encapsula todas as interações com `dockerd`:

#### 5.1.1 Gestão de Imagens

```rust
pub struct ImageManager { docker: Docker }

impl ImageManager {
    /// Faz pull emitindo progresso via event_bus
    pub async fn pull(
        &self,
        image: &str,
        event_bus: Arc<EventBus>,
        deployment_id: DeploymentId,
    ) -> Result<()>;

    /// Verifica se a imagem já existe localmente
    pub async fn exists(&self, image: &str) -> Result<bool>;

    /// Remove imagens não referenciadas por nenhum container gerenciado
    pub async fn prune_unused(&self, managed_images: &[String]) -> Result<()>;
}
```

O pull usa `docker.create_image()` com `futures::Stream` para receber as camadas e emite `Event::DeployProgress` por chunk recebido. Isso permite ao TUI mostrar progresso real de download.

#### 5.1.2 Gestão de Containers

Convenção de nomenclatura:
- Container ativo: `rp_{service_name}`
- Container em staging: `rp_{service_name}_staging_{deployment_id_short}`

```rust
pub struct ContainerManager { docker: Docker }

impl ContainerManager {
    pub async fn create_staging(
        &self,
        svc: &Service,
        dep: &Deployment,
    ) -> Result<String>;  // retorna container_id

    pub async fn start(&self, container_id: &str) -> Result<()>;
    pub async fn stop_graceful(&self, container_id: &str, timeout_secs: u32) -> Result<()>;
    pub async fn rename(&self, id: &str, new_name: &str) -> Result<()>;
    pub async fn remove(&self, container_id: &str) -> Result<()>;
    pub async fn inspect(&self, container_id: &str) -> Result<ContainerInfo>;
}
```

A criação do container de staging sempre inclui:
- `network_mode`: a rede bridge do projeto (`rp_net_{project_id_short}`)
- `labels`: `rustploy.managed=true`, `rustploy.service_id={id}`, `rustploy.deployment_id={id}`
- `restart_policy`: `none` durante staging (o daemon controla o ciclo de vida)
- `host_config.memory`: do `ResourceLimits`
- `host_config.cpu_shares`: do `ResourceLimits`

#### 5.1.3 Gestão de Redes

Cada projeto tem uma rede bridge isolada. Containers do mesmo projeto se veem pelo nome (`rp_{service_name}`), mas o mundo externo só os acessa via Pingora.

```rust
pub struct NetworkManager { docker: Docker }

impl NetworkManager {
    pub async fn ensure_project_network(&self, project_id: ProjectId) -> Result<String>;
    pub async fn remove_project_network(&self, project_id: ProjectId) -> Result<()>;
    pub async fn connect_container(&self, container_id: &str, network_id: &str) -> Result<()>;
    pub async fn disconnect_container(&self, container_id: &str, network_id: &str) -> Result<()>;
}
```

#### 5.1.4 Healthcheck Polling

O daemon implementa seu próprio healthcheck polling em vez de depender do healthcheck nativo do Docker, porque:

1. O healthcheck do Docker tem resolução de intervalo grosseira
2. Precisamos detectar o "ready" em tempo real para minimizar o downtime da janela de swap

```rust
pub async fn poll_healthcheck(
    docker: &Docker,
    svc: &Service,
    container_id: &str,
    network_id: &str,
    max_attempts: u32,
) -> Result<bool> {
    for attempt in 0..max_attempts {
        let info = docker.inspect_container(container_id, None).await?;

        if info.state.map(|s| s.running) != Some(Some(true)) {
            return Err(anyhow!("container parou inesperadamente"));
        }

        let ok = match &svc.spec.healthcheck.kind {
            HealthcheckKind::Http { path, expected_status } => {
                let ip = get_container_ip(&info, network_id)?;
                let url = format!("http://{}:{}{}", ip, svc.spec.port, path);
                let resp = reqwest::get(&url).await;
                resp.map(|r| r.status().as_u16() == *expected_status).unwrap_or(false)
            }
            HealthcheckKind::Tcp => {
                let ip = get_container_ip(&info, network_id)?;
                let addr = format!("{}:{}", ip, svc.spec.port);
                tokio::net::TcpStream::connect(&addr).await.is_ok()
            }
            HealthcheckKind::DockerNative => {
                info.state
                    .and_then(|s| s.health)
                    .and_then(|h| h.status)
                    .map(|s| s == "healthy")
                    .unwrap_or(false)
            }
        };

        if ok { return Ok(true); }

        tokio::time::sleep(Duration::from_secs(
            svc.spec.healthcheck.interval_secs as u64
        )).await;
    }
    Ok(false)
}
```

### 5.2 Subsistema de Ingress — Pingora (`crates/daemon/src/ingress/`)

#### 5.2.1 Tabela de Rotas

O Pingora roda na mesma thread pool do tokio. A tabela de rotas é protegida por `arc_swap::ArcSwap` para leitura lock-free no hot path de cada request:

```rust
use arc_swap::ArcSwap;

#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub domain: String,
    pub backend_addr: String,   // "172.20.0.3:8080"
    pub service_id: ServiceId,
    pub tls_cert: Option<CertPair>,
}

#[derive(Debug, Clone)]
pub struct RouteTable {
    routes: HashMap<String, RouteEntry>,
}

pub struct IngressController {
    table: ArcSwap<RouteTable>,
}

impl IngressController {
    /// Chamado pelo DeployExecutor após Promoting
    pub fn upsert_route(&self, entry: RouteEntry) {
        let mut new_table = (**self.table.load()).clone();
        new_table.routes.insert(entry.domain.clone(), entry);
        self.table.store(Arc::new(new_table));
    }

    pub fn remove_route(&self, domain: &str) {
        let mut new_table = (**self.table.load()).clone();
        new_table.routes.remove(domain);
        self.table.store(Arc::new(new_table));
    }
}
```

#### 5.2.2 ProxyHttp Implementation

```rust
use pingora::prelude::*;

pub struct RustployProxy {
    ingress: Arc<IngressController>,
}

#[async_trait]
impl ProxyHttp for RustployProxy {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX { () }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let host = session
            .get_header(http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let table = self.ingress.table.load();
        let entry = table.routes.get(host)
            .ok_or_else(|| Error::explain(HTTPStatus(404), "no route"))?;

        Ok(Box::new(HttpPeer::new(&entry.backend_addr, false, host.to_string())))
    }
}
```

#### 5.2.3 TLS e ACME

Pingora usa `rustls` nativamente. O gerenciamento de certificados segue este fluxo:

1. **Primeiro deploy de um domínio**: daemon inicia desafio ACME HTTP-01 via `instant-acme`
2. O Pingora expõe o endpoint `/.well-known/acme-challenge/` temporariamente via rota especial
3. Após validação, o certificado é armazenado no SurrealDB (serializado como PEM)
4. O `IngressController` carrega o certificado no `RouteEntry`
5. Renovação automática via cron interno (verifica expiração a cada 12h, renova com > 30 dias de antecedência)

```rust
pub struct TlsManager {
    db: Arc<SurrealClient>,
    acme_account: AcmeAccount,
    ingress: Arc<IngressController>,
}

impl TlsManager {
    pub async fn ensure_cert(&self, domain: &str) -> Result<CertPair>;
    pub async fn renew_expiring(&self) -> Result<Vec<String>>;  // retorna domínios renovados
}
```

### 5.3 EventBus — Canal de Eventos Internos

O `EventBus` é o coração da propagação de estado. Componentes internos publicam eventos; o handler de stream da API os encaminha para os clients conectados:

```rust
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
```

O handler de stream filtra por `service_id` para enviar apenas eventos relevantes ao client:

```rust
// crates/daemon/src/api/stream.rs
async fn stream_handler(
    Query(params): Query<StreamParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut rx = state.event_bus.subscribe();
    let service_id = params.service_id;

    Body::from_stream(async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            if event.service_id() == Some(service_id) || service_id.is_nil() {
                let bytes = bincode::serialize(&event).unwrap();
                let len = (bytes.len() as u32).to_le_bytes();
                let mut frame = Vec::with_capacity(4 + bytes.len());
                frame.extend_from_slice(&len);
                frame.extend_from_slice(&bytes);
                yield Ok::<_, Infallible>(bytes::Bytes::from(frame));
            }
        }
    })
}
```

### 5.4 Coleta de Métricas

O daemon tem uma task tokio dedicada que consulta `docker stats` (via bollard) para cada container vivo, com intervalo configurável (padrão: 2s):

```rust
// crates/daemon/src/metrics.rs

pub async fn metrics_collector(
    docker: Arc<DockerClient>,
    db: Arc<SurrealClient>,
    event_bus: Arc<EventBus>,
    interval: Duration,
) {
    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let services = db.list_running_services().await.unwrap_or_default();

        for svc in services {
            if let Some(container_id) = &svc.live_container_id {
                if let Ok(stats) = docker.container_stats(container_id).await {
                    event_bus.publish(Event::ContainerMetrics {
                        service_id: svc.id,
                        container_id: container_id.clone(),
                        cpu_percent: calculate_cpu_percent(&stats),
                        mem_bytes: stats.memory_stats.usage.unwrap_or(0),
                        mem_limit_bytes: stats.memory_stats.limit.unwrap_or(0),
                        net_rx_bytes: sum_net_rx(&stats),
                        net_tx_bytes: sum_net_tx(&stats),
                        timestamp: chrono::Utc::now().timestamp(),
                    });
                }
            }
        }
    }
}
```

---

## 6. Banco de Dados — SurrealDB Embarcado

### 6.1 Modo de Operação

SurrealDB é iniciado em modo `Db` (embarcado) com backend `RocksDB`. Isso significa zero processo externo, com o banco vivendo dentro do mesmo processo do daemon.

```rust
use surrealdb::engine::local::RocksDb;
use surrealdb::Surreal;

pub async fn connect_db(path: &str) -> Result<Surreal<surrealdb::engine::local::Db>> {
    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("rustploy").use_db("main").await?;
    run_migrations(&db).await?;
    Ok(db)
}
```

### 6.2 Schema Completo

```surql
-- Tabelas
DEFINE TABLE project SCHEMAFULL;
DEFINE FIELD id          ON project TYPE string;
DEFINE FIELD name        ON project TYPE string  ASSERT $value != NONE;
DEFINE FIELD description ON project TYPE option<string>;
DEFINE FIELD created_at  ON project TYPE datetime;
DEFINE INDEX idx_project_name ON project FIELDS name UNIQUE;

DEFINE TABLE service SCHEMAFULL;
DEFINE FIELD id                ON service TYPE string;
DEFINE FIELD name              ON service TYPE string;
DEFINE FIELD project_id        ON service TYPE string;
DEFINE FIELD image             ON service TYPE string;
DEFINE FIELD port              ON service TYPE int;
DEFINE FIELD domain            ON service TYPE string;
DEFINE FIELD env_vars          ON service TYPE array;
DEFINE FIELD volumes           ON service TYPE array;
DEFINE FIELD healthcheck       ON service TYPE object;
DEFINE FIELD resources         ON service TYPE object;
DEFINE FIELD status            ON service TYPE string  DEFAULT 'Stopped';
DEFINE FIELD live_container_id ON service TYPE option<string>;
DEFINE FIELD created_at        ON service TYPE datetime;
DEFINE FIELD updated_at        ON service TYPE datetime;
DEFINE INDEX idx_service_domain ON service FIELDS domain UNIQUE;

DEFINE TABLE deployment SCHEMAFULL;
DEFINE FIELD id          ON deployment TYPE string;
DEFINE FIELD service_id  ON deployment TYPE string;
DEFINE FIELD image       ON deployment TYPE string;
DEFINE FIELD state       ON deployment TYPE string;
DEFINE FIELD states_log  ON deployment TYPE array;
DEFINE FIELD started_at  ON deployment TYPE datetime;
DEFINE FIELD finished_at ON deployment TYPE option<datetime>;

DEFINE TABLE secret SCHEMAFULL;
DEFINE FIELD id         ON secret TYPE string;
DEFINE FIELD project_id ON secret TYPE string;
DEFINE FIELD key        ON secret TYPE string;
DEFINE FIELD value      ON secret TYPE string;  -- criptografado com age
DEFINE INDEX idx_secret_project_key ON secret FIELDS project_id, key UNIQUE;

DEFINE TABLE tls_cert SCHEMAFULL;
DEFINE FIELD id         ON tls_cert TYPE string;
DEFINE FIELD domain     ON tls_cert TYPE string;
DEFINE FIELD cert_pem   ON tls_cert TYPE string;
DEFINE FIELD key_pem    ON tls_cert TYPE string;
DEFINE FIELD expires_at ON tls_cert TYPE datetime;
DEFINE INDEX idx_tls_domain ON tls_cert FIELDS domain UNIQUE;

-- Relações de grafo
DEFINE TABLE has     SCHEMAFULL;  -- project -[has]->    service
DEFINE TABLE deploys SCHEMAFULL;  -- service -[deploys]-> deployment
```

### 6.3 Queries Críticas

```surql
-- Deployments em estado não-terminal (para recovery ao iniciar)
SELECT * FROM deployment
WHERE state NOT IN ['Live', 'Failed', 'Pruning'];

-- Serviços de um projeto (via grafo)
SELECT *, ->has->service AS services
FROM project WHERE id = $project_id
FETCH services;

-- Último deployment de cada serviço
SELECT service_id, state, started_at
FROM deployment
GROUP BY service_id
ORDER BY started_at DESC
LIMIT 1;

-- Serviços com domínios para carregar na tabela de rotas ao iniciar
SELECT id, domain, live_container_id, port
FROM service
WHERE status = 'Running' AND live_container_id != NONE;
```

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
┌─ Rustploy v0.1.0 ─────────────────────────────────────────── [q]uit ─┐
│ PROJETOS                    │ SERVIÇOS                                 │
│ ► my-app-project            │ ► api-service      [RUNNING]  ↑512M 12% │
│   blog-project              │   worker-service   [RUNNING]  ↑128M  3% │
│   staging-env               │   cache-service    [STOPPED]             │
│                             │                                          │
├─────────────────────────────┴──────────────────────────────────────────┤
│ ÚLTIMO DEPLOY: api-service                                             │
│  ✓ Pending → ResolvingDeps → PullingImage → Staging →                 │
│    HealthcheckPolling → SwappingIn → Draining → Promoting → Live       │
│  Duração total: 47s    Concluído: 2025-05-14 22:13:08                  │
├────────────────────────────────────────────────────────────────────────┤
│ [d]eploy  [l]ogs  [m]étrics  [e]nv vars  [r]ollback  [↑↓] navegar     │
└────────────────────────────────────────────────────────────────────────┘
```

#### Tela de Deploy em Progresso

```
┌─ Deploy: api-service ──────────────────────────────────────────────────┐
│ Imagem: ghcr.io/user/api:main-abc123f                                  │
│                                                                        │
│ [████████████░░░░░░░░░░░░] 48%  PullingImage                          │
│                                                                        │
│ Camadas:                                                               │
│  ✓ sha256:a1b2c3... Pull complete     (done)                           │
│  ↓ sha256:g7h8i9... Downloading       35.2 MB / 72.0 MB               │
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
┌─ Logs: api-service ────────────────────────── [f]ilter [w]rap [↑↓] ───┐
│ 22:14:53.121 [INFO]  Server listening on 0.0.0.0:8080                  │
│ 22:14:55.340 [INFO]  Database connection established                   │
│ 22:15:01.002 [INFO]  GET /health 200 2ms                               │
│ 22:15:10.441 [WARN]  Slow query detected: 210ms                        │
│ 22:15:11.882 [INFO]  POST /api/users 201 44ms                          │
│ ▄ (streaming...)                                                       │
└────────────────────────────────────────────────────────────────────────┘
```

### 8.2 Modelo de Estado do TUI

```rust
// crates/client/src/app.rs

pub struct App {
    pub screen: Screen,
    pub projects: Vec<Project>,
    pub services: Vec<Service>,
    pub selected_project: Option<usize>,
    pub selected_service: Option<usize>,
    pub deploy_progress: HashMap<DeploymentId, DeployProgress>,
    pub logs: HashMap<ServiceId, VecDeque<LogLine>>,     // circular buffer, max 2000 linhas
    pub metrics: HashMap<ServiceId, VecDeque<MetricPoint>>, // últimos 60 pontos
    pub notification: Option<(String, NotificationKind, Instant)>,
}

pub enum Screen {
    Dashboard,
    ServiceDetail(ServiceId),
    DeployProgress(DeploymentId),
    Logs(ServiceId),
    Metrics(ServiceId),
    EnvVars(ServiceId),
    Confirm(ConfirmDialog),
}
```

### 8.3 Loop de Eventos

O client usa `tokio::select!` para multiplexar três fontes de eventos:

1. **Keyboard**: crossterm `EventStream`
2. **UDS stream**: eventos em tempo real do daemon
3. **Tick**: timer a 100ms para redesenhar o TUI com animações suaves

```rust
loop {
    terminal.draw(|f| ui::render(f, &app))?;

    tokio::select! {
        Some(key_event) = keyboard.next() => {
            handle_key(&mut app, key_event?).await?;
        }
        Some(daemon_event) = event_rx.recv() => {
            handle_daemon_event(&mut app, daemon_event);
        }
        _ = tick.tick() => {
            app.tick();  // animações, expiração de notificações, etc.
        }
    }

    if app.should_quit { break; }
}
```

---

## 9. Configuração

### 9.1 Arquivo de Configuração

Localização padrão: `/etc/rustploy/config.toml` (ou `~/.config/rustploy/config.toml` para instalação de usuário).

```toml
[daemon]
socket_path = "/run/rustploy/rustploy.sock"
db_path     = "/var/lib/rustploy/db"
log_level   = "info"

[ingress]
http_port    = 80
https_port   = 443
bind_address = "0.0.0.0"

[ingress.acme]
enabled   = true
email     = "admin@example.com"
directory = "https://acme-v02.api.letsencrypt.org/directory"

[docker]
socket_path = "/var/run/docker.sock"

[deploy]
drain_secs   = 10
image_cache  = 2

[metrics]
interval_secs  = 2
history_points = 60

[secrets]
master_key_path = "/etc/rustploy/master.key"
```

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
2. Ao criar um `EnvVarValue::Secret`, o daemon criptografa o valor e armazena o ciphertext no SurrealDB
3. Ao criar o container, os secrets são decriptografados em memória e injetados como variáveis de ambiente
4. O valor plaintext **nunca** é gravado em disco nem transmitido via UDS

### 10.3 Permissões do Socket

```
/run/rustploy/rustploy.sock → owner: rustploy:rustploy, mode: 0660
```

Apenas membros do grupo `rustploy` podem conectar. Root sempre tem acesso.

---

## 11. Tratamento de Erros e Resiliência

### 11.1 Hierarquia de Erros

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ErrorCode {
    // Client errors
    NotFound,
    InvalidSpec,
    DomainConflict,
    ServiceAlreadyDeploying,

    // Server errors
    DockerUnreachable,
    DatabaseError,
    HealthcheckFailed,
    ImagePullFailed,
    IngressError,
}
```

### 11.2 Estratégia de Retry

| Operação          | Retry? | Backoff            | Max tentativas |
|-------------------|--------|--------------------|----------------|
| Docker pull       | Sim    | Exponencial 1s→30s | 3              |
| Healthcheck poll  | Sim    | Fixo (configurável)| Configurável   |
| ACME challenge    | Sim    | Exponencial 5s→60s | 5              |
| DB write          | Sim    | Exponencial 50ms   | 5              |
| Ping container    | Não    | —                  | 1              |

### 11.3 Recovery ao Reiniciar o Daemon

```rust
// crates/daemon/src/deploy/recovery.rs

pub async fn recover_in_flight_deployments(
    db: &SurrealClient,
    executor: &DeployExecutor,
) {
    let in_flight = db.list_non_terminal_deployments().await.unwrap_or_default();

    for dep in in_flight {
        match &dep.state {
            // Pré-swap: rollback seguro (container antigo ainda vivo)
            DeployState::Pending
            | DeployState::ResolvingDeps
            | DeployState::PullingImage { .. }
            | DeployState::Staging
            | DeployState::HealthcheckPolling { .. } => {
                executor.abort_and_cleanup(&dep).await;
            }

            // Swap em progresso: verificar o que está vivo e decidir
            DeployState::SwappingIn | DeployState::Draining { .. } => {
                executor.evaluate_and_finish_swap(&dep).await;
            }

            // Quase done: completar o promote
            DeployState::Promoting => {
                executor.finish_promote(&dep).await;
            }

            // Rollback incompleto: continuar
            DeployState::RollingBack { .. } => {
                executor.finish_rollback(&dep).await;
            }

            _ => {}
        }
    }
}
```

---

## 12. Observabilidade

### 12.1 Logs Estruturados

O daemon usa `tracing` + `tracing-subscriber` com output JSON em produção:

```json
{
  "timestamp": "2025-05-14T22:14:01Z",
  "level": "INFO",
  "target": "daemon::deploy",
  "service_id": "01HZ...",
  "deployment_id": "01HZ...",
  "message": "transitioning state",
  "from": "Staging",
  "to": "HealthcheckPolling"
}
```

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

```toml
# [workspace.dependencies]
tokio             = { version = "1", features = ["full"] }
axum              = { version = "0.7", features = ["macros"] }
hyper-util        = { version = "0.1", features = ["tokio", "server", "http1"] }
serde             = { version = "1", features = ["derive"] }
bincode           = "1"
surrealdb         = { version = "2", features = ["kv-rocksdb"] }
bollard           = { version = "0.17", features = ["ssl"] }
pingora           = "0.4"
pingora-proxy     = "0.4"
ratatui           = "0.28"
crossterm         = { version = "0.28", features = ["event-stream"] }
instant-acme      = "0.7"
rustls            = "0.23"
age               = "0.10"
ulid              = { version = "1", features = ["serde"] }
arc-swap          = "1"
anyhow            = "1"
thiserror         = "2"
tracing           = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
chrono            = { version = "0.4", features = ["serde"] }
reqwest           = { version = "0.12", features = ["rustls-tls"], default-features = false }
async-stream      = "0.3"
```

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

Containers conectados a múltiplas redes têm múltiplos IPs. O healthcheck HTTP deve usar o IP na rede do projeto:

```rust
fn get_container_ip(info: &ContainerInspectResponse, network_id: &str) -> Result<String> {
    info.network_settings
        .as_ref()
        .and_then(|ns| ns.networks.as_ref())
        .and_then(|nets| nets.values().find(|n| n.network_id.as_deref() == Some(network_id)))
        .and_then(|n| n.ip_address.as_deref())
        .filter(|ip| !ip.is_empty())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("container não conectado à rede do projeto"))
}
```

---

## 15. Roteiro de Implementação

### Fase 0 — Infraestrutura (concluída)
- [x] Workspace Cargo com crates `daemon`, `client`, `shared`
- [x] UDS + Axum + Bincode funcionando (echo server)
- [x] TUI Ratatui com input e display de respostas

### Fase 1 — Core do Daemon
- [ ] Definir todos os tipos em `shared` (Command, Event, Response, modelos)
- [ ] Integrar SurrealDB embarcado + schema inicial + migrations
- [ ] CRUD de projetos e serviços via API
- [ ] Integração bollard: pull de imagem, criação de container, gestão de redes
- [ ] EventBus funcional

### Fase 2 — Máquina de Estados
- [ ] `DeployState` enum completo
- [ ] `DeployExecutor` com todas as transições
- [ ] Healthcheck polling (HTTP + TCP + DockerNative)
- [ ] Persistência de estado no SurrealDB
- [ ] Recovery ao reiniciar o daemon

### Fase 3 — Ingress
- [ ] Integrar Pingora como biblioteca
- [ ] `IngressController` com `ArcSwap<RouteTable>`
- [ ] Roteamento por Host header
- [ ] Carregamento de rotas do SurrealDB ao iniciar
- [ ] TLS com `rustls` (certificados manuais primeiro)

### Fase 4 — TUI Completo
- [ ] Dashboard com lista de projetos/serviços e métricas inline
- [ ] Tela de deploy progress com barra de progresso real por camada
- [ ] Stream de logs em tempo real
- [ ] Gráficos sparkline de CPU/RAM
- [ ] Formulário de criação/edição de serviço

### Fase 5 — ACME e Secrets
- [ ] Integração `instant-acme` para Let's Encrypt HTTP-01
- [ ] Renovação automática de certificados
- [ ] Gestão de secrets com `age`

### Fase 6 — Produção
- [ ] Testes de integração com Docker real
- [ ] Systemd unit file
- [ ] Script de instalação
- [ ] Documentação de usuário
- [ ] Benchmark de footprint de memória (alvo: < 50 MB idle)
