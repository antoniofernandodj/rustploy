# Índice: importer

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/importer/src/

### main.rs
struct Cli { command }
enum Commands { Dokploy }
fn main() -> Result<()>

## crates/importer/src/sink/

### mod.rs
fn write_sql_file(path, data) -> Result<()>
fn write_to_db(data) -> Result<()>

## crates/importer/src/source/

### dokploy.rs
struct DokployProject { id, name, description }
struct DokployApplication { id, name, source_type, repository, owner, branch, _build_path, dockerfile, docker_context_path, docker_build_stage, custom_git_url, custom_git_branch, build_type, env, project_id, gitea_repository, gitea_owner }
struct DokployCompose { id, name, compose_file, project_id, env }
struct DokployDomain { host, https, port, application_id, compose_id }
struct DokployData { projects, applications, composes, domains }
struct DokploySource { pool }
impl DokploySource
  fn new(pg_url) -> Result<Self>
  fn fetch_all() -> Result<DokployData>

### mod.rs

## crates/importer/src/transform/

### dokploy.rs
fn transform(data, gitea_url) -> (TransformedData, Report)
fn parse_env(env_str) -> Vec<EnvVar>
fn normalize_path(path) -> String

### mod.rs
struct TransformedData { projects, services }

## crates/importer/src/

### warnings.rs
enum Severity { Blocking, Warning, Info }
struct Issue { severity, scope, code, message, hint }
struct Report { issues }
impl Report
  fn blocking(scope, code, msg)
  fn warn(scope, code, msg, hint)
  fn push(severity, scope, code, msg, hint)
  fn has_blocking() -> bool
  fn print()
