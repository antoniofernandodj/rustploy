//! Assistente de migração: banco antigo (serviço Compose do projeto) → database
//! gerenciado de um servidor compartilhado. Ver `docs/plano-banco-compartilhado.md` §5.
//!
//! O dump/restore é feito pelas ferramentas oficiais de cada motor, num
//! container efêmero (`docker run --rm`) na rede do projeto, em pipe — os dados
//! não passam pelo daemon nem ocupam disco. Senhas vão por variável de ambiente
//! do processo `docker` (nunca na linha de comando).

use crate::{
    api::{AppState, handlers},
    db::{self, migration::Record},
    deploy::{env_resolve, shared_net},
    env_switch,
    shared_db::{self, Engine},
};
use anyhow::{Result, anyhow, bail};
use chrono::Utc;
use shared::{Migration, MigrationStep, Service};
use std::{collections::HashMap, process::Stdio};
use tokio::process::Command;
use tracing::{info, warn};
use ulid::Ulid;

pub const STEPS: [&str; 7] = [
    "Pré-checagem",
    "Parar escritas",
    "Dump + restore",
    "Verificação",
    "Trocar conexão",
    "Subir serviços",
    "Desligar banco antigo",
];

pub fn new_record(req: &shared::MigrationStartReq, project_id: &str, env_var: &str) -> Record {
    Record {
        m: Migration {
            id: format!("mig_{}", Ulid::new()),
            project_id: project_id.to_string(),
            source_service_id: req.source_service_id.clone(),
            source_database: req.source_database.clone(),
            dest_database_id: req.dest_database_id.clone(),
            env_var: env_var.to_string(),
            status: "Running".into(),
            steps: STEPS
                .iter()
                .map(|n| MigrationStep {
                    name: n.to_string(),
                    state: "pending".into(),
                    detail: String::new(),
                })
                .collect(),
            log: vec![],
            created_at: Utc::now(),
        },
        env_changes: vec![],
        stopped_services: vec![],
    }
}

