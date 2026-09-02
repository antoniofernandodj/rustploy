//! Catálogo de templates de aplicações (formato Dokploy), lido dos blueprints
//! em `templates/blueprints/<id>/` e materializado como `&'static [Template]`
//! pelo `build.rs`.
//!
//! Cada template descreve:
//!   - `variables` — pares `key = "<gerador ou referência>"`. Geradores
//!     (`${domain}`, `${password:32}`, `${base64:64}`, `${jwt}`, `${uuid}`,
//!     `${timestamps:<RFC3339>}`, `${email}`, …) produzem valores; referências
//!     (`${outra_var}`) apontam para outra variável já resolvida. O caso
//!     especial `${jwt:<var_segredo>:<var_payload>}` assina um JWT HS256 com
//!     duas outras variáveis (é assim que o template do Supabase produz
//!     `ANON_KEY`/`SERVICE_ROLE_KEY`).
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
    /// já com os `${var}` do template substituídos — tokens que não são do
    /// template ficam intactos, para o container interpolar. O `ComposeSource`
    /// do rustploy ainda não carrega arquivos avulsos, então hoje isto é
    /// informativo.
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
                // Mounts são arquivos de config lidos pelos próprios containers,
                // que fazem a interpolação deles (`vector.yml` usa
                // `${LOGFLARE_PUBLIC_ACCESS_TOKEN?…}`, por exemplo) — um token
                // que não é variável nem gerador do template tem de sobreviver
                // até lá, não pode ser apagado como no env.
                substitute_mode(m.content, &resolved, &mut rng, Unknown::Keep),
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
fn resolve_vars(
    t: &Template,
    user: &[(String, String)],
    rng: &mut Rng,
) -> BTreeMap<String, String> {
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
            if !g.refs_available(resolved) {
                return None;
            }
            return Some(g.generate(rng, resolved));
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
/// Geradores que apontam para outras variáveis (`${jwt:<segredo>:<payload>}`)
/// também contam — sem os alvos resolvidos não há o que assinar.
fn all_refs_available(s: &str, resolved: &BTreeMap<String, String>) -> bool {
    for inner in tokens(s) {
        match parse_gen(inner) {
            Some(g) => {
                if !g.refs_available(resolved) {
                    return false;
                }
            }
            None => {
                if !resolved.contains_key(inner) {
                    return false;
                }
            }
        }
    }
    true
}

/// O que fazer com um `${...}` que não é variável resolvida nem gerador.
#[derive(Clone, Copy, PartialEq)]
enum Unknown {
    /// Descarta o token (env/domínio: o valor é literal, não há quem interpole depois).
    Drop,
    /// Mantém o token intacto (mounts: quem interpola é o container).
    Keep,
}

/// Substitui todos os `${...}` de `s`: variável resolvida → valor; gerador →
/// valor gerado; desconhecido → string vazia (ver [`substitute_mode`] para
/// manter o token).
fn substitute(s: &str, resolved: &BTreeMap<String, String>, rng: &mut Rng) -> String {
    substitute_mode(s, resolved, rng, Unknown::Drop)
}

fn substitute_mode(
    s: &str,
    resolved: &BTreeMap<String, String>,
    rng: &mut Rng,
    unknown: Unknown,
) -> String {
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
                    out.push_str(&g.generate(rng, resolved));
                } else if unknown == Unknown::Keep {
                    out.push_str("${");
                    out.push_str(inner);
                    out.push('}');
                }
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
enum Gen<'a> {
    Domain,
    Password(usize),
    Base64(usize),
    /// `${jwt}` / `${jwt:<n>}` — só um hex aleatório de `n` bytes (um *segredo*
    /// de JWT, não um JWT). Ver [`Gen::JwtSigned`] para o token de fato.
    Jwt(usize),
    Hash(usize),
    Email,
    Username,
    Uuid,
    /// `${timestamp}` — epoch em segundos de agora.
    Timestamp,
    /// `${timestamps:<RFC3339>}` — epoch em segundos do instante dado
    /// (ex.: `${timestamps:2030-01-01T00:00:00Z}`, usado no `exp` dos JWTs).
    Timestamps(&'a str),
    Timezone,
    /// `${jwt:<var_segredo>:<var_payload>}` — JWT HS256 de verdade: assina o
    /// JSON de `var_payload` com o segredo de `var_segredo` (as duas são
    /// variáveis do template, resolvidas antes).
    JwtSigned {
        secret: &'a str,
        payload: &'a str,
    },
}

/// Interpreta um miolo de token (`"password:32"`, `"domain"`, …) como gerador.
fn parse_gen(inner: &str) -> Option<Gen<'_>> {
    let (name, arg) = match inner.split_once(':') {
        Some((n, a)) => (n, Some(a)),
        None => (inner, None),
    };
    let num = |default: usize| arg.and_then(|a| a.parse::<usize>().ok()).unwrap_or(default);
    Some(match name {
        "domain" => Gen::Domain,
        "password" => Gen::Password(num(16)),
        "base64" => Gen::Base64(num(32)),
        // `${jwt:<segredo>:<payload>}` (dois nomes de variável) assina um token;
        // qualquer outra forma continua sendo o hex aleatório de sempre.
        "jwt" => match arg.and_then(|a| a.split_once(':')) {
            Some((secret, payload)) => Gen::JwtSigned { secret, payload },
            None => Gen::Jwt(num(32)),
        },
        "hash" => Gen::Hash(num(8)),
        "email" => Gen::Email,
        "username" => Gen::Username,
        "uuid" => Gen::Uuid,
        "timestamp" => Gen::Timestamp,
        "timestamps" => Gen::Timestamps(arg?),
        "timezone" => Gen::Timezone,
        _ => return None,
    })
}

