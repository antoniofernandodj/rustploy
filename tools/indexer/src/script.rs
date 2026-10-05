//! Índice dos arquivos que não são Rust: scripts Luau e JS, templates `.gv`/`.gvb`,
//! `index.html` da webui e folhas de estilo.
//!
//! Aqui não há parser de verdade, só leitura linha a linha com regex. Esses
//! arquivos são pequenos e seguem padrões regulares (`function x(`,
//! `local function x(`, `Alpine.data("x", …)`, `on_click="handler:…"`), e o
//! que interessa deles é pouco: o comentário de cabeçalho, as funções com o
//! comentário logo acima e, nos templates, quem chama quem.

use crate::summarize;
use regex::Regex;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::LazyLock;

pub const EXTENSIONS: &[&str] = &["luau", "js", "gv", "gvb", "html", "gss", "css"];

/// Uma linha de notação por tipo de arquivo, para o cabeçalho da área.
pub fn notation(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "luau" => {
            "> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), \
             `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.\n"
        }
        "gv" => {
            "> `.gv`: `<screen>`/`<component>`, `props`, `imports` (componentes), `script`, \
             `views` (valores de `if=\"{view}\" equals=`) e `handlers` (chamados em `on_*=`).\n"
        }
        "gvb" => {
            "> `.gvb`: `<app>`/`<screen>`/`<component>`, `props`, `imports` (`link(rel = import)`), \
             `script`, `telas` (as `screen(name = …)` do app), `views` (valores de `@view ==`), \
             `abas` (valores de `@tab`/`@*_tab`) e `handlers` (`on_*` e `action`).\n"
        }
        "js" => {
            "> JS: `Alpine.store/data(\"x\")` abre um bloco com seus métodos indentados; \
             `get:` lista os getters; `function x(a)` = função de módulo.\n"
        }
        "html" => {
            "> HTML: `seções` = comentários `── X ──`; `x-data` = componentes Alpine; \
             `chama` = métodos usados em `@click`/`@submit`/….\n"
        }
        _ => return None,
    })
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("regex do indexer")
}

// ── Descrição do arquivo (comentário de cabeçalho) ───────────────────────────

/// Primeira frase do comentário que abre o arquivo, sem o prefixo
/// `nome.ext — ` que vários arquivos repetem.
pub fn describe(path: &str, text: &str) -> Option<String> {
    let ext = path.rsplit('.').next().unwrap_or("");
    let lines = match ext {
        "luau" => line_comment_header(text, "--", &["--!"]),
        "js" => {
            let l = line_comment_header(text, "//", &[]);
            if l.is_empty() {
                block_comment_header(text, "/*", "*/")
            } else {
                l
            }
        }
        "gvb" => line_comment_header(text, "//", &[]),
        "gss" | "css" => block_comment_header(text, "/*", "*/"),
        "gv" | "html" => block_comment_header(text, "<!--", "-->"),
        _ => return None,
    };
    let name = path.rsplit('/').next().unwrap_or(path);
    let mut lines = lines;
    if let Some(first) = lines.first_mut()
        && first.contains(name)
        && let Some((_, rest)) = first.split_once(" — ")
    {
        *first = rest.to_owned();
    }
    summarize(
        lines
            .iter()
            .map(|l| l.trim_matches(|c: char| c == '─' || c.is_whitespace())),
    )
}

/// Linhas do primeiro bloco de comentários de linha (`--`, `//`) no topo.
fn line_comment_header(text: &str, marker: &str, skip: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in text.lines().map(str::trim) {
        // Diretivas (`--!strict`) e linhas em branco antes do comentário.
        if skip.iter().any(|s| line.starts_with(s)) || (line.is_empty() && out.is_empty()) {
            continue;
        }
        match line.strip_prefix(marker) {
            Some(rest) => out.push(rest.trim().to_owned()),
            None => break,
        }
    }
    out
}

