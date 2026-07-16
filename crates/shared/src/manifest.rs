//! Infra-as-Code: structs do manifesto declarativo (`rustploy.yml`).
//!
//! Estes tipos são *format-agnostic* (apenas `serde`); o parse de YAML vive nos
//! clientes. Eles mapeiam para os modelos internos [`Project`] e [`ServiceSpec`]
//! de forma ergonômica para edição humana e versionamento em Git.

use crate::models::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Manifesto raiz (agregador): vários projetos, inline ou via `include:`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerManifest {
    #[serde(rename = "apiVersion", default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default)]
    pub projects: Vec<ProjectEntry>,
}

impl ServerManifest {
    /// Constrói um manifesto raiz redigido a partir de TODOS os projetos do
    /// banco (para `Command::ManifestExportAll`): cada `(Project, Vec<Service>)`
    /// vira uma entrada inline via [`ProjectManifest::from_existing_redacted`].
    /// Devolve o manifesto e o [`EnvDoc`] complementar (valores reais das vars
    /// `Plain`, aninhados por projeto → serviço; ver [`redact_env_map`] para o
    /// porquê `Secret` fica de fora — e os dados não-secretos de todo Git
    /// provider referenciado por um serviço `git`, ver [`GitProviderDoc`]).
    ///
    /// O aninhamento do [`EnvDoc`] dá **escopo real**: `DATABASE_URL` de
    /// projetos/serviços diferentes ocupam entradas distintas (`[project."A".env]`
    /// vs. `[project."B".env]`), sem a sobrescrita "última-vence" que o antigo
    /// `.env` plano tinha.
    ///
    /// `providers` é o catálogo de Git providers conectados no daemon,
    /// indexado pelo ID interno (`gp_...`) — usado para resolver
    /// `GitSource.provider_id` para o **nome** que vai no YAML.
    pub fn from_existing_redacted(
        items: &[(Project, Vec<Service>)],
        providers: &BTreeMap<String, GitProvider>,
    ) -> (Self, EnvDoc) {
        let mut doc = EnvDoc::default();
        let projects = items
            .iter()
            .map(|(project, services)| {
                ProjectEntry::Inline(ProjectManifest::from_existing_redacted(
                    project, services, providers, &mut doc,
                ))
            })
            .collect();
        (
            ServerManifest {
                api_version: Some(API_VERSION.to_string()),
                projects,
            },
            doc,
        )
    }
}

/// Documento TOML que acompanha o manifesto no export/import de
/// Infra-as-Code: variáveis de ambiente **e** Git providers (Gitea)
/// conectados. Substitui o antigo `.env` plano: o aninhamento por
/// **projeto → serviço** dá escopo real, então vars homônimas de escopos
/// diferentes (ex.: `APP_PORT` no serviço `api` e no `worker`, ou `DATABASE_URL`
/// em dois projetos) ficam em entradas distintas em vez de uma sobrescrever a
/// outra.
///
/// O placeholder no YAML continua `${KEY}` **cru** — o escopo vem de ONDE o
/// placeholder está (env do projeto vs. env de um serviço), não codificado no
/// nome. Assim um nome de var com qualquer caractere (inclusive `__`) é só uma
/// folha de tabela, sem parsing de separador para dar errado.
///
/// Serializa como:
/// ```toml
/// [project."Chiquitos".env]
/// DATABASE_URL = "..."
///
/// [project."Chiquitos".service."api".env]
/// APP_PORT = "3000"
///
/// [git_provider."meu-gitea"]
/// kind = "gitea"
/// base_url = "https://git.example.com"
/// auth_mode = "pat"
/// ```
///
/// O `git_provider` segue o mesmo princípio: no YAML, `source.git.provider`
/// carrega só o **nome** do provider (referência estável, portável entre
/// exports/imports — o mesmo padrão usado para projetos/serviços); os dados
/// não-secretos do provider ficam aqui. Segredos (client secret OAuth, PAT,
/// access/refresh token) nunca são exportados — ficam só no banco do daemon de
/// origem, exatamente como uma env var `Secret` nunca é decifrada para o
/// cliente.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvDoc {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub project: BTreeMap<String, ProjectEnvDoc>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub git_provider: BTreeMap<String, GitProviderDoc>,
}

/// Dados não-secretos de um Git provider conectado (ver [`EnvDoc`]). No
/// import, se o nome não existir ainda no daemon de destino, um provider
/// **pendente** é criado a partir destes campos (sem token/secret) — o
/// usuário completa a autenticação depois pela GUI (OAuth ou colar o PAT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitProviderDoc {
    /// `gitea` (único suportado hoje).
    pub kind: String,
    pub base_url: String,
    /// `oauth` | `pat`.
    pub auth_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectEnvDoc {
    /// Vars de env do próprio projeto (herdadas por todos os serviços).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service: BTreeMap<String, ServiceEnvDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceEnvDoc {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

/// Uma entrada do manifesto raiz: projeto inline OU referência a um arquivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectEntry {
    /// `- include: ./web/rustploy.yml`
    Include { include: String },
    /// Projeto declarado inline.
    Inline(ProjectManifest),
}

