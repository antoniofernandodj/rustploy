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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
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

    /// Converte cada serviço do manifesto numa [`ServiceSpec`] já vinculada ao projeto.
    pub fn service_specs(&self, project_id: &str) -> Vec<ServiceSpec> {
        self.services
            .iter()
            .map(|s| s.to_spec(project_id))
            .collect()
    }

    /// Substitui `${VAR}` em todos os valores de env (projeto + serviços) usando `lookup`.
    /// Retorna a lista de variáveis não resolvidas (para o cliente avisar/abortar).
    pub fn interpolate<F>(&mut self, lookup: &F) -> Vec<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut missing = Vec::new();
        interpolate_map(&mut self.project.env, lookup, &mut missing);
        for s in &mut self.services {
            interpolate_map(&mut s.env, lookup, &mut missing);
        }
        missing
    }

    /// Reconstrói um manifesto a partir do estado atual no banco (para `export`).
    /// Segredos são emitidos como `secret:NOME`, nunca o valor decifrado.
    pub fn from_existing(project: &Project, services: &[Service]) -> Self {
        ProjectManifest {
            api_version: Some(API_VERSION.to_string()),
            project: ProjectMeta {
                name: project.name.clone(),
                description: project.description.clone(),
                env: env_vars_to_map(&project.env_vars),
            },
            services: services.iter().map(ServiceManifest::from_spec).collect(),
        }
    }
}

impl ServiceManifest {
    pub fn to_spec(&self, project_id: &str) -> ServiceSpec {
        ServiceSpec {
            name: self.name.clone(),
            project_id: project_id.to_string(),
            source: self.source.to_source(),
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

    pub fn from_spec(svc: &Service) -> Self {
        let spec = &svc.spec;
        ServiceManifest {
            name: spec.name.clone(),
            source: SourceManifest::from_source(&spec.source),
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
    fn to_source(&self) -> ServiceSource {
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
                provider_id: g.provider_id.clone(),
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

    fn from_source(src: &ServiceSource) -> Self {
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
                    provider_id: g.provider_id.clone(),
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

fn interpolate_map<F>(map: &mut BTreeMap<String, String>, lookup: &F, missing: &mut Vec<String>)
where
    F: Fn(&str) -> Option<String>,
{
    for value in map.values_mut() {
        *value = interpolate_str(value, lookup, missing);
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

        let specs = m.service_specs("proj-1");
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

    #[test]
    fn interpolation_resolves_and_reports_missing() {
        let mut m: ProjectManifest = serde_yaml::from_str(SAMPLE).unwrap();
        let missing = m.interpolate(&|k| (k == "DB_PASS").then(|| "s3cr3t".to_string()));
        assert!(missing.is_empty());
        let env = m.project_fields().2;
        let db = env.iter().find(|e| e.key == "DB_PASS").unwrap();
        assert!(matches!(&db.value, EnvVarValue::Plain(v) if v == "s3cr3t"));

        let mut m2: ProjectManifest = serde_yaml::from_str(SAMPLE).unwrap();
        let missing2 = m2.interpolate(&|_| None);
        assert_eq!(missing2, vec!["DB_PASS".to_string()]);
    }

    #[test]
    fn mem_round_trip() {
        assert_eq!(parse_mem("256m"), Some(256 * 1024 * 1024));
        assert_eq!(parse_mem("2g"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_mem("1024"), Some(1024));
        assert_eq!(humanize_mem(256 * 1024 * 1024), "256m");
        assert_eq!(humanize_mem(2 * 1024 * 1024 * 1024), "2g");
    }
}
