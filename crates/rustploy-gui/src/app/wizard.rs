//! Novo-serviço wizard (Application / Database / Compose / Template) — port do
//! fluxo da crate `remote-client` para o modelo contexto+ações do glacier-ui.
//! Este módulo concentra o que não é UI: o catálogo de bancos, a geração de
//! senha, o compose gerado por banco, os JSON builders das listas do wizard e a
//! montagem do `ServiceSpec` final enviado em `Command::ServiceCreate`.

use shared::templates::{self, Template};
use shared::{
    ComposeSource, EnvVar, EnvVarValue, Healthcheck, ResourceLimits, ServiceSource, ServiceSpec,
};

// ── Bancos de dados ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    MongoDb,
    Postgres,
    MariaDb,
    MySql,
    Redis,
}

impl DbKind {
    pub const ALL: &'static [DbKind] = &[
        Self::MongoDb,
        Self::Postgres,
        Self::MariaDb,
        Self::MySql,
        Self::Redis,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::MongoDb => "MongoDB",
            Self::Postgres => "PostgreSQL",
            Self::MariaDb => "MariaDB",
            Self::MySql => "MySQL",
            Self::Redis => "Redis",
        }
    }

    pub fn default_image(self) -> &'static str {
        match self {
            Self::MongoDb => "mongo:8",
            Self::Postgres => "postgres:18",
            Self::MariaDb => "mariadb:11",
            Self::MySql => "mysql:8",
            Self::Redis => "redis:7",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::MongoDb => 27017,
            Self::Postgres => 5432,
            Self::MariaDb | Self::MySql => 3306,
            Self::Redis => 6379,
        }
    }

    /// Id estável usado no `ServiceSpec.db_kind` e nas ações/contexto do wizard.
    pub fn kind_id(self) -> &'static str {
        match self {
            Self::MongoDb => "mongodb",
            Self::Postgres => "postgres",
            Self::MariaDb => "mariadb",
            Self::MySql => "mysql",
            Self::Redis => "redis",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "postgres" => Some(Self::Postgres),
            "mongodb" => Some(Self::MongoDb),
            "mariadb" => Some(Self::MariaDb),
            "mysql" => Some(Self::MySql),
            "redis" => Some(Self::Redis),
            _ => None,
        }
    }

    /// Valor inicial do campo "User" do formulário (o usuário pode trocar).
    pub fn default_user(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MongoDb => "root",
            Self::MariaDb | Self::MySql => "user",
            Self::Redis => "",
        }
    }

    /// Quais campos o formulário deste banco mostra.
    pub fn has_db_name(self) -> bool {
        matches!(self, Self::Postgres | Self::MariaDb | Self::MySql)
    }
    pub fn has_user(self) -> bool {
        !matches!(self, Self::Redis)
    }
    pub fn has_root_password(self) -> bool {
        matches!(self, Self::MariaDb | Self::MySql)
    }
    pub fn has_replica_sets(self) -> bool {
        matches!(self, Self::MongoDb)
    }
}

/// Senha aleatória URL-safe (mesmo gerador do antigo `remote-client`):
/// splitmix64 semeado no relógio — suficiente para credenciais iniciais de um
/// banco local, que o usuário pode trocar antes de criar.
pub fn token_urlsafe(n: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345);
    let mut state = seed ^ (seed << 13) ^ (seed >> 7) ^ 0x9e3779b97f4a7c15;
    (0..n)
        .map(|_| {
            // splitmix64
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            ALPHABET[(z % ALPHABET.len() as u64) as usize] as char
        })
        .collect()
}

// ── JSON builders (listas renderizadas pelos `ForEach` do wizard) ───────────