impl Gen<'_> {
    /// As variáveis referenciadas pelo gerador já estão resolvidas? Só
    /// `${jwt:<segredo>:<payload>}` referencia outras variáveis; o resto gera
    /// sozinho.
    fn refs_available(&self, resolved: &BTreeMap<String, String>) -> bool {
        match self {
            Gen::JwtSigned { secret, payload } => {
                resolved.contains_key(*secret) && resolved.contains_key(*payload)
            }
            _ => true,
        }
    }

    fn generate(&self, rng: &mut Rng, resolved: &BTreeMap<String, String>) -> String {
        match self {
            Gen::Domain => format!("app-{}.example.com", rng.hex(4)),
            Gen::Password(n) => rng.password(*n),
            Gen::Base64(n) => base64(&rng.bytes(*n)),
            Gen::Jwt(n) => rng.hex(*n),
            Gen::Hash(n) => rng.hex(*n),
            Gen::Email => format!("admin-{}@example.com", rng.hex(4)),
            Gen::Username => format!("admin{}", rng.hex(2)),
            Gen::Uuid => rng.uuid_v4(),
            Gen::Timestamp => std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .to_string(),
            // Data inválida → token vira vazio (mesma regra dos desconhecidos).
            Gen::Timestamps(iso) => chrono::DateTime::parse_from_rfc3339(iso)
                .map(|d| d.timestamp().to_string())
                .unwrap_or_default(),
            Gen::Timezone => "UTC".to_string(),
            Gen::JwtSigned { secret, payload } => {
                let get = |k: &str| resolved.get(k).map(String::as_str).unwrap_or_default();
                jwt_hs256(get(secret), get(payload))
            }
        }
    }
}