/// Conteúdo do primeiro comentário de bloco (`/* */`, `<!-- -->`) do arquivo,
/// desde que só haja preâmbulo (`<!doctype>`, `<html>`, `<head>`, `<meta>`) antes dele.
fn block_comment_header(text: &str, open: &str, close: &str) -> Vec<String> {
    let Some(start) = text.find(open) else {
        return Vec::new();
    };
    let before = &text[..start];
    let preamble_ok = before.lines().map(str::trim).all(|l| {
        l.is_empty()
            || ["<!doctype", "<html", "<head", "<meta", "<title"]
                .iter()
                .any(|p| l.to_lowercase().starts_with(p))
    });
    if !preamble_ok {
        return Vec::new();
    }
    let body = &text[start + open.len()..];
    let body = &body[..body.find(close).unwrap_or(body.len())];
    body.lines()
        .map(|l| l.trim().trim_start_matches('*').trim().to_owned())
        .collect()
}

// ── Símbolos ─────────────────────────────────────────────────────────────────

pub fn render(path: &str, text: &str) -> String {
    match path.rsplit('.').next().unwrap_or("") {
        "luau" => render_luau(text),
        "js" => render_js(text),
        "gv" => render_gv(text),
        "gvb" => render_gvb(text),
        "html" => render_html(text),
        _ => String::new(),
    }
}

/// Junta os comentários consecutivos logo acima de uma definição.
#[derive(Default)]
struct PendingDoc(Vec<String>);

impl PendingDoc {
    fn feed(&mut self, line: &str, markers: &[&str]) -> bool {
        let t = line.trim();
        for m in markers {
            if let Some(rest) = t.strip_prefix(m) {
                self.0.push(rest.trim_end_matches("*/").trim().to_owned());
                return true;
            }
        }
        false
    }

    fn take(&mut self) -> Option<String> {
        let lines = std::mem::take(&mut self.0);
        summarize(lines.iter().map(String::as_str))
    }
}

fn with_doc(out: &mut String, body: String, doc: Option<String>) {
    match doc {
        Some(d) => {
            let _ = writeln!(out, "{body} — {d}");
        }
        None => {
            let _ = writeln!(out, "{body}");
        }
    }
}

/// `a: string, b: number?` → `a, b`; `kind = "info"` → `kind`.
fn param_names(params: &str) -> String {
    params
        .split(',')
        .map(|p| p.split([':', '=']).next().unwrap_or("").trim())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

static LUAU_FN: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(local\s+)?function\s+([\w.:]+)\s*\(([^)]*)\)"));
static LUAU_ASSIGN_FN: LazyLock<Regex> =
    LazyLock::new(|| re(r"^([\w.]+)\s*=\s*function\s*\(([^)]*)\)"));
static LUAU_TYPE: LazyLock<Regex> = LazyLock::new(|| re(r"^export\s+type\s+(\w+)"));

fn render_luau(text: &str) -> String {
    let mut out = String::new();
    let mut doc = PendingDoc::default();
    let mut types = Vec::new();
    for line in text.lines() {
        if line.starts_with("--!") {
            continue;
        }
        // Só nível 0: comentários e funções indentados são detalhe de
        // implementação de outra função.
        if line.starts_with("--") {
            doc.feed(line, &["--"]);
            continue;
        }
        if let Some(c) = LUAU_FN.captures(line) {
            let kw = if c.get(1).is_some() {
                "local"
            } else {
                "function"
            };
            with_doc(
                &mut out,
                format!("{kw} {}({})", &c[2], param_names(&c[3])),
                doc.take(),
            );
        } else if let Some(c) = LUAU_ASSIGN_FN.captures(line) {
            with_doc(
                &mut out,
                format!("function {}({})", &c[1], param_names(&c[2])),
                doc.take(),
            );
        } else if let Some(c) = LUAU_TYPE.captures(line) {
            types.push(c[1].to_owned());
            doc.take();
        } else {
            doc.take();
        }
    }
    if !types.is_empty() {
        let _ = writeln!(out, "types: {}", types.join(", "));
    }
    out
}

