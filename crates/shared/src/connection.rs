//! Connection string de banco/broker, montada num lugar só.
//!
//! Antes havia uma cópia em Luau (GUI) e outra em JS (webui), e nenhuma servia
//! como `DATABASE_URL` (Postgres saía como JDBC, o host era um palpite). Ver
//! `docs/plano-banco-compartilhado.md` §3.

/// Percent-encoding de um componente de userinfo (`@`, `:`, `/`, `?`, `#`…).
pub fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Esquema de URI por `db_kind`; `None` = sem esquema (Kafka, serviço comum).
/// MariaDB usa `mysql://` (a maioria dos drivers não conhece `mariadb://`).
pub fn scheme(db_kind: Option<&str>) -> Option<&'static str> {
    match db_kind.map(str::to_ascii_lowercase).as_deref() {
        Some("postgres" | "postgresql") => Some("postgresql"),
        Some("mysql" | "mariadb") => Some("mysql"),
        Some("redis") => Some("redis"),
        Some("mongodb" | "mongo") => Some("mongodb"),
        Some("rabbitmq") => Some("amqp"),
        Some("nats") => Some("nats"),
        Some("kafka") => None,
        _ => Some("http"),
    }
}

/// Destino de uma conexão.
#[derive(Debug, Default, Clone)]
pub struct ConnTarget<'a> {
    pub host: &'a str,
    pub port: u16,
    pub database: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    /// Só MongoDB: database onde o usuário foi criado (`authSource`).
    pub auth_source: Option<&'a str>,
}

fn non_empty(s: Option<&str>) -> Option<&str> {
    s.filter(|v| !v.is_empty())
}

/// URI padrão (`scheme://user:senha@host:porta/database`) que os drivers
/// aceitam como `DATABASE_URL`. Sem esquema conhecido, só `host:porta`.
pub fn connection_url(db_kind: Option<&str>, t: &ConnTarget<'_>) -> String {
    let hp = format!("{}:{}", t.host, t.port);
    let Some(scheme) = scheme(db_kind) else {
        return hp;
    };
    let (user, pass) = (non_empty(t.user), non_empty(t.password));
    let userinfo = match (user, pass) {
        (None, None) => String::new(),
        (Some(u), None) => format!("{}@", pct_encode(u)),
        (u, Some(p)) => format!("{}:{}@", pct_encode(u.unwrap_or("")), pct_encode(p)),
    };
    let mut url = format!("{scheme}://{userinfo}{hp}");
    let mongo = scheme == "mongodb";
    match non_empty(t.database) {
        Some(d) => {
            url.push('/');
            url.push_str(d);
        }
        None if mongo => url.push('/'),
        None => {}
    }
    if mongo && user.is_some() {
        if let Some(src) = non_empty(t.auth_source).or(non_empty(t.database)) {
            url.push_str("?authSource=");
            url.push_str(src);
        } else {
            url.push_str("?authSource=admin");
        }
    }
    url
}

/// `(database, usuário, senha, authSource)` de um banco/broker, lidos das env
/// vars nas convenções que o wizard grava (`wizard::db_env_vars`). `get`
/// devolve o valor já resolvido (projeto + serviço, secrets decifradas).
pub fn credentials(
    db_kind: Option<&str>,
    get: impl Fn(&str) -> Option<String>,
) -> (Option<String>, Option<String>, Option<String>, Option<&'static str>) {
    let g = |k: &str| get(k).filter(|v| !v.is_empty());
    match db_kind.map(str::to_ascii_lowercase).as_deref() {
        Some("postgres" | "postgresql") => {
            (g("POSTGRES_DB"), g("POSTGRES_USER"), g("POSTGRES_PASSWORD"), None)
        }
        Some("mysql" | "mariadb") => {
            (g("MYSQL_DATABASE"), g("MYSQL_USER"), g("MYSQL_PASSWORD"), None)
        }
        Some("mongodb" | "mongo") => (
            None,
            g("MONGO_INITDB_ROOT_USERNAME"),
            g("MONGO_INITDB_ROOT_PASSWORD"),
            Some("admin"),
        ),
        Some("redis") => (None, None, g("REDIS_PASSWORD"), None),
        Some("rabbitmq") => (None, g("RABBITMQ_DEFAULT_USER"), g("RABBITMQ_DEFAULT_PASS"), None),
        _ => (None, None, None, None),
    }
}