/// Manifesto de um único projeto (`project:` + `services:`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectManifest {
    #[serde(rename = "apiVersion", default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    pub project: ProjectMeta,
    #[serde(default)]
    pub services: Vec<ServiceManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMeta {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Env herdada por todos os serviços. Valor `secret:NOME` vira referência a secret.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceManifest {
    pub name: String,
    pub source: SourceManifest,
    #[serde(default)]
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub tls: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// Cada item: `host:container` ou `host:container:ro`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub healthcheck: Option<HealthcheckManifest>,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub replicas: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Tipo de banco: postgres | mongodb | mariadb | mysql | redis
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub db: Option<String>,
}

/// Origem do serviço: exatamente uma das três chaves deve estar presente.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<GitManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitManifest {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dockerfile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_stage: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub submodules: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub watch_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<String>,
    /// Nome do Git provider conectado (Gitea) a usar para autenticar o clone —
    /// referência por **nome**, não pelo ID interno do banco (ver [`EnvDoc`]
    /// e [`GitProviderDoc`] para onde ficam os dados reais do provider).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthcheckManifest {
    /// `none` | `tcp` | `http` | `docker`
    #[serde(rename = "type", default = "default_hc_type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_period: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourcesManifest {
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub cpu_shares: u64,
    /// Aceita sufixos `k`/`m`/`g` (ex.: `256m`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mem: Option<String>,
}

/// Resultado de um `apply`: o que foi criado/atualizado/removido em cada recurso,
/// mais a lista de serviços para os quais um deploy foi disparado (`--deploy`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApplyReport {
    pub actions: Vec<ResourceAction>,
    pub deployed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAction {
    pub kind: ResourceActionKind,
    /// `nome-do-projeto` ou `projeto/serviço`.
    pub name: String,
    pub action: ActionVerb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceActionKind {
    Project,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionVerb {
    Created,
    Updated,
    Unchanged,
    Deleted,
}

impl std::fmt::Display for ActionVerb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::Updated => write!(f, "updated"),
            Self::Unchanged => write!(f, "unchanged"),
            Self::Deleted => write!(f, "deleted"),
        }
    }
}

impl std::fmt::Display for ResourceActionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Project => write!(f, "project"),
            Self::Service => write!(f, "service"),
        }
    }
}

// --------------------------------------------------------------------------
// Conversão manifesto -> modelos internos
// --------------------------------------------------------------------------

impl ProjectManifest {
    /// Campos do projeto: `(name, description, env_vars)`.
    pub fn project_fields(&self) -> (String, Option<String>, Vec<EnvVar>) {
        (
            self.project.name.clone(),
            self.project.description.clone(),
            env_map_to_vars(&self.project.env),
        )
    }

    /// Converte cada serviço do manifesto numa [`ServiceSpec`] já vinculada ao
    /// projeto. `provider_ids` resolve o **nome** do Git provider (referência
    /// usada em `source.git.provider` no YAML) para o ID interno do banco —
    /// serviços cujo nome não está no mapa ficam sem provider vinculado.
    pub fn service_specs(
        &self,
        project_id: &str,
        provider_ids: &BTreeMap<String, String>,
    ) -> Vec<ServiceSpec> {
        self.services
            .iter()
            .map(|s| s.to_spec(project_id, provider_ids))
            .collect()
    }

    /// Substitui `${VAR}` em todos os valores de env (projeto + serviços)
    /// resolvendo contra o [`EnvDoc`] **com escopo**: o env do projeto olha só a
    /// tabela do projeto; o env de cada serviço olha a tabela do serviço e, se a
    /// var não estiver lá, cai para a do projeto (serviços herdam o env do
    /// projeto no rustploy). Retorna a lista de variáveis não resolvidas, cada
    /// uma rotulada com o escopo (`"projeto: VAR"` ou `"projeto/serviço: VAR"`)
    /// para o cliente saber qual tabela preencher.
    pub fn interpolate(&mut self, env: &EnvDoc) -> Vec<String> {
        let mut missing = Vec::new();
        let pname = self.project.name.clone();
        let pdoc = env.project.get(&pname);

        // Env do projeto: resolve só na tabela do projeto.
        {
            let lookup = |k: &str| pdoc.and_then(|p| p.env.get(k)).cloned();
            interpolate_map_scoped(&mut self.project.env, &lookup, &pname, None, &mut missing);
        }
        // Env de cada serviço: tabela do serviço, com fallback para a do projeto.
        for s in &mut self.services {
            let sdoc = pdoc.and_then(|p| p.service.get(&s.name));
            let lookup = |k: &str| {
                sdoc.and_then(|sv| sv.env.get(k))
                    .or_else(|| pdoc.and_then(|p| p.env.get(k)))
                    .cloned()
            };
            interpolate_map_scoped(&mut s.env, &lookup, &pname, Some(&s.name), &mut missing);
        }
        missing
    }

