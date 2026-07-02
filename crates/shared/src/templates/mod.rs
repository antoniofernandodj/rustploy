//! Catálogo de templates de aplicações (formato Dokploy), lido dos blueprints
//! em `templates/blueprints/<id>/` e materializado como `&'static [Template]`
//! pelo `build.rs`.
//!
//! Cada template descreve:
//!   - `variables` — pares `key = "<gerador ou referência>"`. Geradores
//!     (`${domain}`, `${password:32}`, `${base64:64}`, `${jwt}`, `${email}`, …)
//!     produzem valores; referências (`${outra_var}`) apontam para outra
//!     variável já resolvida.
//!   - `env`       — o `.env` do compose (interpola `${var}` das variables).
//!   - `domains`   — roteamento (serviço + porta + host).
//!   - `mounts`    — arquivos de config a materializar (best-effort, ver abaixo).
//!   - `compose`   — o `docker-compose.yml` literal.
//!
//! `render` resolve as variáveis (gerando segredos aleatórios) e devolve o
//! compose + as env vars já substituídas prontas para virar um `ServiceSpec`.

use std::collections::BTreeMap;

// ── Tipos ─────────────────────────────────────────────────────────────────────

pub struct Template {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub logo: &'static str,
    pub default_port: u16,
    pub compose: &'static str,
    pub variables: &'static [Var],
    pub env: &'static [Kv],
    pub domains: &'static [Domain],
    pub mounts: &'static [Mount],
}

pub struct Var {
    pub key: &'static str,
    /// Valor bruto: gerador `${...}`, referência `${outra}` ou literal.
    pub raw: &'static str,
}

pub struct Kv {
    pub key: &'static str,
    pub raw: &'static str,
}

pub struct Domain {
    pub service_name: &'static str,
    pub port: u16,
    pub host: &'static str,
    pub path: &'static str,
}

pub struct Mount {
    pub file_path: &'static str,
    pub content: &'static str,
}

impl std::fmt::Debug for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Template({})", self.id)
    }
}

// Catálogo gerado em tempo de compilação (ver build.rs).
include!(concat!(env!("OUT_DIR"), "/templates_catalog.rs"));

// ── Registro ────────────────────────────────────────────────────────────────

pub fn all() -> &'static [Template] {
    TEMPLATES
}

pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.id == id)
}

/// Templates cujo nome/descrição/id batem com o termo de busca, ordenados por nome.
pub fn filtered(search: &str) -> Vec<&'static Template> {
    let search = search.to_lowercase();
    let mut out: Vec<_> = TEMPLATES
        .iter()
        .filter(|t| {
            search.is_empty()
                || t.name.to_lowercase().contains(&search)
                || t.description.to_lowercase().contains(&search)
                || t.id.contains(&search)
        })
        .collect();
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

/// Variáveis que o usuário edita no wizard: as que são um domínio (`${domain}`).
/// Todo o resto (senhas, chaves, e-mails) é gerado automaticamente.
pub fn editable_vars(t: &'static Template) -> Vec<&'static Var> {
    t.variables
        .iter()
        .filter(|v| matches!(parse_gen(v.raw.trim()), Some(Gen::Domain)))
        .collect()
}

// ── Resolução / render ────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct Rendered {
    pub compose: String,
    pub env: Vec<(String, String)>,
    pub domain: Option<String>,
    pub port: u16,
    /// Arquivos de config declarados por `[[config.mounts]]` (filePath/content),
    /// já com os `${var}` substituídos. O `ComposeSource` do rustploy ainda não
    /// carrega arquivos avulsos, então hoje isto é informativo.
    pub mounts: Vec<(String, String)>,
}