static JS_CONTAINER: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"^(\s*)(?:Alpine\.(store|data)\(\s*"(\w+)"|(?:export\s+)?class\s+(\w+))"#)
});
static JS_TOP_FN: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:export\s+)?(?:async\s+)?function\s*\*?\s*(\w+)\s*\(([^)]*)\)"));
static JS_TOP_ARROW: LazyLock<Regex> = LazyLock::new(|| {
    re(r"^(?:export\s+)?const\s+(\w+)\s*=\s*(?:async\s*)?(?:\(([^)]*)\)|(\w+))\s*=>")
});
static JS_METHOD: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(\s+)(?:async\s+)?(\w+)\s*\(([^)]*)\)\s*\{\s*$"));
static JS_GETTER: LazyLock<Regex> = LazyLock::new(|| re(r"^(\s+)get\s+(\w+)\s*\(\)"));

const JS_KEYWORDS: &[&str] = &[
    "if", "for", "while", "switch", "catch", "function", "return", "with",
];

fn render_js(text: &str) -> String {
    let mut out = String::new();
    let mut doc = PendingDoc::default();
    // Indentação esperada dos métodos do bloco Alpine/classe aberto.
    let mut member_indent: Option<usize> = None;
    let mut getters: Vec<String> = Vec::new();

    fn flush(out: &mut String, getters: &mut Vec<String>, indent: Option<usize>) {
        if !getters.is_empty() {
            let pad = " ".repeat(indent.unwrap_or(0));
            let _ = writeln!(out, "{pad}get: {}", getters.join(", "));
            getters.clear();
        }
    }

    for line in text.lines() {
        if doc.feed(line, &["//", "/**", "*/", "* ", "*"]) {
            continue;
        }
        if let Some(c) = JS_CONTAINER.captures(line) {
            flush(&mut out, &mut getters, member_indent);
            let indent = c[1].len();
            let head = match (c.get(2), c.get(3), c.get(4)) {
                (Some(kind), Some(name), _) => {
                    format!("Alpine.{}(\"{}\")", kind.as_str(), name.as_str())
                }
                (_, _, Some(class)) => format!("class {}", class.as_str()),
                _ => unreachable!(),
            };
            with_doc(
                &mut out,
                format!("{}{head}", " ".repeat(indent)),
                doc.take(),
            );
            member_indent = Some(indent + 2);
        } else if let Some(c) = JS_TOP_FN.captures(line) {
            flush(&mut out, &mut getters, member_indent);
            member_indent = None;
            with_doc(
                &mut out,
                format!("function {}({})", &c[1], param_names(&c[2])),
                doc.take(),
            );
        } else if let Some(c) = JS_TOP_ARROW.captures(line) {
            flush(&mut out, &mut getters, member_indent);
            member_indent = None;
            let params = c
                .get(2)
                .or(c.get(3))
                .map(|m| param_names(m.as_str()))
                .unwrap_or_default();
            with_doc(&mut out, format!("const {}({params})", &c[1]), doc.take());
        } else if let Some(c) = JS_GETTER.captures(line)
            && Some(c[1].len()) == member_indent
        {
            getters.push(c[2].to_owned());
            doc.take();
        } else if let Some(c) = JS_METHOD.captures(line)
            && Some(c[1].len()) == member_indent
            && !JS_KEYWORDS.contains(&&c[2])
        {
            with_doc(
                &mut out,
                format!("{}{}({})", &c[1], &c[2], param_names(&c[3])),
                doc.take(),
            );
        } else {
            doc.take();
        }
    }
    flush(&mut out, &mut getters, member_indent);
    out
}

static GV_ROOT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"<(screen|component)\b(?:[^>]*\btitle="([^"]*)")?"#));
static GV_PROP: LazyLock<Regex> = LazyLock::new(|| re(r#"<prop\s+name="(\w+)""#));
static GV_IMPORT: LazyLock<Regex> = LazyLock::new(|| re(r#"rel="import"[^>]*\bas="(\w+)""#));
static GV_SCRIPT: LazyLock<Regex> = LazyLock::new(|| re(r#"<script\s+src="([^"]+)""#));
static GV_VIEW: LazyLock<Regex> = LazyLock::new(|| re(r#"if="\{view\}"\s+equals="(\w+)""#));
static GV_HANDLER: LazyLock<Regex> = LazyLock::new(|| re(r#"\bon_\w+="([A-Za-z_][\w.]*)"#));

fn render_gv(text: &str) -> String {
    let mut out = String::new();
    // Comentários podem citar tags/atributos: ignorá-los evita falso positivo.
    let text = strip_blocks(text, "<!--", "-->");
    if let Some(c) = GV_ROOT.captures(&text) {
        match c.get(2) {
            Some(t) => {
                let _ = writeln!(out, "<{} \"{}\">", &c[1], t.as_str());
            }
            None => {
                let _ = writeln!(out, "<{}>", &c[1]);
            }
        }
    }
    let list = |re: &Regex, sorted: bool| -> Vec<String> {
        let mut v: Vec<String> = re.captures_iter(&text).map(|c| c[1].to_owned()).collect();
        if sorted {
            v = v.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
        } else {
            let mut seen = BTreeSet::new();
            v.retain(|x| seen.insert(x.clone()));
        }
        v
    };
    for (label, items) in [
        ("props", list(&GV_PROP, false)),
        ("imports", list(&GV_IMPORT, true)),
        ("script", list(&GV_SCRIPT, false)),
        ("views", list(&GV_VIEW, false)),
        ("handlers", list(&GV_HANDLER, true)),
    ] {
        if !items.is_empty() {
            let _ = writeln!(out, "{label}: {}", items.join(", "));
        }
    }
    out
}

static GVB_ROOT: LazyLock<Regex> = LazyLock::new(|| re(r"(?m)^\s*(app|screen|component)\b"));
static GVB_SCREEN_TITLE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?m)^\s*screen\([^)]*\btitle = "?([^",)]+)"#));
static GVB_SCREEN_NAME: LazyLock<Regex> = LazyLock::new(|| re(r"\bscreen\([^)]*\bname = (\w+)"));
static GVB_PROP: LazyLock<Regex> = LazyLock::new(|| re(r"\bprop\([^)]*?\bname = (\w+)"));
static GVB_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| re(r"\blink\([^)]*\brel = import[^)]*\bas = ([\w-]+)"));
static GVB_SCRIPT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"\bscript\([^)]*\bsrc = "?([^",)\s]+)"#));
static GVB_VIEW: LazyLock<Regex> = LazyLock::new(|| re(r#"@view\s*==\s*"(\w+)""#));
static GVB_TAB: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"@\w*tab\s*==\s*"(\w+)"|\bif = @\w*tab,\s*equals = "?(\w+)"#)
});
static GVB_HANDLER: LazyLock<Regex> =
    LazyLock::new(|| re(r"\b(?:on_\w+|action) = (?:app:)?([A-Za-z_]\w*)"));

/// Tira os comentários (`//` e `/* */`) de um `.gvb` sem tocar no que está dentro
/// de uma string (`"…"`, `"""…"""`, `l"""…"""`): um `https://` ou um CSS num
/// `style """…"""` não é comentário, e um comentário que cita `on_click = x` não
/// é um handler.
fn strip_gvb_comments(text: &str) -> String {
    let cs: Vec<char> = text.chars().collect();
    let starts = |i: usize, s: &str| s.chars().enumerate().all(|(k, c)| cs.get(i + k) == Some(&c));
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < cs.len() {
        if starts(i, "//") {
            while i < cs.len() && cs[i] != '\n' {
                i += 1;
            }
        } else if starts(i, "/*") {
            i += 2;
            while i < cs.len() && !starts(i, "*/") {
                i += 1;
            }
            i = (i + 2).min(cs.len());
        } else if starts(i, "\"\"\"") || starts(i, "l\"\"\"") {
            let open = if cs[i] == 'l' { 4 } else { 3 };
            out.extend(&cs[i..i + open]);
            i += open;
            while i < cs.len() && !starts(i, "\"\"\"") {
                out.push(cs[i]);
                i += 1;
            }
        } else if cs[i] == '"' {
            out.push('"');
            i += 1;
            while i < cs.len() && cs[i] != '"' {
                if cs[i] == '\\' && i + 1 < cs.len() {
                    out.push(cs[i]);
                    i += 1;
                }
                out.push(cs[i]);
                i += 1;
            }
            if i < cs.len() {
                out.push('"');
                i += 1;
            }
        } else {
            out.push(cs[i]);
            i += 1;
        }
    }
    out
}

fn render_gvb(text: &str) -> String {
    let mut out = String::new();
    let text = strip_gvb_comments(text);
    if let Some(c) = GVB_ROOT.captures(&text) {
        match (&c[1], GVB_SCREEN_TITLE.captures(&text)) {
            ("screen", Some(t)) => {
                let _ = writeln!(out, "<screen \"{}\">", &t[1]);
            }
            (root, _) => {
                let _ = writeln!(out, "<{root}>");
            }
        }
    }
    // Primeiro grupo que casou (`@x == "v"` ou `if = @x, equals = v`).
    let list = |re: &Regex, sorted: bool| -> Vec<String> {
        let mut v: Vec<String> = re
            .captures_iter(&text)
            .filter_map(|c| c.iter().skip(1).flatten().next().map(|m| m.as_str().to_owned()))
            .collect();
        if sorted {
            v = v.into_iter().collect::<BTreeSet<_>>().into_iter().collect();
        } else {
            let mut seen = BTreeSet::new();
            v.retain(|x| seen.insert(x.clone()));
        }
        v
    };
    for (label, items) in [
        ("props", list(&GVB_PROP, false)),
        ("imports", list(&GVB_IMPORT, true)),
        ("script", list(&GVB_SCRIPT, false)),
        ("telas", list(&GVB_SCREEN_NAME, false)),
        ("views", list(&GVB_VIEW, false)),
        ("abas", list(&GVB_TAB, false)),
        ("handlers", list(&GVB_HANDLER, true)),
    ] {
        if !items.is_empty() {
            let _ = writeln!(out, "{label}: {}", items.join(", "));
        }
    }
    out
}

static HTML_SECTION: LazyLock<Regex> = LazyLock::new(|| re(r"<!--\s*──\s*([^─]+?)\s*─"));
static HTML_XDATA: LazyLock<Regex> = LazyLock::new(|| re(r#"x-data="(\w+)""#));
static HTML_EVENT: LazyLock<Regex> = LazyLock::new(|| re(r#"@[\w.-]+="([^"]*)""#));
static CALL: LazyLock<Regex> = LazyLock::new(|| re(r"(\w+)\s*\("));

fn render_html(text: &str) -> String {
    let mut out = String::new();
    let sections: Vec<String> = HTML_SECTION
        .captures_iter(text)
        .map(|c| c[1].trim().to_owned())
        .collect();
    let xdata: BTreeSet<String> = HTML_XDATA
        .captures_iter(text)
        .map(|c| c[1].to_owned())
        .collect();
    let calls: BTreeSet<String> = HTML_EVENT
        .captures_iter(text)
        .flat_map(|c| {
            CALL.captures_iter(&c[1])
                .map(|m| m[1].to_owned())
                .collect::<Vec<_>>()
        })
        .filter(|name| !JS_KEYWORDS.contains(&name.as_str()))
        .collect();
    for (label, items) in [
        ("seções", sections),
        ("x-data", xdata.into_iter().collect()),
        ("chama", calls.into_iter().collect()),
    ] {
        if !items.is_empty() {
            let _ = writeln!(out, "{label}: {}", items.join(", "));
        }
    }
    out
}

fn strip_blocks(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(open) {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find(close) {
            Some(j) => &rest[i + j + close.len()..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gvb_ignora_comentarios_mas_nao_strings() {
        let src = r#"// Cabeçalho do arquivo.
// segunda linha
component {
  // on_click = fantasma
  resources {
    link(rel = import, href = "components/nav_item.gvb", as = NavItem)
    style """
      .a { color: red; } // não é comentário aqui? é CSS: fica
    """
  }
  button(on_click = salvar:@id, text = "https://x // y")
  NavItem(label = A, action = nav_a)
  if @view == "deployments" { x }
  ServiceLogsTab(if = @tab, equals = logs)
  if @proj_tab == "env" { y }
  prop(name = zzz)
  script(src = "scripts/a.luau")
}
"#;
        let r = render_gvb(src);
        assert!(r.starts_with("<component>\n"), "{r}");
        assert!(r.contains("imports: NavItem"), "{r}");
        assert!(r.contains("handlers: nav_a, salvar"), "{r}");
        assert!(!r.contains("fantasma"), "{r}");
        assert!(r.contains("views: deployments"), "{r}");
        assert!(r.contains("abas: logs, env"), "{r}");
        assert!(r.contains("script: scripts/a.luau"), "{r}");
        assert!(r.contains("props: zzz"), "{r}");
        assert_eq!(
            describe("views/x.gvb", src).as_deref(),
            Some("Cabeçalho do arquivo.")
        );
    }

    #[test]
    fn gvb_app_lista_as_telas() {
        let r = render_gvb("app(id = x) {\n  screen(name = main, initial = true, title = Rustploy) { }\n  screen(name = log, src = \"log.gvb\")\n}\n");
        assert!(r.starts_with("<app>\n"), "{r}");
        assert!(r.contains("telas: main, log"), "{r}");
    }
}