    /// Reconstrói um manifesto a partir do estado atual no banco (para `export`).
    /// Segredos são emitidos como `secret:NOME`, nunca o valor decifrado.
    /// `providers` resolve `GitSource.provider_id` para o nome do provider
    /// (ver [`GitManifest::provider`]) — indexado pelo ID interno (`gp_...`).
    pub fn from_existing(
        project: &Project,
        services: &[Service],
        providers: &BTreeMap<String, GitProvider>,
    ) -> Self {
        ProjectManifest {
            api_version: Some(API_VERSION.to_string()),
            project: ProjectMeta {
                name: project.name.clone(),
                description: project.description.clone(),
                env: env_vars_to_map(&project.env_vars),
            },
            services: services
                .iter()
                .map(|s| ServiceManifest::from_spec(s, providers))
                .collect(),
        }
    }

    /// Como [`from_existing`](Self::from_existing), mas redige todo valor de env
    /// var `Plain` para `${KEY}` — o YAML nunca carrega um valor real. Os
    /// valores reais são acumulados no [`EnvDoc`] **na tabela do escopo certo**
    /// (env do projeto → `project[name].env`; env de cada serviço →
    /// `project[name].service[svc].env`), de forma que vars homônimas de escopos
    /// diferentes não colidam. `Secret` segue como `secret:NOME`, nunca
    /// decifrada (não entra no [`EnvDoc`]). Usado pelo par YAML+TOML do fluxo
    /// "Infra as Code" da GUI (`Command::ManifestExportAll`).
    pub fn from_existing_redacted(
        project: &Project,
        services: &[Service],
        providers: &BTreeMap<String, GitProvider>,
        doc: &mut EnvDoc,
    ) -> Self {
        let mut m = Self::from_existing(project, services, providers);
        let pentry = doc.project.entry(project.name.clone()).or_default();
        redact_env_map(&mut m.project.env, &mut pentry.env);
        for s in &mut m.services {
            let sentry = pentry.service.entry(s.name.clone()).or_default();
            redact_env_map(&mut s.env, &mut sentry.env);
        }
        for s in services {
            let ServiceSource::Git(g) = &s.spec.source else {
                continue;
            };
            let Some(provider) = g.provider_id.as_ref().and_then(|id| providers.get(id)) else {
                continue;
            };
            doc.git_provider
                .entry(provider.name.clone())
                .or_insert_with(|| GitProviderDoc::from_provider(provider));
        }
        m
    }
}

impl ServiceManifest {
    pub fn to_spec(&self, project_id: &str, provider_ids: &BTreeMap<String, String>) -> ServiceSpec {
        ServiceSpec {
            name: self.name.clone(),
            project_id: project_id.to_string(),
            source: self.source.to_source(provider_ids),
            port: self.port,
            host_port: self.host_port,
            domain: self.domain.clone(),
            tls_enabled: self.tls,
            env_vars: env_map_to_vars(&self.env),
            env_comments: Vec::new(),
            volumes: self.volumes.iter().filter_map(|v| parse_volume(v)).collect(),
            healthcheck: self
                .healthcheck
                .as_ref()
                .map(HealthcheckManifest::to_healthcheck)
                .unwrap_or_default(),
            replicas: self.replicas.max(1),
            resources: self
                .resources
                .as_ref()
                .map(ResourcesManifest::to_limits)
                .unwrap_or_default(),
            run_command: self.command.clone(),
            run_args: self.args.clone(),
            db_kind: self.db.clone(),
            // TODO(multi-domain): o manifesto ainda carrega só o `domain` legado;
            // o campo `domains` fica vazio no import/export.
            domains: vec![],
        }
    }

    pub fn from_spec(svc: &Service, providers: &BTreeMap<String, GitProvider>) -> Self {
        let spec = &svc.spec;
        ServiceManifest {
            name: spec.name.clone(),
            source: SourceManifest::from_source(&spec.source, providers),
            port: spec.port,
            host_port: spec.host_port,
            domain: spec.domain.clone(),
            tls: spec.tls_enabled,
            env: env_vars_to_map(&spec.env_vars),
            volumes: spec.volumes.iter().map(format_volume).collect(),
            healthcheck: HealthcheckManifest::from_healthcheck(&spec.healthcheck),
            replicas: spec.replicas.max(1),
            resources: ResourcesManifest::from_limits(&spec.resources),
            command: spec.run_command.clone(),
            args: spec.run_args.clone(),
            db: spec.db_kind.clone(),
        }
    }
}

