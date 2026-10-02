# Índice: importer

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/importer/src/

### main.rs — CLI do importer: migra projetos e serviços de outra plataforma (hoje, Dokploy) para o rustploy.
struct Cli { command }
enum Commands { Dokploy }
fn main() -> Result<()>

## rustploy-daemon/crates/importer/src/sink/

### mod.rs — Grava os dados transformados no banco do rustploy (ou num arquivo SQL), casando projetos pelo nome.
fn write_sql_file(path, data) -> Result<()>
fn write_to_db(data) -> Result<()> — Grava os dados transformados direto no SQLite do rustploy: atualiza ou cria projetos pelo nome e sub…

## rustploy-daemon/crates/importer/src/source/

### dokploy.rs — Leitura dos projetos, aplicações, stacks Compose e domínios direto do Postgres do Dokploy.
struct DokployProject { id, name, description }
struct DokployApplication { id, name, source_type, repository, owner, branch, _build_path, dockerfile, docker_context_path, docker_build_stage, custom_git_url, custom_git_branch, build_type, env, project_id, gitea_repository, gitea_owner }
struct DokployCompose { id, name, compose_file, project_id, env }
struct DokployDomain { host, https, port, application_id, compose_id }
struct DokployData { projects, applications, composes, domains }
struct DokploySource { pool }
impl DokploySource
  fn new(pg_url) -> Result<Self>
  fn fetch_all() -> Result<DokployData>

### mod.rs — Fontes de dados do importer, uma por plataforma de origem.

## rustploy-daemon/crates/importer/src/transform/

### dokploy.rs — Converte os dados do Dokploy em projetos e serviços do rustploy, anotando o que não dá para migrar n…
fn transform(data, gitea_url) -> (TransformedData, Report)
fn parse_env(env_str) -> Vec<EnvVar>
fn normalize_path(path) -> String

### mod.rs — Conversão dos dados de origem para os modelos do rustploy (`TransformedData`).
struct TransformedData { projects, services }

## rustploy-daemon/crates/importer/src/

### warnings.rs — `Report` do importer: problemas encontrados na migração, por severidade (bloqueante, aviso, informaç…
enum Severity { Blocking, Warning, Info }
struct Issue { severity, scope, code, message, hint }
struct Report { issues }
impl Report
  fn blocking(scope, code, msg)
  fn warn(scope, code, msg, hint)
  fn push(severity, scope, code, msg, hint)
  fn has_blocking() -> bool
  fn print()
