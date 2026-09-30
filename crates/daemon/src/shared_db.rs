//! Provisionamento de database + usuário dentro de um servidor de banco
//! compartilhado, por motor, via `docker exec` no container do servidor.
//!
//! Tudo roda com o cliente que já vem na imagem do banco (nada é instalado no
//! host). SQL/JS e senhas viajam por **stdin**, nunca na linha de comando do
//! `docker` (que o `ps` do host mostraria). As credenciais de administrador
//! vêm das env vars do próprio container (as mesmas que o wizard grava).
//! Ver `docs/plano-banco-compartilhado.md` §4.2–4.3.

use anyhow::{Result, anyhow, bail};
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Postgres,
    MySql,
    MariaDb,
    Mongo,
}

impl Engine {
    pub fn from_kind(kind: Option<&str>) -> Option<Self> {
        match kind?.to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" => Some(Self::Postgres),
            "mysql" => Some(Self::MySql),
            "mariadb" => Some(Self::MariaDb),
            "mongodb" | "mongo" => Some(Self::Mongo),
            _ => None,
        }
    }

    /// Nome da env var padrão da connection string no projeto consumidor.
    pub fn default_env_var(self) -> &'static str {
        match self {
            Self::Mongo => "MONGODB_URI",
            _ => "DATABASE_URL",
        }
    }

    /// `db_kind` que `shared::connection` entende.
    pub fn kind_id(self) -> &'static str {
        match self {
            Self::Postgres => "postgres",
            Self::MySql => "mysql",
            Self::MariaDb => "mariadb",
            Self::Mongo => "mongodb",
        }
    }

    /// Script `sh` que abre o cliente de administração (lê o SQL/JS do stdin).
    fn admin_sh(self) -> &'static str {
        match self {
            Self::Postgres => r#"exec psql -U "$POSTGRES_USER" -d postgres -v ON_ERROR_STOP=1 -q"#,
            // `mariadb` existe nas imagens ≥ 11; `mysql` nas demais.
            Self::MySql | Self::MariaDb => {
                r#"B=$(command -v mariadb || command -v mysql); MYSQL_PWD="$MYSQL_ROOT_PASSWORD" exec "$B" -uroot"#
            }
            Self::Mongo => {
                r#"B=$(command -v mongosh || command -v mongo); exec "$B" --quiet -u "$MONGO_INITDB_ROOT_USERNAME" -p "$MONGO_INITDB_ROOT_PASSWORD" --authenticationDatabase admin"#
            }
        }
    }
}

/// Nome de database/usuário: `[a-z][a-z0-9_]*`, até 32 chars (limite do usuário
/// no MySQL). É a **única** barreira antes de o nome entrar no SQL — nada de
/// texto livre concatenado.
pub fn validate_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 32
        && name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !ok {
        bail!("nome inválido: use [a-z0-9_], começando por letra, até 32 caracteres");
    }
    if matches!(
        name,
        "postgres" | "template0" | "template1" | "root" | "admin" | "mysql" | "sys" | "local" | "config"
    ) {
        bail!("\"{name}\" é um nome reservado do servidor de banco");
    }
    Ok(())
}

/// Senha aleatória alfanumérica (segura em SQL, JS e URL sem escapes).
pub fn generate_password() -> Result<String> {
    use std::io::Read;
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut buf = [0u8; 28];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut buf)?;
    Ok(buf
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect())
}

/// Script de criação: database, usuário com acesso só a ele e limites.
pub fn create_script(
    e: Engine,
    name: &str,
    pass: &str,
    conn_limit: Option<u32>,
    stmt_timeout_ms: Option<u32>,
) -> String {
    match e {
        Engine::Postgres => {
            let mut s = format!(
                "CREATE ROLE \"{name}\" LOGIN PASSWORD '{pass}';\n\
                 CREATE DATABASE \"{name}\" OWNER \"{name}\";\n\
                 REVOKE ALL ON DATABASE \"{name}\" FROM PUBLIC;\n"
            );
            if let Some(n) = conn_limit {
                s += &format!("ALTER ROLE \"{name}\" CONNECTION LIMIT {n};\n");
            }
            if let Some(ms) = stmt_timeout_ms {
                s += &format!("ALTER ROLE \"{name}\" SET statement_timeout = '{ms}ms';\n");
            }
            s
        }
        Engine::MySql | Engine::MariaDb => {
            let mut s = format!(
                "CREATE DATABASE `{name}`;\n\
                 CREATE USER '{name}'@'%' IDENTIFIED BY '{pass}';\n\
                 GRANT ALL PRIVILEGES ON `{name}`.* TO '{name}'@'%';\n"
            );
            if let Some(n) = conn_limit {
                s += &format!("ALTER USER '{name}'@'%' WITH MAX_USER_CONNECTIONS {n};\n");
            }
            s
        }
        Engine::Mongo => format!(
            "db.getSiblingDB(\"{name}\").createUser({{user: \"{name}\", pwd: \"{pass}\", \
             roles: [{{role: \"readWrite\", db: \"{name}\"}}]}});\n"
        ),
    }
}