impl SourceManifest {
    fn to_source(&self, provider_ids: &BTreeMap<String, String>) -> ServiceSource {
        if let Some(image) = &self.registry {
            ServiceSource::Registry {
                image: image.clone(),
            }
        } else if let Some(g) = &self.git {
            let d = GitSource::default();
            ServiceSource::Git(GitSource {
                url: g.url.clone(),
                branch: g.branch.clone().unwrap_or(d.branch),
                root_path: g.root_path.clone().unwrap_or(d.root_path),
                watch_paths: g.watch_paths.clone(),
                submodules: g.submodules,
                dockerfile_path: g.dockerfile.clone().unwrap_or(d.dockerfile_path),
                build_context: g.context.clone().unwrap_or(d.build_context),
                build_stage: g.build_stage.clone(),
                credentials: g.credentials.clone(),
                username: g.username.clone(),
                provider_id: g.provider.as_ref().and_then(|name| provider_ids.get(name)).cloned(),
            })
        } else if let Some(content) = &self.compose {
            ServiceSource::Compose(ComposeSource {
                content: content.clone(),
            })
        } else {
            // Sem origem declarada: registry vazio (será rejeitado no deploy).
            ServiceSource::Registry {
                image: String::new(),
            }
        }
    }

    fn from_source(src: &ServiceSource, providers: &BTreeMap<String, GitProvider>) -> Self {
        match src {
            ServiceSource::Registry { image } => SourceManifest {
                registry: Some(image.clone()),
                ..Default::default()
            },
            ServiceSource::Git(g) => SourceManifest {
                git: Some(GitManifest {
                    url: g.url.clone(),
                    branch: Some(g.branch.clone()),
                    root_path: Some(g.root_path.clone()),
                    dockerfile: Some(g.dockerfile_path.clone()),
                    context: Some(g.build_context.clone()),
                    build_stage: g.build_stage.clone(),
                    submodules: g.submodules,
                    watch_paths: g.watch_paths.clone(),
                    username: g.username.clone(),
                    credentials: g.credentials.clone(),
                    provider: g
                        .provider_id
                        .as_ref()
                        .and_then(|id| providers.get(id))
                        .map(|p| p.name.clone()),
                }),
                ..Default::default()
            },
            ServiceSource::Compose(c) => SourceManifest {
                compose: Some(c.content.clone()),
                ..Default::default()
            },
        }
    }
}

impl HealthcheckManifest {
    fn to_healthcheck(&self) -> Healthcheck {
        let d = Healthcheck::default();
        let kind = match self.kind.to_lowercase().as_str() {
            "none" => HealthcheckKind::None,
            "http" => HealthcheckKind::Http {
                path: self.path.clone().unwrap_or_else(|| "/".to_string()),
                expected_status: self.status.unwrap_or(200),
            },
            "docker" => HealthcheckKind::DockerNative,
            _ => HealthcheckKind::Tcp,
        };
        Healthcheck {
            kind,
            interval_secs: self.interval.unwrap_or(d.interval_secs),
            timeout_secs: self.timeout.unwrap_or(d.timeout_secs),
            retries: self.retries.unwrap_or(d.retries),
            start_period_secs: self.start_period.unwrap_or(d.start_period_secs),
        }
    }

    fn from_healthcheck(hc: &Healthcheck) -> Option<Self> {
        let (kind, path, status) = match &hc.kind {
            HealthcheckKind::None => ("none", None, None),
            HealthcheckKind::Tcp => ("tcp", None, None),
            HealthcheckKind::DockerNative => ("docker", None, None),
            HealthcheckKind::Http {
                path,
                expected_status,
            } => ("http", Some(path.clone()), Some(*expected_status)),
        };
        Some(HealthcheckManifest {
            kind: kind.to_string(),
            path,
            status,
            interval: Some(hc.interval_secs),
            timeout: Some(hc.timeout_secs),
            retries: Some(hc.retries),
            start_period: Some(hc.start_period_secs),
        })
    }
}

impl ResourcesManifest {
    fn to_limits(&self) -> ResourceLimits {
        ResourceLimits {
            cpu_shares: self.cpu_shares,
            mem_limit_bytes: self.mem.as_deref().and_then(parse_mem).unwrap_or(0),
        }
    }

    fn from_limits(limits: &ResourceLimits) -> Option<Self> {
        if limits.cpu_shares == 0 && limits.mem_limit_bytes == 0 {
            return None;
        }
        Some(ResourcesManifest {
            cpu_shares: limits.cpu_shares,
            mem: (limits.mem_limit_bytes > 0).then(|| humanize_mem(limits.mem_limit_bytes)),
        })
    }
}

impl GitProviderDoc {
    fn from_provider(p: &GitProvider) -> Self {
        GitProviderDoc {
            kind: p.kind.as_str().to_string(),
            base_url: p.base_url.clone(),
            auth_mode: p.auth_mode.as_str().to_string(),
            oauth_client_id: p.oauth_client_id.clone(),
        }
    }
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

pub const API_VERSION: &str = "rustploy/v1";

const SECRET_PREFIX: &str = "secret:";

fn env_map_to_vars(map: &BTreeMap<String, String>) -> Vec<EnvVar> {
    map.iter()
        .map(|(k, v)| EnvVar {
            key: k.clone(),
            value: match v.strip_prefix(SECRET_PREFIX) {
                Some(name) => EnvVarValue::Secret(name.to_string()),
                None => EnvVarValue::Plain(v.clone()),
            },
        })
        .collect()
}

fn env_vars_to_map(vars: &[EnvVar]) -> BTreeMap<String, String> {
    vars.iter()
        .map(|e| {
            let v = match &e.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => format!("{SECRET_PREFIX}{name}"),
            };
            (e.key.clone(), v)
        })
        .collect()
}