/// JWS compacto `header.payload.assinatura` em HS256. O payload entra como o
/// JSON do template (inclusive com quebras de linha — é o que o próprio
/// Supabase publica nas chaves de exemplo).
fn jwt_hs256(secret: &str, payload_json: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const HEADER: &str = r#"{"alg":"HS256","typ":"JWT"}"#;
    let signing_input = format!(
        "{}.{}",
        base64_url(HEADER.as_bytes()),
        base64_url(payload_json.trim().as_bytes())
    );

    let mut mac = <Hmac<Sha256>>::new_from_slice(secret.as_bytes())
        .expect("HMAC aceita chave de qualquer tamanho");
    mac.update(signing_input.as_bytes());
    format!(
        "{signing_input}.{}",
        base64_url(&mac.finalize().into_bytes())
    )
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

    /// UUID v4 (bits de versão/variante fixados) — `POOLER_TENANT_ID` e afins.
    fn uuid_v4(&mut self) -> String {
        let mut b = self.bytes(16);
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        let hex = |s: &[u8]| s.iter().map(|x| format!("{x:02x}")).collect::<String>();
        format!(
            "{}-{}-{}-{}-{}",
            hex(&b[0..4]),
            hex(&b[4..6]),
            hex(&b[6..8]),
            hex(&b[8..10]),
            hex(&b[10..16])
        )
    }

    fn password(&mut self, n: usize) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
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
        assert!(
            all().len() > 100,
            "catálogo deveria ter centenas de templates"
        );
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
    fn supabase_renders_signed_keys_and_pooler_uuid() {
        let t = find("supabase").expect("supabase no catálogo");
        let user = vec![("main_domain".to_string(), "sb.test.com".to_string())];
        let r = render(t, &user);
        let get = |k: &str| {
            r.env
                .iter()
                .find(|(kk, _)| kk == k)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| panic!("{k} ausente no env renderizado"))
        };

        // ANON_KEY/SERVICE_ROLE_KEY são JWTs HS256 assinados com JWT_SECRET.
        let secret = get("JWT_SECRET");
        for (key, role) in [("ANON_KEY", "anon"), ("SERVICE_ROLE_KEY", "service_role")] {
            let token = get(key);
            let parts: Vec<&str> = token.split('.').collect();
            assert_eq!(parts.len(), 3, "{key} não é um JWS compacto: {token}");
            // Assinatura confere?
            let expected = jwt_hs256(
                &secret,
                &String::from_utf8(b64url_decode(parts[1])).unwrap(),
            );
            assert_eq!(token, expected, "assinatura de {key} não confere");
            // Payload traz o role certo e um `exp` numérico (do `${timestamps:…}`).
            let payload = String::from_utf8(b64url_decode(parts[1])).unwrap();
            assert!(payload.contains(role), "{key} sem role {role}: {payload}");
            let exp: i64 = payload
                .split("\"exp\":")
                .nth(1)
                .and_then(|s| s.trim().trim_end_matches(['}', '\n', ' ']).parse().ok())
                .unwrap_or_else(|| panic!("exp não numérico em {key}: {payload}"));
            assert_eq!(exp, 1_893_456_000, "exp deveria ser 2030-01-01T00:00:00Z");
        }

        // `${uuid}` do POOLER_TENANT_ID: v4 formatado.
        let tenant = get("POOLER_TENANT_ID");
        assert_eq!(tenant.len(), 36, "tenant id não é um uuid: {tenant}");
        assert_eq!(tenant.chars().filter(|c| *c == '-').count(), 4);
        assert_eq!(
            &tenant[14..15],
            "4",
            "versão do uuid deveria ser 4: {tenant}"
        );

        // O realtime exige exatamente 16 caracteres nesta chave.
        assert_eq!(get("REALTIME_DB_ENC_KEY").len(), 16);

        // Domínio propagado para as URLs públicas e para o ingress.
        assert_eq!(get("SUPABASE_PUBLIC_URL"), "https://sb.test.com");
        assert_eq!(get("API_EXTERNAL_URL"), "https://sb.test.com/auth/v1");
        assert_eq!(r.domain.as_deref(), Some("sb.test.com"));

        // Mounts: o entrypoint do kong precisa chegar com o `\$(` das expressões
        // Lua intacto (escaping do TOML) e o kong.yml com as rotas novas.
        let mount = |p: &str| {
            r.mounts
                .iter()
                .find(|(path, _)| path == p)
                .map(|(_, c)| c.clone())
                .unwrap_or_else(|| panic!("mount {p} ausente"))
        };
        assert!(mount("/volumes/api/kong-entrypoint.sh").contains(r"\$((headers.authorization"));
        assert!(mount("/volumes/api/kong.yml").contains("SUPABASE_PUBLISHABLE_KEY"));
    }

    #[test]
    fn unknown_tokens_survive_in_mounts_but_not_in_env() {
        let mut rng = Rng::new();
        let map = BTreeMap::new();
        // Env/domínio: token desconhecido some (o valor é literal).
        assert_eq!(substitute("x=${nope}!", &map, &mut rng), "x=!");
        // Mount: quem interpola é o container, então o token fica.
        assert_eq!(
            substitute_mode("x=${nope}!", &map, &mut rng, Unknown::Keep),
            "x=${nope}!"
        );

        // No supabase isso é o `x-api-key` que o vector resolve sozinho.
        let t = find("supabase").expect("supabase no catálogo");
        let r = render(t, &[]);
        let (_, vector) = r
            .mounts
            .iter()
            .find(|(p, _)| p == "/volumes/logs/vector.yml")
            .expect("vector.yml nos mounts");
        assert!(
            vector.contains("${LOGFLARE_PUBLIC_ACCESS_TOKEN?"),
            "token do vector perdido"
        );
    }

    /// Decodifica base64url sem padding (só para os testes).
    fn b64url_decode(s: &str) -> Vec<u8> {
        const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut bits = 0u32;
        let mut nbits = 0u32;
        let mut out = Vec::new();
        for c in s.bytes() {
            let v = T.iter().position(|t| *t == c).expect("char base64url") as u32;
            bits = (bits << 6) | v;
            nbits += 6;
            if nbits >= 8 {
                nbits -= 8;
                out.push((bits >> nbits) as u8);
            }
        }
        out
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
    base64_with(data, T, true)
}

/// Base64url **sem** padding (RFC 4648 §5) — o alfabeto que o JWS exige.
fn base64_url(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    base64_with(data, T, false)
}

fn base64_with(data: &[u8], table: &[u8], pad: bool) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(table[(n >> 18 & 63) as usize] as char);
        out.push(table[(n >> 12 & 63) as usize] as char);
        for (i, shift) in [(1usize, 6u32), (2, 0)] {
            if chunk.len() > i {
                out.push(table[(n >> shift & 63) as usize] as char);
            } else if pad {
                out.push('=');
            }
        }
    }
    out
}