/// Serviços declarados num YAML de Compose: `(chave, imagem)`, na ordem.
/// Leitura por linhas (o `shared` não depende de parser YAML): as chaves são as
/// de menor recuo sob `services:`.
pub fn compose_services(content: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = vec![];
    let mut in_services = false;
    let mut key_indent: Option<usize> = None;
    for line in content.lines() {
        let trimmed = line.trim_start();
        let blank = trimmed.is_empty() || trimmed.starts_with('#');
        let indent = line.len() - trimmed.len();
        if !in_services {
            if line.trim_end() == "services:" {
                in_services = true;
            }
            continue;
        }
        if blank {
            continue;
        }
        if indent == 0 {
            break; // outro bloco de topo (`volumes:`, `networks:`…)
        }
        let ki = *key_indent.get_or_insert(indent);
        if indent == ki {
            if let Some((k, _)) = trimmed.split_once(':') {
                let k = k.trim().trim_matches(|c| c == '"' || c == '\'');
                if !k.is_empty() {
                    out.push((k.to_string(), None));
                }
            }
        } else if indent > ki
            && let Some(img) = trimmed.strip_prefix("image:")
            && let Some(last) = out.last_mut()
        {
            let img = img.trim().trim_matches(|c| c == '"' || c == '\'');
            last.1 = Some(img.to_string());
        }
    }
    out
}

/// Host (chave do serviço no YAML) que recebe a conexão numa stack Compose:
/// `ingress_service` se preenchido; senão o único serviço; senão o que tem
/// imagem do `db_kind`; senão `None` (quem chama cai no nome do container, que
/// sempre resolve — em vez de inventar um).
pub fn compose_host(
    content: &str,
    ingress_service: Option<&str>,
    db_kind: Option<&str>,
) -> Option<String> {
    if let Some(i) = ingress_service.filter(|s| !s.is_empty()) {
        return Some(i.to_string());
    }
    let services = compose_services(content);
    if let [(k, _)] = services.as_slice() {
        return Some(k.clone());
    }
    let kind = db_kind?.to_ascii_lowercase();
    let needle = match kind.as_str() {
        "postgresql" => "postgres",
        "mongo" => "mongo",
        k => k,
    };
    let mut matches = services
        .iter()
        .filter(|(_, img)| img.as_deref().is_some_and(|i| i.contains(needle)));
    match (matches.next(), matches.next()) {
        (Some((k, _)), None) => Some(k.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t<'a>(h: &'a str, p: u16) -> ConnTarget<'a> {
        ConnTarget { host: h, port: p, ..Default::default() }
    }

    #[test]
    fn postgres_uri_padrao_com_senha_escapada() {
        let u = connection_url(
            Some("postgres"),
            &ConnTarget {
                database: Some("app"),
                user: Some("app"),
                password: Some("p@ss:w/rd"),
                ..t("rp-shared-x-postgres", 5432)
            },
        );
        assert_eq!(u, "postgresql://app:p%40ss%3Aw%2Frd@rp-shared-x-postgres:5432/app");
    }

    #[test]
    fn mariadb_usa_esquema_mysql() {
        let u = connection_url(Some("mariadb"), &ConnTarget { user: Some("u"), password: Some("p"), database: Some("d"), ..t("h", 3306) });
        assert_eq!(u, "mysql://u:p@h:3306/d");
    }

    #[test]
    fn mongo_auth_source_e_o_database() {
        let u = connection_url(Some("mongodb"), &ConnTarget { user: Some("u"), password: Some("p"), database: Some("proj"), ..t("h", 27017) });
        assert_eq!(u, "mongodb://u:p@h:27017/proj?authSource=proj");
        let root = connection_url(Some("mongodb"), &ConnTarget { user: Some("root"), password: Some("p"), auth_source: Some("admin"), ..t("h", 27017) });
        assert_eq!(root, "mongodb://root:p@h:27017/?authSource=admin");
    }

    #[test]
    fn sem_credenciais_e_sem_esquema() {
        assert_eq!(connection_url(Some("postgres"), &t("h", 5432)), "postgresql://h:5432");
        assert_eq!(connection_url(Some("redis"), &ConnTarget { password: Some("s"), ..t("h", 6379) }), "redis://:s@h:6379");
        assert_eq!(connection_url(Some("kafka"), &t("h", 9092)), "h:9092");
        assert_eq!(connection_url(None, &t("h", 80)), "http://h:80");
    }

    const YAML: &str = "# c\nservices:\n\n  # nota\n  db:\n    image: postgres:18\n    environment:\n      A: 1\n  pooler:\n    image: edoburu/pgbouncer\nvolumes:\n  d:\n";

    #[test]
    fn compose_host_regras() {
        assert_eq!(compose_host(YAML, Some("pooler"), Some("postgres")).as_deref(), Some("pooler"));
        assert_eq!(compose_host(YAML, None, Some("postgres")).as_deref(), Some("db"));
        assert_eq!(compose_host(YAML, None, None), None);
        assert_eq!(compose_host("services:\n  rp_db:\n    image: x\n", None, None).as_deref(), Some("rp_db"));
        assert_eq!(compose_host("image: x", None, None), None);
    }
}