/// Redige, in-place, todo valor `Plain` (isto é, que não começa com
/// `secret:`) de `map` para `${KEY}`, guardando o valor real em `dotenv_out`.
/// Valores já em formato `secret:NOME` são deixados intactos e não entram no
/// `.env` (a secret nunca é decifrada para o cliente).
fn redact_env_map(map: &mut BTreeMap<String, String>, dotenv_out: &mut BTreeMap<String, String>) {
    for (k, v) in map.iter_mut() {
        if v.starts_with(SECRET_PREFIX) {
            continue;
        }
        dotenv_out.insert(k.clone(), v.clone());
        *v = format!("${{{k}}}");
    }
}

/// Serializa um [`EnvDoc`] como texto TOML (o arquivo de variáveis que
/// acompanha o manifesto no export de IaC). Ver [`EnvDoc`] para o layout.
pub fn format_env_doc(doc: &EnvDoc) -> String {
    toml::to_string(doc).unwrap_or_default()
}

/// Faz o parse do texto TOML do arquivo de variáveis num [`EnvDoc`]. Erro de
/// sintaxe vira `Err(mensagem)` (o import aborta antes de aplicar qualquer
/// coisa). Texto vazio é um [`EnvDoc`] vazio (todas as `${VAR}` viram faltantes).
pub fn parse_env_doc(text: &str) -> Result<EnvDoc, String> {
    toml::from_str(text).map_err(|e| e.to_string())
}

/// Serializa um mapa `KEY -> VALUE` como texto `.env` (uma linha por var,
/// ordenado por chave). Valores com espaço ou `#` são colocados entre aspas
/// duplas (sem escaping — casa com o parser simples de [`parse_dotenv`]).
pub fn format_dotenv(map: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (k, v) in map {
        if v.is_empty() || v.chars().any(|c| c.is_whitespace() || c == '#') {
            out.push_str(&format!("{k}=\"{v}\"\n"));
        } else {
            out.push_str(&format!("{k}={v}\n"));
        }
    }
    out
}

/// Parser simples de texto `.env`: linhas `KEY=VALUE`, ignora vazias e `#
/// comentário`. Aspas simples/duplas ao redor do valor são removidas (sem
/// escaping interno). Mesma semântica usada pelo `--env-file` do CLI.
pub fn parse_dotenv(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            map.insert(k.trim().to_string(), v.to_string());
        }
    }
    map
}

/// Interpola um mapa de env de um escopo (projeto ou serviço), rotulando as
/// vars não resolvidas com o escopo (`"projeto: VAR"` / `"projeto/serviço: VAR"`)
/// para que o cliente saiba exatamente qual tabela do TOML preencher.
fn interpolate_map_scoped<F>(
    map: &mut BTreeMap<String, String>,
    lookup: &F,
    project: &str,
    service: Option<&str>,
    missing: &mut Vec<String>,
) where
    F: Fn(&str) -> Option<String>,
{
    let mut local = Vec::new();
    for value in map.values_mut() {
        *value = interpolate_str(value, lookup, &mut local);
    }
    for name in local {
        let label = match service {
            Some(s) => format!("{project}/{s}: {name}"),
            None => format!("{project}: {name}"),
        };
        if !missing.contains(&label) {
            missing.push(label);
        }
    }
}

/// Substitui ocorrências de `${VAR}` em `input`. Variáveis ausentes ficam intactas
/// e são acumuladas em `missing`. Não toca em `secret:` refs (sem `${}`).
fn interpolate_str<F>(input: &str, lookup: &F, missing: &mut Vec<String>) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = input[i + 2..].find('}') {
                let name = &input[i + 2..i + 2 + end];
                match lookup(name) {
                    Some(val) => out.push_str(&val),
                    None => {
                        if !missing.contains(&name.to_string()) {
                            missing.push(name.to_string());
                        }
                        out.push_str(&input[i..i + 2 + end + 1]); // mantém ${VAR}
                    }
                }
                i += 2 + end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// `host:container` ou `host:container:ro`.
fn parse_volume(s: &str) -> Option<VolumeMount> {
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(VolumeMount {
        host_path: parts[0].to_string(),
        container_path: parts[1].to_string(),
        read_only: parts.get(2).map(|m| *m == "ro").unwrap_or(false),
    })
}

fn format_volume(v: &VolumeMount) -> String {
    if v.read_only {
        format!("{}:{}:ro", v.host_path, v.container_path)
    } else {
        format!("{}:{}", v.host_path, v.container_path)
    }
}

/// Aceita `1024`, `256k`, `256m`, `2g` (case-insensitive).
fn parse_mem(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult) = match s.chars().last().unwrap().to_ascii_lowercase() {
        'k' => (&s[..s.len() - 1], 1024u64),
        'm' => (&s[..s.len() - 1], 1024 * 1024),
        'g' => (&s[..s.len() - 1], 1024 * 1024 * 1024),
        c if c.is_ascii_digit() => (s, 1),
        _ => return None,
    };
    num.trim().parse::<u64>().ok().map(|n| n * mult)
}