/// Nome de database de **origem**: vai para scripts shell/SQL, então só o
/// conjunto seguro (o de destino já é validado por `shared_db::validate_name`).
pub fn valid_source_db(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 63
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ── persistência de progresso ───────────────────────────────────────────────

async fn step(state: &AppState, rec: &mut Record, i: usize, st: &str, detail: &str) {
    rec.m.steps[i].state = st.into();
    if !detail.is_empty() {
        rec.m.steps[i].detail = detail.into();
    }
    let _ = db::migration::save(&state.db, rec).await;
}

async fn log(state: &AppState, rec: &mut Record, text: &str) {
    for l in text.lines().filter(|l| !l.trim().is_empty()) {
        rec.m.log.push(l.to_string());
    }
    let n = rec.m.log.len();
    if n > 400 {
        rec.m.log.drain(..n - 400);
    }
    let _ = db::migration::save(&state.db, rec).await;
}

// ── scripts por motor ───────────────────────────────────────────────────────

/// Script `bash` do pipe dump|restore. Variáveis: `ORIGEM_{HOST,PORT,USER,PASS,DB}`,
/// `DESTINO_{…}`.
pub fn pipe_script(e: Engine) -> &'static str {
    match e {
        Engine::Postgres => {
            r#"set -euo pipefail
PGPASSWORD="$ORIGEM_PASS" pg_dump --format=custom --no-owner --no-acl \
    -h "$ORIGEM_HOST" -p "$ORIGEM_PORT" -U "$ORIGEM_USER" "$ORIGEM_DB" \
  | PGPASSWORD="$DESTINO_PASS" pg_restore --no-owner --role="$DESTINO_USER" --exit-on-error \
    -h "$DESTINO_HOST" -p "$DESTINO_PORT" -U "$DESTINO_USER" --dbname="$DESTINO_DB"
"#
        }
        Engine::MySql | Engine::MariaDb => {
            // `mysqldump` (MySQL) × `mariadb-dump` (MariaDB ≥ 10.5): flags diferentes.
            if e == Engine::MySql {
                r#"set -euo pipefail
umask 077
printf '[client]\nuser=%s\npassword=%s\nhost=%s\nport=%s\n' "$ORIGEM_USER" "$ORIGEM_PASS" "$ORIGEM_HOST" "$ORIGEM_PORT" > /tmp/o.cnf
printf '[client]\nuser=%s\npassword=%s\nhost=%s\nport=%s\n' "$DESTINO_USER" "$DESTINO_PASS" "$DESTINO_HOST" "$DESTINO_PORT" > /tmp/d.cnf
mysqldump --defaults-extra-file=/tmp/o.cnf --single-transaction --routines --triggers --events \
    --no-tablespaces --set-gtid-purged=OFF "$ORIGEM_DB" \
  | sed -E 's/DEFINER=`[^`]+`@`[^`]+`/DEFINER=CURRENT_USER/g' \
  | mysql --defaults-extra-file=/tmp/d.cnf "$DESTINO_DB"
"#
            } else {
                r#"set -euo pipefail
umask 077
printf '[client]\nuser=%s\npassword=%s\nhost=%s\nport=%s\n' "$ORIGEM_USER" "$ORIGEM_PASS" "$ORIGEM_HOST" "$ORIGEM_PORT" > /tmp/o.cnf
printf '[client]\nuser=%s\npassword=%s\nhost=%s\nport=%s\n' "$DESTINO_USER" "$DESTINO_PASS" "$DESTINO_HOST" "$DESTINO_PORT" > /tmp/d.cnf
mariadb-dump --defaults-extra-file=/tmp/o.cnf --single-transaction --routines --triggers --events "$ORIGEM_DB" \
  | sed -E 's/DEFINER=`[^`]+`@`[^`]+`/DEFINER=CURRENT_USER/g' \
  | mariadb --defaults-extra-file=/tmp/d.cnf "$DESTINO_DB"
"#
            }
        }
        Engine::Mongo => {
            r#"set -euo pipefail
umask 077
q() { printf "password: '%s'\n" "${1//\'/\'\'}"; }
q "$ORIGEM_PASS" > /tmp/o.yaml
q "$DESTINO_PASS" > /tmp/d.yaml
mongodump --config=/tmp/o.yaml --host="$ORIGEM_HOST" --port="$ORIGEM_PORT" --username="$ORIGEM_USER" \
    --authenticationDatabase=admin --db="$ORIGEM_DB" --archive \
  | mongorestore --config=/tmp/d.yaml --host="$DESTINO_HOST" --port="$DESTINO_PORT" --username="$DESTINO_USER" \
    --authenticationDatabase="$DESTINO_DB" --archive --nsFrom="$ORIGEM_DB.*" --nsTo="$DESTINO_DB.*"
"#
        }
    }
}