/// Script de remoção (**apaga os dados**).
pub fn drop_script(e: Engine, name: &str) -> String {
    match e {
        Engine::Postgres => format!(
            "DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE);\nDROP ROLE IF EXISTS \"{name}\";\n"
        ),
        Engine::MySql | Engine::MariaDb => {
            format!("DROP DATABASE IF EXISTS `{name}`;\nDROP USER IF EXISTS '{name}'@'%';\n")
        }
        Engine::Mongo => format!(
            "const d = db.getSiblingDB(\"{name}\"); d.dropUser(\"{name}\"); d.dropDatabase();\n"
        ),
    }
}

/// Espera o administrador do servidor responder. O deploy marca "Live" antes
/// de a imagem terminar de inicializar (MySQL/MariaDB reiniciam após o init, e
/// nesse meio-tempo a senha de root ainda não vale) — sem isto, criar um
/// database logo após subir o servidor falha com "Access denied".
pub async fn wait_ready(e: Engine, container: &str, tries: u32) -> Result<()> {
    let ping = match e {
        Engine::Mongo => "db.runCommand({ping: 1});\n",
        _ => "select 1;\n",
    };
    let mut last = anyhow!("servidor não respondeu");
    for _ in 0..tries {
        match exec(e, container, ping).await {
            Ok(_) => return Ok(()),
            Err(err) => last = err,
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
    Err(anyhow!("o servidor não ficou pronto a tempo: {last}"))
}

/// Roda `script` no cliente de administração dentro de `container`.
pub async fn exec(e: Engine, container: &str, script: &str) -> Result<String> {
    let mut child = Command::new("docker")
        .args(["exec", "-i", container, "sh", "-c", e.admin_sh()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| anyhow!("falha ao executar docker: {err}"))?;
    let mut stdin = child.stdin.take().ok_or_else(|| anyhow!("sem stdin"))?;
    let script = script.to_string();
    let writer = tokio::spawn(async move {
        let _ = stdin.write_all(script.as_bytes()).await;
        // `drop(stdin)` fecha o pipe e o cliente termina.
    });
    let out = child.wait_with_output().await?;
    let _ = writer.await;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("{}", err.trim());
    }
    // O mongosh devolve exit 0 mesmo com exceção em alguns casos.
    if e == Engine::Mongo && (stdout.contains("MongoServerError") || stdout.contains("Uncaught")) {
        bail!("{}", stdout.trim());
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_validos_e_invalidos() {
        assert!(validate_name("rdo_itemize").is_ok());
        assert!(validate_name("a1").is_ok());
        for bad in ["", "1abc", "Abc", "a-b", "a b", "a;drop", "postgres", &"x".repeat(33)] {
            assert!(validate_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn scripts_por_motor() {
        let pg = create_script(Engine::Postgres, "app", "pw", Some(20), Some(30000));
        assert!(pg.contains("CREATE ROLE \"app\" LOGIN PASSWORD 'pw'"));
        assert!(pg.contains("CONNECTION LIMIT 20") && pg.contains("statement_timeout = '30000ms'"));
        let my = create_script(Engine::MariaDb, "app", "pw", Some(5), Some(1));
        assert!(my.contains("'app'@'%'") && my.contains("MAX_USER_CONNECTIONS 5"));
        assert!(!my.contains("statement"));
        let mo = create_script(Engine::Mongo, "app", "pw", Some(5), None);
        assert!(mo.contains("readWrite") && !mo.contains("LIMIT"));
        assert!(drop_script(Engine::Postgres, "app").contains("WITH (FORCE)"));
    }

    #[test]
    fn senha_alfanumerica() {
        let p = generate_password().unwrap();
        assert_eq!(p.len(), 28);
        assert!(p.chars().all(|c| c.is_ascii_alphanumeric()));
    }
}
