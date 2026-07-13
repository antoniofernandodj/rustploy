//! Validação de `<name>`, `<reference>` (tag) e `<digest>` da OCI Distribution
//! Spec — implementada como scanner manual (sem a crate `regex`) porque a
//! gramática é regular e pequena. Ver `docs/plano-registry-embutido.md`.
//!
//! Gramática de `<name>` (distribution/reference/regexp.go):
//!   alphanumeric := [a-z0-9]+
//!   separator    := '.' | '_' | '__' | '-'+
//!   component    := alphanumeric (separator alphanumeric)*
//!   name         := component ('/' component)*        (máx. 255 chars)
//!
//! Por construção só aceita `[a-z0-9._/-]`, o que já rejeita path traversal
//! (`..`, `../`, `\`), maiúsculas, `:` e componentes vazios (`//`, `/` líder
//! ou final) — crítico porque `<name>` vira caminho de arquivo no storage CAS.

/// Valida `<name>` contra a gramática OCI distribution-spec.
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 255 {
        return false;
    }
    name.split('/').all(is_valid_component)
}

fn is_alnum(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit()
}

fn is_valid_component(c: &str) -> bool {
    let b = c.as_bytes();
    if b.is_empty() {
        return false;
    }
    if !is_alnum(b[0]) {
        return false;
    }
    let mut i = 1;
    while i < b.len() && is_alnum(b[i]) {
        i += 1;
    }
    while i < b.len() {
        // Consome exatamente um token separador: '.', '_', "__" ou '-'+.
        match b[i] {
            b'.' => i += 1,
            b'_' => {
                i += 1;
                if i < b.len() && b[i] == b'_' {
                    i += 1; // "__" é um único separador; "___" não existe na gramática
                }
            }
            b'-' => {
                while i < b.len() && b[i] == b'-' {
                    i += 1;
                }
            }
            _ => return false,
        }
        // Todo separador tem que ser seguido por pelo menos um alfanumérico
        // (senão o componente termina em separador, ou há dois seguidos).
        let run_start = i;
        while i < b.len() && is_alnum(b[i]) {
            i += 1;
        }
        if i == run_start {
            return false;
        }
    }
    true
}

/// Valida `<reference>` quando não é um digest — regex da spec:
/// `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}` (máx. 128 chars).
pub fn is_valid_tag(tag: &str) -> bool {
    let b = tag.as_bytes();
    if b.is_empty() || b.len() > 128 {
        return false;
    }
    let is_head = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let is_tail = |c: u8| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-');
    is_head(b[0]) && b[1..].iter().all(|&c| is_tail(c))
}

/// Extrai os 64 hex lowercase de `sha256:<hex>`. Único algoritmo suportado
/// nesta fase.
pub fn parse_digest(s: &str) -> Option<&str> {
    let hex = s.strip_prefix("sha256:")?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejeita_path_traversal_e_formatos_invalidos() {
        let bad = [
            "../etc/passwd",
            "..",
            "app/../etc",
            "app/..",
            "a/./b",
            "App",
            "a:b",
            "a\\b",
            "",
            "a//b",
            "/a",
            "a/",
            "-a",
            "a-",
            ".a",
            "a.",
            "a..b",
            "a___b",
            &"a".repeat(256),
        ];
        for name in bad {
            assert!(!is_valid_name(name), "aceitou indevidamente: {name:?}");
        }
    }

    #[test]
    fn aceita_nomes_validos() {
        let good = ["hello", "myorg/app", "a/b/c-1_2.3", "a__b", "a--b", "a.b.c"];
        for name in good {
            assert!(is_valid_name(name), "rejeitou indevidamente: {name:?}");
        }
    }

    #[test]
    fn valida_tags() {
        assert!(is_valid_tag("v1"));
        assert!(is_valid_tag("latest"));
        assert!(is_valid_tag("1.0.0-rc1"));
        assert!(!is_valid_tag(""));
        assert!(!is_valid_tag(".v1"));
        assert!(!is_valid_tag(&"a".repeat(129)));
    }

    #[test]
    fn valida_digest() {
        let hex = "a".repeat(64);
        let digest = format!("sha256:{hex}");
        assert_eq!(parse_digest(&digest), Some(hex.as_str()));
        assert_eq!(parse_digest("sha256:abc"), None);
        assert_eq!(parse_digest("md5:abc"), None);
        assert_eq!(parse_digest(&format!("sha256:{}", "Z".repeat(64))), None);
    }
}