/// `(sh, stdin)` que imprime `tabela|contagem` do database `db` (um por linha,
/// ordenado), pelo cliente de administração do servidor — para comparar
/// origem × destino. `count(*)` real: `TABLE_ROWS` do InnoDB é estimativa.
pub fn counts_plan(e: Engine, db: &str) -> (String, String) {
    match e {
        Engine::Postgres => (
            format!(r#"exec psql -U "$POSTGRES_USER" -d {db} -At -v ON_ERROR_STOP=1"#),
            "SELECT format('SELECT %L || ''|'' || count(*) FROM %I.%I', table_schema || '.' || table_name, table_schema, table_name) \
             FROM information_schema.tables WHERE table_type = 'BASE TABLE' \
             AND table_schema NOT IN ('pg_catalog', 'information_schema') ORDER BY table_schema, table_name \\gexec\n"
                .into(),
        ),
        Engine::MySql | Engine::MariaDb => (
            format!(
                r#"B=$(command -v mariadb || command -v mysql); export MYSQL_PWD="$MYSQL_ROOT_PASSWORD"
for t in $($B -uroot -N -e "select table_name from information_schema.tables where table_schema='{db}' and table_type='BASE TABLE' order by 1"); do
  echo "$t|$($B -uroot -N -e "select count(*) from \`{db}\`.\`$t\`")"
done"#
            ),
            String::new(),
        ),
        Engine::Mongo => (
            Engine::Mongo.admin_sh_pub().into(),
            format!(
                "const d = db.getSiblingDB(\"{db}\"); d.getCollectionNames().filter(c => !c.startsWith('system.')).sort()\
                 .forEach(c => print(c + '|' + d.getCollection(c).countDocuments({{}})));\n"
            ),
        ),
    }
}

// ── passos ──────────────────────────────────────────────────────────────────

struct Side {
    host: String,
    port: u16,
    user: String,
    pass: String,
}

/// Credenciais de administração do banco **antigo**, das env vars resolvidas.
fn source_side(e: Engine, svc: &Service, host: &str, env: &HashMap<String, String>) -> Result<Side> {
    let g = |k: &str| env.get(k).cloned().unwrap_or_default();
    let (user, pass) = match e {
        Engine::Postgres => (g("POSTGRES_USER"), g("POSTGRES_PASSWORD")),
        Engine::MySql | Engine::MariaDb => {
            if !g("MYSQL_ROOT_PASSWORD").is_empty() {
                ("root".to_string(), g("MYSQL_ROOT_PASSWORD"))
            } else {
                (g("MYSQL_USER"), g("MYSQL_PASSWORD"))
            }
        }
        Engine::Mongo => (g("MONGO_INITDB_ROOT_USERNAME"), g("MONGO_INITDB_ROOT_PASSWORD")),
    };
    if user.is_empty() {
        bail!("não achei o usuário de administração do banco antigo nas env vars");
    }
    Ok(Side {
        host: host.to_string(),
        port: svc.spec.port,
        user,
        pass,
    })
}

/// Imagem (e tag) do servidor de destino: a mesma traz as ferramentas na versão certa.
fn dest_image(server: &Service) -> Result<String> {
    let shared::ServiceSource::Compose(c) = &server.spec.source else {
        bail!("servidor de destino não é Compose");
    };
    let host = shared::connection::compose_host(
        &c.content,
        c.ingress_service.as_deref(),
        server.spec.db_kind.as_deref(),
    );
    shared::connection::compose_services(&c.content)
        .into_iter()
        .find(|(k, _)| Some(k) == host.as_ref())
        .and_then(|(_, img)| img)
        .ok_or_else(|| anyhow!("não achei a imagem do servidor de destino"))
}

/// Versão maior/menor de uma tag de imagem (`postgres:18` → (18, 0)); `None` se
/// não der para ler (`latest`, digest…) — nesse caso a checagem é pulada.
pub fn image_version(image: &str) -> Option<(u32, u32)> {
    let tag = image.rsplit_once(':')?.1;
    let mut it = tag.split(|c: char| !c.is_ascii_digit());
    let major = it.next()?.parse().ok()?;
    let minor = it.next().and_then(|m| m.parse().ok()).unwrap_or(0);
    Some((major, minor))
}

fn source_image(svc: &Service) -> Option<String> {
    let shared::ServiceSource::Compose(c) = &svc.spec.source else {
        return None;
    };
    let host = shared::connection::compose_host(&c.content, c.ingress_service.as_deref(), svc.spec.db_kind.as_deref());
    shared::connection::compose_services(&c.content)
        .into_iter()
        .find(|(k, _)| Some(k) == host.as_ref())
        .and_then(|(_, i)| i)
}

/// Serviços do projeto que usam o banco antigo: os rodando que referenciam a
/// env var da conexão ou o hostname do banco antigo.
async fn dependents(
    state: &AppState,
    source: &Service,
    host: &str,
    env_var: &str,
) -> Result<Vec<Service>> {
    let mut out = vec![];
    for s in db::services::list(&state.db, &source.spec.project_id).await? {
        if s.id == source.id || s.spec.db_kind.is_some() {
            continue;
        }
        if !matches!(s.status, shared::ServiceStatus::Running | shared::ServiceStatus::Degraded) {
            continue;
        }
        let env = env_resolve::resolve(&state.db, &state.secrets, &s).await?;
        if env.iter().any(|(k, v)| k == env_var || v.contains(host)) {
            out.push(s);
        }
    }
    Ok(out)
}

async fn docker_run_pipe(
    engine: Engine,
    image: &str,
    network: &str,
    origem: &Side,
    origem_db: &str,
    destino: &Side,
    destino_db: &str,
) -> Result<String> {
    let mut cmd = Command::new("docker");
    cmd.args(["run", "--rm", "--network", network]);
    let vars = [
        ("ORIGEM_HOST", origem.host.clone()),
        ("ORIGEM_PORT", origem.port.to_string()),
        ("ORIGEM_USER", origem.user.clone()),
        ("ORIGEM_PASS", origem.pass.clone()),
        ("ORIGEM_DB", origem_db.to_string()),
        ("DESTINO_HOST", destino.host.clone()),
        ("DESTINO_PORT", destino.port.to_string()),
        ("DESTINO_USER", destino.user.clone()),
        ("DESTINO_PASS", destino.pass.clone()),
        ("DESTINO_DB", destino_db.to_string()),
    ];
    for (k, v) in &vars {
        // `-e NOME` sem valor: o docker lê do ambiente dele — nada no argv.
        cmd.args(["-e", k]).env(k, v);
    }
    cmd.args(["--entrypoint", "bash", image, "-c", pipe_script(engine)]);
    let out = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| anyhow!("falha ao executar docker: {e}"))?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        bail!("{}", text.trim());
    }
    Ok(text)
}

/// Roda a migração inteira. Erro em qualquer passo → desfaz (até o passo 6).
pub async fn run(state: AppState, mut rec: Record) {
    let mut cur = 0usize;
    let res = run_inner(&state, &mut rec, &mut cur).await;
    match res {
        Ok(()) => {
            rec.m.status = "Completed".into();
            let _ = db::migration::save(&state.db, &rec).await;
            info!(id = %rec.m.id, "migração concluída");
        }
        Err(e) => {
            warn!(id = %rec.m.id, step = cur, error = %e, "migração falhou");
            step(&state, &mut rec, cur, "failed", &e.to_string()).await;
            for i in cur + 1..rec.m.steps.len() {
                rec.m.steps[i].state = "skipped".into();
            }
            // Desfaz o que já foi feito: env de volta, serviços parados sobem.
            if let Err(u) = undo(&state, &mut rec, false).await {
                log(&state, &mut rec, &format!("rollback automático falhou: {u}")).await;
            }
            rec.m.status = "Failed".into();
            let _ = db::migration::save(&state.db, &rec).await;
        }
    }
}

async fn run_inner(state: &AppState, rec: &mut Record, cur: &mut usize) -> Result<()> {
    let source = db::services::get(&state.db, &rec.m.source_service_id)
        .await?
        .ok_or_else(|| anyhow!("banco antigo não encontrado"))?;
    let dest = db::managed_database::get(&state.db, &rec.m.dest_database_id)
        .await?
        .ok_or_else(|| anyhow!("database de destino não encontrado"))?;
    let server = db::services::get(&state.db, &dest.server_service_id)
        .await?
        .ok_or_else(|| anyhow!("servidor de destino não encontrado"))?;
    let engine = Engine::from_kind(source.spec.db_kind.as_deref()).ok_or_else(|| anyhow!("motor não suportado"))?;
    let network = db::projects::network_name(&state.db, &source.spec.project_id).await?;
    let src_db = rec.m.source_database.clone();

    // 1. Pré-checagem ────────────────────────────────────────────────────────
    *cur = 0;
    step(state, rec, 0, "running", "").await;
    if Engine::from_kind(server.spec.db_kind.as_deref()) != Some(engine) {
        bail!("origem e destino precisam ser do mesmo motor (MySQL ↔ MariaDB também não é permitido)");
    }
    if let (Some(s), Some(d)) = (
        source_image(&source).and_then(|i| image_version(&i)),
        dest_image(&server).ok().and_then(|i| image_version(&i)),
    ) && d < s
    {
        bail!("o servidor de destino ({}.{}) é mais antigo que a origem ({}.{})", d.0, d.1, s.0, s.1);
    }
    let src_cid = shared_net::server_container(&state.docker.inner, &source)
        .await
        .ok_or_else(|| anyhow!("o banco antigo não está no ar"))?;
    let dst_cid = shared_net::server_container(&state.docker.inner, &server)
        .await
        .ok_or_else(|| anyhow!("o servidor compartilhado não está no ar"))?;
    shared_db::wait_ready(engine, &dst_cid, 10).await?;
    let (sh, stdin) = counts_plan(engine, &dest.name);
    let dest_now = shared_db::exec_sh(engine, &dst_cid, &sh, &stdin).await?;
    if !dest_now.trim().is_empty() {
        bail!("o database de destino não está vazio; crie um novo ou apague o conteúdo");
    }
    let (sh, stdin) = counts_plan(engine, &src_db);
    // Lê a origem já aqui: valida que o database existe e as credenciais servem.
    let env: HashMap<String, String> = env_resolve::resolve(&state.db, &state.secrets, &source).await?.into_iter().collect();
    let src_host = shared::connection::compose_host(
        match &source.spec.source {
            shared::ServiceSource::Compose(c) => &c.content,
            _ => "",
        },
        match &source.spec.source {
            shared::ServiceSource::Compose(c) => c.ingress_service.as_deref(),
            _ => None,
        },
        source.spec.db_kind.as_deref(),
    )
    .unwrap_or_else(|| shared::app_network_alias(&source.spec.name));
    let _ = shared_db::exec_sh(engine, &src_cid, &sh, &stdin)
        .await
        .map_err(|e| anyhow!("não consegui ler o database de origem \"{src_db}\": {e}"))?;
    step(state, rec, 0, "ok", "motor, versão, origem e destino conferidos; destino vazio").await;

    // 2. Parar escritas ──────────────────────────────────────────────────────
    *cur = 1;
    step(state, rec, 1, "running", "").await;
    let deps = dependents(state, &source, &src_host, &rec.m.env_var).await?;
    for d in &deps {
        rec.stopped_services.push(d.id.clone());
    }
    let _ = db::migration::save(&state.db, rec).await;
    for d in &deps {
        let r = handlers::service_stop::handle(state.clone(), d.id.clone()).await;
        if let shared::Response::Err { message, .. } = r {
            bail!("falha ao parar {}: {message}", d.spec.name);
        }
    }
    step(state, rec, 1, "ok", &format!("{} serviço(s) parado(s)", deps.len())).await;

    // Contagens da origem, já sem escritas.
    let (sh, stdin) = counts_plan(engine, &src_db);
    let counts_src = shared_db::exec_sh(engine, &src_cid, &sh, &stdin).await?;

    // 3. Dump + restore ──────────────────────────────────────────────────────
    *cur = 2;
    step(state, rec, 2, "running", "").await;
    let origem = source_side(engine, &source, &src_host, &env)?;
    let destino = Side {
        host: shared_net::alias_of(&server),
        port: server.spec.port,
        user: dest.name.clone(),
        pass: dest.password.clone(),
    };
    let image = dest_image(&server)?;
    let out = docker_run_pipe(engine, &image, &network, &origem, &src_db, &destino, &dest.name).await?;
    log(state, rec, &out).await;
    step(state, rec, 2, "ok", "").await;

    // 4. Verificação ─────────────────────────────────────────────────────────
    *cur = 3;
    step(state, rec, 3, "running", "").await;
    let (sh, stdin) = counts_plan(engine, &dest.name);
    let counts_dst = shared_db::exec_sh(engine, &dst_cid, &sh, &stdin).await?;
    let norm = |s: &str| -> Vec<String> {
        let mut v: Vec<String> = s.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
        v.sort();
        v
    };
    let (a, b) = (norm(&counts_src), norm(&counts_dst));
    if a != b {
        log(state, rec, &format!("origem:\n{}\ndestino:\n{}", a.join("\n"), b.join("\n"))).await;
        bail!("contagens diferentes entre origem e destino (veja o log)");
    }
    step(state, rec, 3, "ok", &format!("{} tabela(s)/coleção(ões) conferidas", a.len())).await;

    // 5. Trocar a conexão ────────────────────────────────────────────────────
    *cur = 4;
    step(state, rec, 4, "running", "").await;
    let view = handlers::managed_database::to_view(&server, engine, &dest);
    rec.env_changes = env_switch::switch(&state.db, &rec.m.project_id, &rec.m.env_var, &view.connection_url).await?;
    step(state, rec, 4, "ok", &format!("{} aponta para o banco novo (valor antigo guardado)", rec.m.env_var)).await;

    // 6. Subir os serviços ───────────────────────────────────────────────────
    *cur = 5;
    step(state, rec, 5, "running", "").await;
    for id in rec.stopped_services.clone() {
        let r = handlers::deploy_start::handle(state.clone(), id.clone()).await;
        if let shared::Response::Err { message, .. } = r {
            bail!("falha ao subir {id}: {message}");
        }
    }
    step(state, rec, 5, "ok", "redeploy enfileirado; acompanhe o healthcheck de cada serviço").await;

    // 7. Período de observação: o banco antigo fica parado, não apagado. ──────
    *cur = 6;
    step(state, rec, 6, "running", "").await;
    let r = handlers::service_stop::handle(state.clone(), source.id.clone()).await;
    if let shared::Response::Err { message, .. } = r {
        bail!("falha ao parar o banco antigo: {message}");
    }
    step(state, rec, 6, "ok", "banco antigo parado, mantido para rollback").await;
    Ok(())
}

/// Volta tudo: env var antiga, banco antigo de pé (se foi parado) e serviços
/// parados de novo no ar. `source_stopped`: o passo 7 já parou o banco antigo.
pub async fn undo(state: &AppState, rec: &mut Record, source_stopped: bool) -> Result<()> {
    if !rec.env_changes.is_empty() {
        env_switch::restore(&state.db, &rec.m.project_id, &rec.env_changes).await?;
        rec.env_changes.clear();
    }
    if source_stopped {
        let r = handlers::deploy_start::handle(state.clone(), rec.m.source_service_id.clone()).await;
        if let shared::Response::Err { message, .. } = r {
            bail!("falha ao subir o banco antigo: {message}");
        }
    }
    for id in rec.stopped_services.clone() {
        let _ = handlers::deploy_start::handle(state.clone(), id).await;
    }
    rec.stopped_services.clear();
    let _ = db::migration::save(&state.db, rec).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versao_da_imagem() {
        assert_eq!(image_version("postgres:18"), Some((18, 0)));
        assert_eq!(image_version("mysql:8.4"), Some((8, 4)));
        assert_eq!(image_version("mariadb:11.4.2"), Some((11, 4)));
        assert_eq!(image_version("postgres:latest"), None);
        assert_eq!(image_version("postgres"), None);
    }

    #[test]
    fn nome_de_origem_seguro() {
        assert!(valid_source_db("db_2024"));
        for bad in ["", "a b", "a;b", "a'b", "a`b", "a-b"] {
            assert!(!valid_source_db(bad), "{bad}");
        }
    }

    #[test]
    fn scripts_sem_senha_em_argumento() {
        for e in [Engine::Postgres, Engine::MySql, Engine::MariaDb, Engine::Mongo] {
            let s = pipe_script(e);
            assert!(s.starts_with("set -euo pipefail"));
            assert!(!s.contains("--password") && !s.contains("-p\"$"), "{e:?}");
        }
        assert!(pipe_script(Engine::MariaDb).contains("mariadb-dump"));
        assert!(!pipe_script(Engine::MariaDb).contains("set-gtid-purged"));
    }
}