/// Resolve as variáveis (usando `user` para as editáveis) e devolve o compose +
/// env + domínio prontos. `user` são pares `(key, valor)` das variáveis de
/// domínio; vazio = tudo gerado.
pub fn render(t: &Template, user: &[(String, String)]) -> Rendered {
    let mut rng = Rng::new();
    let resolved = resolve_vars(t, user, &mut rng);

    let env = t
        .env
        .iter()
        .map(|kv| (kv.key.to_string(), substitute(kv.raw, &resolved, &mut rng)))
        .collect();

    let domain = t
        .domains
        .first()
        .map(|d| substitute(d.host, &resolved, &mut rng));

    let mounts = t
        .mounts
        .iter()
        .map(|m| {
            (
                m.file_path.to_string(),
                substitute(m.content, &resolved, &mut rng),
            )
        })
        .collect();

    Rendered {
        compose: t.compose.to_string(),
        env,
        domain,
        port: t.default_port,
        mounts,
    }
}

/// Resolve o mapa `variable -> valor`, semeando com os valores do usuário.
fn resolve_vars(t: &Template, user: &[(String, String)], rng: &mut Rng) -> BTreeMap<String, String> {
    let mut resolved: BTreeMap<String, String> = BTreeMap::new();
    for (k, v) in user {
        if !v.trim().is_empty() {
            resolved.insert(k.clone(), v.clone());
        }
    }

    // Várias passadas: referências entre variáveis exigem que o alvo já esteja
    // resolvido. Um número de passadas = nº de variáveis cobre qualquer cadeia.
    for _ in 0..=t.variables.len() {
        let mut progressed = false;
        for var in t.variables {
            if resolved.contains_key(var.key) {
                continue;
            }
            if let Some(val) = try_resolve(var.raw.trim(), &resolved, rng) {
                resolved.insert(var.key.to_string(), val);
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }

    // Sobrou alguma variável não resolvida (ref circular/ausente): resolve o que
    // der agora (geradores viram valor, refs pendentes viram vazio).
    for var in t.variables {
        resolved
            .entry(var.key.to_string())
            .or_insert_with(|| substitute(var.raw.trim(), &BTreeMap::new(), rng));
    }
    resolved
}

/// Tenta resolver um valor bruto; devolve `None` se depender de uma referência
/// ainda não resolvida (para tentar de novo numa próxima passada).
fn try_resolve(raw: &str, resolved: &BTreeMap<String, String>, rng: &mut Rng) -> Option<String> {
    // Token único `${...}`?
    if let Some(inner) = single_token(raw) {
        if let Some(g) = parse_gen(inner) {
            return Some(g.generate(rng));
        }
        // referência a outra variável
        return resolved.get(inner).cloned();
    }
    // Literal (possivelmente com refs embutidas): só resolve se todas as refs
    // já existem; geradores embutidos podem ser resolvidos na hora.
    if all_refs_available(raw, resolved) {
        return Some(substitute(raw, resolved, rng));
    }
    None
}

/// `${x}` (e só isso, sem texto ao redor) → `Some("x")`.
fn single_token(s: &str) -> Option<&str> {
    let inner = s.strip_prefix("${")?.strip_suffix('}')?;
    if inner.contains("${") || inner.contains('}') {
        return None;
    }
    Some(inner)
}

/// Toda referência `${k}` de `s` a uma variável (não-gerador) já está resolvida?
fn all_refs_available(s: &str, resolved: &BTreeMap<String, String>) -> bool {
    for inner in tokens(s) {
        if parse_gen(inner).is_none() && !resolved.contains_key(inner) {
            return false;
        }
    }
    true
}

/// Substitui todos os `${...}` de `s`: variável resolvida → valor; gerador →
/// valor gerado; desconhecido → string vazia.
fn substitute(s: &str, resolved: &BTreeMap<String, String>, rng: &mut Rng) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                let inner = &after[..end];
                if let Some(val) = resolved.get(inner) {
                    out.push_str(val);
                } else if let Some(g) = parse_gen(inner) {
                    out.push_str(&g.generate(rng));
                }
                // desconhecido: descarta o token
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Itera os miolos de todos os `${...}` em `s`.
fn tokens(s: &str) -> impl Iterator<Item = &str> {
    let mut rest = s;
    std::iter::from_fn(move || {
        let start = rest.find("${")?;
        let after = &rest[start + 2..];
        let end = after.find('}')?;
        let tok = &after[..end];
        rest = &after[end + 1..];
        Some(tok)
    })
}

// ── Geradores ─────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum Gen {
    Domain,
    Password(usize),
    Base64(usize),
    Jwt(usize),
    Hash(usize),
    Email,
    Username,
    Timestamp,
    Timezone,
}

/// Interpreta um miolo de token (`"password:32"`, `"domain"`, …) como gerador.
fn parse_gen(inner: &str) -> Option<Gen> {
    let (name, arg) = match inner.split_once(':') {
        Some((n, a)) => (n, a.parse::<usize>().ok()),
        None => (inner, None),
    };
    Some(match name {
        "domain" => Gen::Domain,
        "password" => Gen::Password(arg.unwrap_or(16)),
        "base64" => Gen::Base64(arg.unwrap_or(32)),
        "jwt" => Gen::Jwt(arg.unwrap_or(32)),
        "hash" => Gen::Hash(arg.unwrap_or(8)),
        "email" => Gen::Email,
        "username" => Gen::Username,
        "timestamp" => Gen::Timestamp,
        "timezone" => Gen::Timezone,
        _ => return None,
    })
}

impl Gen {
    fn generate(&self, rng: &mut Rng) -> String {
        match self {
            Gen::Domain => format!("app-{}.example.com", rng.hex(4)),
            Gen::Password(n) => rng.password(*n),
            Gen::Base64(n) => base64(&rng.bytes(*n)),
            Gen::Jwt(n) => rng.hex(*n),
            Gen::Hash(n) => rng.hex(*n),
            Gen::Email => format!("admin-{}@example.com", rng.hex(4)),
            Gen::Username => format!("admin{}", rng.hex(2)),
            Gen::Timestamp => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .to_string(),
            Gen::Timezone => "UTC".to_string(),
        }
    }
}

// ── PRNG (splitmix64, semeado no relógio) ─────────────────────────────────────

struct Rng {
    state: u64,
}

impl Rng {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
        Self {
            state: seed ^ 0x9e3779b97f4a7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next_u64() & 0xff) as u8).collect()
    }

    fn hex(&mut self, n: usize) -> String {
        self.bytes(n).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn password(&mut self, n: usize) -> String {
        const ALPHABET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        (0..n)
            .map(|_| ALPHABET[(self.next_u64() % ALPHABET.len() as u64) as usize] as char)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_nonempty_and_unique() {
        assert!(all().len() > 100, "catálogo deveria ter centenas de templates");
        let mut ids: Vec<_> = all().iter().map(|t| t.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "ids duplicados no catálogo");
    }

    #[test]
    fn wordpress_resolves_env_and_domain() {
        let t = find("wordpress").expect("wordpress no catálogo");
        let user = vec![("main_domain".to_string(), "wp.test.com".to_string())];
        let r = render(t, &user);

        let get = |k: &str| r.env.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone());
        // Literal preservado.
        assert_eq!(get("DB_NAME").as_deref(), Some("wordpress"));
        // Gerador de senha: 32 chars, sem `${}` remanescente.
        let pw = get("DB_PASSWORD").unwrap();
        assert_eq!(pw.len(), 32);
        assert!(!pw.contains("${"));
        // Domínio do usuário propagado.
        assert_eq!(r.domain.as_deref(), Some("wp.test.com"));
        assert!(!r.compose.is_empty());
    }

    #[test]
    fn literal_dollar_is_not_treated_as_token() {
        // `$argon2id$...` tem `$` mas não `${...}` — deve passar intacto.
        let mut rng = Rng::new();
        let map = BTreeMap::new();
        let s = "$argon2id$v=19$m=65536$abc";
        assert_eq!(substitute(s, &map, &mut rng), s);
    }

    #[test]
    fn editable_vars_are_only_domains() {
        let t = find("wordpress").expect("wordpress");
        let ed = editable_vars(t);
        assert!(ed.iter().all(|v| v.key.contains("domain")));
    }
}

/// Base64 padrão (com padding), sem dependência externa.
fn base64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}