/// Linhas do passo "escolha o banco": `{id, label, image}`.
pub fn db_rows_json() -> String {
    let rows: Vec<serde_json::Value> = DbKind::ALL
        .iter()
        .map(|d| {
            serde_json::json!({
                "id": d.kind_id(),
                "label": d.label(),
                "image": d.default_image(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Templates filtrados pelo termo de busca:
/// `{id, name, description, logo, logo_kind}`.
///
/// `logo` é o caminho (relativo à raiz do workspace, de onde o rustploy-gui roda)
/// do arquivo em `crates/shared/templates/blueprints/<id>/<arquivo>`; `logo_kind`
/// é `"svg"` (vetor), `"img"` (raster) ou `"none"` — o `TemplateRow` escolhe o
/// widget certo (`Svg` vs `Image`) por esse campo.
pub fn templates_json(search: &str) -> String {
    let rows: Vec<serde_json::Value> = templates::filtered(search)
        .into_iter()
        .map(|t| {
            let (logo, logo_kind) = template_logo(t);
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "logo": logo,
                "logo_kind": logo_kind,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Caminho e tipo do logo do template. Vazio → `("", "none")`.
fn template_logo(t: &'static Template) -> (String, &'static str) {
    if t.logo.is_empty() {
        return (String::new(), "none");
    }
    let ext = t.logo.rsplit('.').next().unwrap_or("").to_lowercase();
    let kind = match ext.as_str() {
        "svg" => "svg",
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico" => "img",
        _ => "none",
    };
    let path = format!("crates/shared/templates/blueprints/{}/{}", t.id, t.logo);
    (path, kind)
}

pub fn find_template(id: &str) -> Option<&'static Template> {
    templates::find(id)
}

/// Variáveis que o usuário preenche: só os domínios (`${domain}`) do template —
/// segredos e chaves são gerados no `render`. Formato `{idx, label, placeholder}`;
/// o valor digitado vive na chave de contexto `ns_tv_<idx>` (o KDL interpola
/// `value="ns_tv_{v.idx}"` por item do `ForEach`).
pub fn template_vars_json(t: &'static Template) -> String {
    let rows: Vec<serde_json::Value> = templates::editable_vars(t)
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let label = if v.key == "main_domain" {
                "Domínio".to_string()
            } else {
                v.key.replace('_', " ")
            };
            serde_json::json!({
                "idx": i.to_string(),
                "label": label,
                "placeholder": "meuapp.exemplo.com",
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Slug default do nome do serviço a partir do nome do template.
pub fn template_slug(t: &Template) -> String {
    t.name.to_lowercase().replace(' ', "-")
}

// ── Montagem do ServiceSpec ──────────────────────────────────────────────────

fn base_spec(
    name: String,
    project_id: String,
    source: ServiceSource,
    port: u16,
    env_vars: Vec<EnvVar>,
    db_kind: Option<String>,
) -> ServiceSpec {
    ServiceSpec {
        name,
        project_id,
        source,
        port,
        host_port: None,
        domain: None,
        tls_enabled: false,
        env_vars,
        env_comments: vec![],
        volumes: vec![],
        healthcheck: Healthcheck::default(),
        replicas: 1,
        resources: ResourceLimits::default(),
        run_command: None,
        run_args: vec![],
        db_kind,
    }
}

/// Application: nasce como Registry de imagem vazia — o usuário configura a
/// origem (Git/imagem) na aba General do serviço, como no `remote-client`.
pub fn app_spec(name: String, project_id: String) -> ServiceSpec {
    base_spec(
        name,
        project_id,
        ServiceSource::Registry { image: String::new() },
        80,
        vec![],
        None,
    )
}

/// Compose stack: nasce com compose vazio, a configurar dentro do serviço.
pub fn compose_spec(name: String, project_id: String) -> ServiceSpec {
    base_spec(
        name,
        project_id,
        ServiceSource::Compose(ComposeSource { content: String::new() }),
        80,
        vec![],
        None,
    )
}

/// Campos do formulário de banco, lidos do contexto por `Root` no `ns_create`.
pub struct DbFormInput {
    pub db_name: String,
    pub user: String,
    pub password: String,
    pub root_password: String,
    pub image: String,
    pub use_replica_sets: bool,
}

pub fn db_spec(db: DbKind, name: String, project_id: String, f: &DbFormInput) -> ServiceSpec {
    let image = if f.image.trim().is_empty() {
        db.default_image().to_string()
    } else {
        f.image.trim().to_string()
    };
    base_spec(
        name,
        project_id,
        ServiceSource::Compose(ComposeSource { content: db_compose(db, &image, f) }),
        db.default_port(),
        db_env_vars(db, f),
        Some(db.kind_id().to_string()),
    )
}

fn db_env_vars(db: DbKind, f: &DbFormInput) -> Vec<EnvVar> {
    let plain = |k: &str, v: &str| EnvVar {
        key: k.to_string(),
        value: EnvVarValue::Plain(v.to_string()),
    };
    match db {
        DbKind::Postgres => vec![
            plain("POSTGRES_DB", &f.db_name),
            plain("POSTGRES_USER", &f.user),
            plain("POSTGRES_PASSWORD", &f.password),
        ],
        DbKind::MongoDb => {
            let mut vars = vec![
                plain("MONGO_INITDB_ROOT_USERNAME", &f.user),
                plain("MONGO_INITDB_ROOT_PASSWORD", &f.password),
            ];
            if f.use_replica_sets {
                vars.push(plain("MONGO_REPLICA_SET_NAME", "rs0"));
            }
            vars
        }
        DbKind::MariaDb | DbKind::MySql => vec![
            plain("MYSQL_DATABASE", &f.db_name),
            plain("MYSQL_USER", &f.user),
            plain("MYSQL_PASSWORD", &f.password),
            plain("MYSQL_ROOT_PASSWORD", &f.root_password),
        ],
        DbKind::Redis => {
            if f.password.is_empty() {
                vec![]
            } else {
                vec![plain("REDIS_PASSWORD", &f.password)]
            }
        }
    }
}

fn db_compose(db: DbKind, img: &str, f: &DbFormInput) -> String {
    match db {
        DbKind::Postgres => format!(
            "services:\n  postgres:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      POSTGRES_DB: ${{POSTGRES_DB}}\n      POSTGRES_USER: ${{POSTGRES_USER}}\n      POSTGRES_PASSWORD: ${{POSTGRES_PASSWORD}}\n    volumes:\n      - pgdata:/var/lib/postgresql\n\nvolumes:\n  pgdata:\n"
        ),
        DbKind::MongoDb => {
            let replica = if f.use_replica_sets {
                "      MONGO_REPLICA_SET_NAME: rs0\n"
            } else {
                ""
            };
            format!(
                "services:\n  mongo:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MONGO_INITDB_ROOT_USERNAME: ${{MONGO_INITDB_ROOT_USERNAME}}\n      MONGO_INITDB_ROOT_PASSWORD: ${{MONGO_INITDB_ROOT_PASSWORD}}\n{replica}    volumes:\n      - mongodata:/data/db\n\nvolumes:\n  mongodata:\n"
            )
        }
        DbKind::MariaDb => format!(
            "services:\n  mariadb:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MYSQL_DATABASE: ${{MYSQL_DATABASE}}\n      MYSQL_USER: ${{MYSQL_USER}}\n      MYSQL_PASSWORD: ${{MYSQL_PASSWORD}}\n      MYSQL_ROOT_PASSWORD: ${{MYSQL_ROOT_PASSWORD}}\n    volumes:\n      - mariadbdata:/var/lib/mysql\n\nvolumes:\n  mariadbdata:\n"
        ),
        DbKind::MySql => format!(
            "services:\n  mysql:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MYSQL_DATABASE: ${{MYSQL_DATABASE}}\n      MYSQL_USER: ${{MYSQL_USER}}\n      MYSQL_PASSWORD: ${{MYSQL_PASSWORD}}\n      MYSQL_ROOT_PASSWORD: ${{MYSQL_ROOT_PASSWORD}}\n    volumes:\n      - mysqldata:/var/lib/mysql\n\nvolumes:\n  mysqldata:\n"
        ),
        DbKind::Redis => {
            let cmd = if f.password.is_empty() {
                String::new()
            } else {
                "    command: redis-server --requirepass ${REDIS_PASSWORD}\n".to_string()
            };
            format!(
                "services:\n  redis:\n    image: {img}\n    restart: unless-stopped\n{cmd}    volumes:\n      - redisdata:/data\n\nvolumes:\n  redisdata:\n"
            )
        }
    }
}

/// Template: resolve as variáveis (gerando segredos), monta o `.env` e o domínio
/// e devolve um serviço Compose pronto. `values` são os domínios digitados pelo
/// usuário, na ordem de `templates::editable_vars` (ver `template_vars_json`).
pub fn template_spec(
    t: &'static Template,
    name: String,
    project_id: String,
    values: &[String],
) -> ServiceSpec {
    let name = if name.trim().is_empty() { template_slug(t) } else { name };
    let user: Vec<(String, String)> = templates::editable_vars(t)
        .iter()
        .zip(values.iter())
        .map(|(v, val)| (v.key.to_string(), val.clone()))
        .collect();
    let rendered = templates::render(t, &user);
    let env_vars = rendered
        .env
        .into_iter()
        .map(|(key, value)| EnvVar { key, value: EnvVarValue::Plain(value) })
        .collect();
    let mut spec = base_spec(
        name,
        project_id,
        ServiceSource::Compose(ComposeSource { content: rendered.compose }),
        rendered.port,
        env_vars,
        None,
    );
    spec.domain = rendered.domain.filter(|d| !d.is_empty());
    spec
}