fn humanize_mem(bytes: u64) -> String {
    const G: u64 = 1024 * 1024 * 1024;
    const M: u64 = 1024 * 1024;
    const K: u64 = 1024;
    if bytes % G == 0 {
        format!("{}g", bytes / G)
    } else if bytes % M == 0 {
        format!("{}m", bytes / M)
    } else if bytes % K == 0 {
        format!("{}k", bytes / K)
    } else {
        bytes.to_string()
    }
}

fn default_hc_type() -> String {
    "tcp".to_string()
}
fn one() -> u32 {
    1
}
fn is_one(n: &u32) -> bool {
    *n == 1
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn is_zero_u64(n: &u64) -> bool {
    *n == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
apiVersion: rustploy/v1
project:
  name: minha-app
  description: "API e front"
  env:
    LOG_LEVEL: info
    DB_PASS: ${DB_PASS}
services:
  - name: web
    source:
      git:
        url: https://github.com/acme/web
        branch: main
    port: 3000
    domain: app.example.com
    tls: true
    env:
      API_TOKEN: secret:api-token
    volumes:
      - /data/web:/var/lib/web:ro
    healthcheck:
      type: http
      path: /health
      status: 200
    resources:
      cpu_shares: 512
      mem: 256m
"#;

    #[test]
    fn parses_and_converts() {
        let m: ProjectManifest = serde_yaml::from_str(SAMPLE).unwrap();
        let (name, desc, env) = m.project_fields();
        assert_eq!(name, "minha-app");
        assert_eq!(desc.as_deref(), Some("API e front"));
        assert_eq!(env.len(), 2);

        let specs = m.service_specs("proj-1", &BTreeMap::new());
        assert_eq!(specs.len(), 1);
        let web = &specs[0];
        assert_eq!(web.project_id, "proj-1");
        assert_eq!(web.port, 3000);
        assert!(web.tls_enabled);
        assert_eq!(web.domain.as_deref(), Some("app.example.com"));
        assert!(matches!(&web.source, ServiceSource::Git(g) if g.url.contains("acme/web")));
        assert_eq!(web.volumes.len(), 1);
        assert!(web.volumes[0].read_only);
        assert!(matches!(
            &web.healthcheck.kind,
            HealthcheckKind::Http { expected_status: 200, .. }
        ));
        assert_eq!(web.resources.cpu_shares, 512);
        assert_eq!(web.resources.mem_limit_bytes, 256 * 1024 * 1024);

        // env: secret ref vira EnvVarValue::Secret
        let token = web
            .env_vars
            .iter()
            .find(|e| e.key == "API_TOKEN")
            .unwrap();
        assert!(matches!(&token.value, EnvVarValue::Secret(n) if n == "api-token"));
    }

    fn env_doc_with(project: &str, vars: &[(&str, &str)]) -> EnvDoc {
        let env = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        EnvDoc {
            project: BTreeMap::from([(
                project.to_string(),
                ProjectEnvDoc {
                    env,
                    service: BTreeMap::new(),
                },
            )]),
            git_provider: BTreeMap::new(),
        }
    }

    #[test]
    fn interpolation_resolves_and_reports_missing() {
        // SAMPLE é o projeto "minha-app" com DB_PASS no env do projeto.
        let mut m: ProjectManifest = serde_yaml::from_str(SAMPLE).unwrap();
        let env = env_doc_with("minha-app", &[("DB_PASS", "s3cr3t")]);
        let missing = m.interpolate(&env);
        assert!(missing.is_empty());
        let vars = m.project_fields().2;
        let db = vars.iter().find(|e| e.key == "DB_PASS").unwrap();
        assert!(matches!(&db.value, EnvVarValue::Plain(v) if v == "s3cr3t"));

        // Sem valor: a faltante é rotulada com o escopo do projeto.
        let mut m2: ProjectManifest = serde_yaml::from_str(SAMPLE).unwrap();
        let missing2 = m2.interpolate(&EnvDoc::default());
        assert_eq!(missing2, vec!["minha-app: DB_PASS".to_string()]);
    }

    #[test]
    fn interpolation_scopes_same_key_per_service() {
        // Duas ocorrências de APP_PORT (serviços diferentes) com valores
        // diferentes: cada uma resolve pela SUA tabela, sem colisão.
        let yaml = r#"
project:
  name: chiquitos
services:
  - name: api
    source:
      registry: nginx
    env:
      APP_PORT: "${APP_PORT}"
  - name: worker
    source:
      registry: nginx
    env:
      APP_PORT: "${APP_PORT}"
"#;
        let mut m: ProjectManifest = serde_yaml::from_str(yaml).unwrap();
        let env = EnvDoc {
            project: BTreeMap::from([(
                "chiquitos".to_string(),
                ProjectEnvDoc {
                    env: BTreeMap::new(),
                    service: BTreeMap::from([
                        (
                            "api".to_string(),
                            ServiceEnvDoc {
                                env: BTreeMap::from([("APP_PORT".into(), "3000".into())]),
                            },
                        ),
                        (
                            "worker".to_string(),
                            ServiceEnvDoc {
                                env: BTreeMap::from([("APP_PORT".into(), "8080".into())]),
                            },
                        ),
                    ]),
                },
            )]),
            git_provider: BTreeMap::new(),
        };
        assert!(m.interpolate(&env).is_empty());
        let api = m.services.iter().find(|s| s.name == "api").unwrap();
        let worker = m.services.iter().find(|s| s.name == "worker").unwrap();
        assert_eq!(api.env.get("APP_PORT"), Some(&"3000".to_string()));
        assert_eq!(worker.env.get("APP_PORT"), Some(&"8080".to_string()));
    }

    #[test]
    fn service_falls_back_to_project_env() {
        let yaml = r#"
project:
  name: p
services:
  - name: api
    source:
      registry: nginx
    env:
      DATABASE_URL: "${DATABASE_URL}"
"#;
        let mut m: ProjectManifest = serde_yaml::from_str(yaml).unwrap();
        // DATABASE_URL só existe na tabela do PROJETO; o serviço herda via fallback.
        let env = env_doc_with("p", &[("DATABASE_URL", "postgres://x")]);
        assert!(m.interpolate(&env).is_empty());
        assert_eq!(
            m.services[0].env.get("DATABASE_URL"),
            Some(&"postgres://x".to_string())
        );
    }

    #[test]
    fn mem_round_trip() {
        assert_eq!(parse_mem("256m"), Some(256 * 1024 * 1024));
        assert_eq!(parse_mem("2g"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_mem("1024"), Some(1024));
        assert_eq!(humanize_mem(256 * 1024 * 1024), "256m");
        assert_eq!(humanize_mem(2 * 1024 * 1024 * 1024), "2g");
    }

    #[test]
    fn dotenv_format_and_parse_round_trip() {
        let mut map = BTreeMap::new();
        map.insert("SIMPLE".to_string(), "value".to_string());
        map.insert("WITH_SPACE".to_string(), "hello world".to_string());
        let text = format_dotenv(&map);
        assert_eq!(parse_dotenv(&text), map);
    }

    fn sample_project() -> Project {
        Project {
            id: "proj-1".into(),
            name: "acme".into(),
            description: None,
            env_vars: vec![
                EnvVar {
                    key: "LOG_LEVEL".into(),
                    value: EnvVarValue::Plain("info".into()),
                },
                EnvVar {
                    key: "DB_PASS".into(),
                    value: EnvVarValue::Secret("db-pass".into()),
                },
            ],
            env_comments: vec![],
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn redacted_export_never_leaks_plain_values() {
        let project = sample_project();
        let mut doc = EnvDoc::default();
        let manifest =
            ProjectManifest::from_existing_redacted(&project, &[], &BTreeMap::new(), &mut doc);

        // Plain vira placeholder no YAML; valor real só aparece no TOML, na
        // tabela do projeto.
        assert_eq!(manifest.project.env.get("LOG_LEVEL"), Some(&"${LOG_LEVEL}".to_string()));
        assert_eq!(doc.project["acme"].env.get("LOG_LEVEL"), Some(&"info".to_string()));

        // Secret nunca é decifrada: permanece como referência e não entra no TOML.
        assert_eq!(manifest.project.env.get("DB_PASS"), Some(&"secret:db-pass".to_string()));
        assert!(!doc.project["acme"].env.contains_key("DB_PASS"));

        let yaml = serde_yaml::to_string(&manifest).unwrap();
        assert!(!yaml.contains("info"), "valor real de Plain vazou pro YAML: {yaml}");
    }

    #[test]
    fn server_manifest_redacted_scopes_per_project() {
        let p1 = sample_project();
        let mut p2 = sample_project();
        p2.id = "proj-2".into();
        p2.name = "beta".into();
        // Mesma chave (LOG_LEVEL), valores diferentes por projeto: sem colisão.
        p2.env_vars[0].value = EnvVarValue::Plain("debug".into());

        let (server, doc) =
            ServerManifest::from_existing_redacted(&[(p1, vec![]), (p2, vec![])], &BTreeMap::new());
        assert_eq!(server.projects.len(), 2);
        assert_eq!(doc.project["acme"].env.get("LOG_LEVEL"), Some(&"info".to_string()));
        assert_eq!(doc.project["beta"].env.get("LOG_LEVEL"), Some(&"debug".to_string()));
        assert!(!doc.project["acme"].env.contains_key("DB_PASS"));
    }

    #[test]
    fn env_doc_toml_round_trip_and_quotes_names_with_spaces() {
        let doc = EnvDoc {
            project: BTreeMap::from([(
                "Chiquitos".to_string(),
                ProjectEnvDoc {
                    env: BTreeMap::from([("DATABASE_URL".into(), "postgres://x".into())]),
                    service: BTreeMap::from([(
                        "Landing page".to_string(),
                        ServiceEnvDoc {
                            env: BTreeMap::from([("MY__WEIRD__VAR".into(), "ok".into())]),
                        },
                    )]),
                },
            )]),
            git_provider: BTreeMap::new(),
        };
        let text = format_env_doc(&doc);
        // Nome com espaço fica entre aspas (nomes simples ficam crus); var com
        // `__` é só uma folha, sem mangling.
        assert!(text.contains(r#"[project.Chiquitos.service."Landing page".env]"#), "{text}");
        assert!(text.contains("MY__WEIRD__VAR = \"ok\""), "{text}");
        let back = parse_env_doc(&text).unwrap();
        assert_eq!(
            back.project["Chiquitos"].service["Landing page"].env.get("MY__WEIRD__VAR"),
            Some(&"ok".to_string())
        );
    }

    #[test]
    fn git_provider_export_references_by_name_and_redacts_secrets() {
        let provider = GitProvider {
            id: "gp_01ABC".into(),
            kind: GitProviderKind::Gitea,
            name: "meu-gitea".into(),
            base_url: "https://git.example.com".into(),
            auth_mode: GitAuthMode::Pat,
            oauth_client_id: None,
            account: Some(GitAccount { login: "alice".into(), avatar_url: None }),
            created_at: chrono::Utc::now(),
        };
        let providers = BTreeMap::from([(provider.id.clone(), provider.clone())]);

        let svc_spec = ServiceSpec {
            name: "web".into(),
            project_id: "proj-1".into(),
            source: ServiceSource::Git(GitSource {
                url: "https://git.example.com/acme/web".into(),
                provider_id: Some(provider.id.clone()),
                ..GitSource::default()
            }),
            port: 3000,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: vec![],
        };
        let svc = Service {
            id: "svc-1".into(),
            spec: svc_spec,
            status: ServiceStatus::Stopped,
            live_container_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let project = sample_project();
        let mut doc = EnvDoc::default();
        let manifest = ProjectManifest::from_existing_redacted(&project, &[svc], &providers, &mut doc);

        // YAML só carrega o NOME do provider, nunca o ID interno nem segredos.
        let git = manifest.services[0].source.git.as_ref().unwrap();
        assert_eq!(git.provider.as_deref(), Some("meu-gitea"));

        // TOML carrega os dados não-secretos, indexados pelo mesmo nome.
        let pdoc = doc.git_provider.get("meu-gitea").unwrap();
        assert_eq!(pdoc.kind, "gitea");
        assert_eq!(pdoc.base_url, "https://git.example.com");
        assert_eq!(pdoc.auth_mode, "pat");
        let toml_text = format_env_doc(&doc);
        assert!(!toml_text.contains("alice"), "conta/segredo vazou pro TOML: {toml_text}");

        // Import: nome -> ID resolve de volta ao mesmo provider.
        let provider_ids = BTreeMap::from([("meu-gitea".to_string(), provider.id.clone())]);
        let specs = manifest.service_specs("proj-1", &provider_ids);
        let ServiceSource::Git(g) = &specs[0].source else {
            panic!("esperava ServiceSource::Git");
        };
        assert_eq!(g.provider_id.as_deref(), Some(provider.id.as_str()));
    }

    /// `manifest_import::handle` reserializa os `ProjectManifest` já
    /// interpolados de volta pra YAML (`serde_yaml::to_string`) antes de
    /// repassar pra `manifest_apply::handle`, que os reparseia
    /// (`serde_yaml::from_str`). Confere que `source.git.provider` sobrevive a
    /// esse segundo round-trip — não só ao primeiro (export -> texto colado).
    #[test]
    fn git_provider_survives_interpolate_then_reserialize_roundtrip() {
        let yaml = r#"
project:
  name: p
services:
  - name: api
    source:
      git:
        url: https://gitea.example.com/acme/api.git
        branch: main
        provider: Gitea
    port: 3000
"#;
        let mut m: ProjectManifest = serde_yaml::from_str(yaml).unwrap();
        assert!(m.interpolate(&EnvDoc::default()).is_empty());

        // Mesmo passo de manifest_import::handle: reserializa pra YAML texto.
        let reserialized = serde_yaml::to_string(&m).unwrap();
        // Mesmo passo de manifest_apply::handle: reparseia esse texto.
        let reparsed: ProjectManifest = serde_yaml::from_str(&reserialized).unwrap();

        let git = reparsed.services[0].source.git.as_ref().unwrap();
        assert_eq!(
            git.provider.as_deref(),
            Some("Gitea"),
            "provider perdido no round-trip; YAML reserializado:\n{reserialized}"
        );
    }
}
