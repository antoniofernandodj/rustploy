//! `comandos.md`: uma linha por variante de `Command` ligando as três pontas
//! de uma feature: o handler que a trata no daemon e quem a manda na GUI
//! (Luau) e na webui (JS).
//!
//! Tudo por nome. As variantes (e a doc de cada uma) saem do `enum Command`
//! via `syn`. O handler sai dos braços do `match` em `api/routes.rs` que
//! chamam `handlers::<arquivo>::<fn>`. Quem chama sai de uma busca pelo nome
//! da variante nos scripts, na forma em que um Command é montado
//! (`X = …`/`"X"` no Luau, `X: …`/`"X"` no JS, `"X"`/`\"X\"` no JSON da API de
//! agente da GUI), ignorando linhas de comentário.

use crate::{doc_summary, short, tokens};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
use syn::visit::Visit;

const PROTOCOL: &str = "crates/shared/src/protocol.rs";
const ROUTES: &str = "crates/daemon/src/api/routes.rs";
const GUI_SCRIPTS: &str = "crates/rustploy-gui/views/scripts/";
const WEBUI: &str = "crates/daemon/webui/";
const AGENT: &str = "crates/rustploy-gui/src/agent/";

struct Variant {
    name: String,
    group: String,
    doc: Option<String>,
}

pub fn render(root: &Path, files: &[String]) -> String {
    let variants = variants(root);
    let handlers = handlers(root);
    let gui = callers(
        root,
        files,
        GUI_SCRIPTS,
        "luau",
        "--",
        r#""([A-Z]\w+)"|\b([A-Z]\w+)\s*="#,
    );
    let web = callers(
        root,
        files,
        WEBUI,
        "js",
        "//",
        r#""([A-Z]\w+)"|\b([A-Z]\w+)\s*:"#,
    );
    let agent = callers(root, files, AGENT, "rs", "//", r#""([A-Z]\w+)\\?""#);

    let mut s = String::new();
    s.push_str("# Índice: Command → handler no daemon → quem chama\n\n");
    let _ = writeln!(
        s,
        "> Gerado por `cargo run -p indexer`; não editar à mão. Uma linha por variante do\n\
         > `enum Command` (`{PROTOCOL}`), agrupada pelos comentários do enum.\n\
         > `daemon:` = `crates/daemon/src/api/handlers/<arquivo>.rs` (`::fn` quando não é `handle`);\n\
         > `gui:` = arquivos sob `{GUI_SCRIPTS}`; `web:` = sob `{WEBUI}`; `—` = ninguém;\n\
         > `agente:` (só quando há) = sob `{AGENT}` (API de agente da GUI; `catalog` = só documentado)."
    );

    let mut group = "";
    let mut only_gui = Vec::new();
    let mut only_web = Vec::new();
    let mut nobody: Vec<String> = Vec::new();
    for v in &variants {
        if v.group != group {
            group = &v.group;
            let _ = writeln!(
                s,
                "\n## {}",
                if group.is_empty() {
                    "(sem grupo)"
                } else {
                    group
                }
            );
        }
        let join = |set: Option<&BTreeSet<String>>| match set {
            Some(set) if !set.is_empty() => set.iter().cloned().collect::<Vec<_>>().join(", "),
            _ => "—".to_owned(),
        };
        let (g, w) = (gui.get(&v.name), web.get(&v.name));
        match (g.is_some(), w.is_some()) {
            (true, false) => only_gui.push(v.name.as_str()),
            (false, true) => only_web.push(v.name.as_str()),
            (false, false) => nobody.push(match agent.contains_key(&v.name) {
                true => format!("{} (agente)", v.name),
                false => v.name.clone(),
            }),
            _ => {}
        }
        let daemon = handlers
            .get(&v.name)
            .map(String::as_str)
            .unwrap_or("(inline em routes.rs)");
        let _ = write!(s, "{}", v.name);
        if let Some(d) = &v.doc {
            let _ = write!(s, " — {}", short(d, 70));
        }
        let _ = write!(
            s,
            " | daemon: {daemon} | gui: {} | web: {}",
            join(g),
            join(w)
        );
        if let Some(a) = agent.get(&v.name) {
            let _ = write!(s, " | agente: {}", join(Some(a)));
        }
        s.push('\n');
    }

    // A regra do projeto é que GUI e webui andem juntas: o que só um lado
    // usa é, na maioria das vezes, uma lacuna de paridade.
    s.push_str("\n## Paridade\n\n");
    let nobody: Vec<&str> = nobody.iter().map(String::as_str).collect();
    for (label, list) in [
        ("Só GUI", only_gui),
        ("Só webui", only_web),
        ("Nem GUI nem webui", nobody),
    ] {
        let _ = writeln!(
            s,
            "- {label} ({}): {}",
            list.len(),
            if list.is_empty() {
                "—".into()
            } else {
                list.join(", ")
            }
        );
    }
    s
}

/// Variantes na ordem de declaração, com o grupo dado pelo último comentário
/// `// Grupo` visto acima delas (o `syn` descarta comentários comuns, então o
/// grupo sai de uma leitura por linha do mesmo trecho).
fn variants(root: &Path) -> Vec<Variant> {
    let text = fs::read_to_string(root.join(PROTOCOL)).expect("ler protocol.rs");
    let file = syn::parse_file(&text).expect("parsear protocol.rs");
    let Some(en) = file.items.iter().find_map(|i| match i {
        syn::Item::Enum(e) if e.ident == "Command" => Some(e),
        _ => None,
    }) else {
        return Vec::new();
    };

    let mut groups: BTreeMap<String, String> = BTreeMap::new();
    let variant_line = Regex::new(r"^    (\w+)\b").unwrap();
    let group_line = Regex::new(r"^    //\s*(.+?)\s*$").unwrap();
    let body = text
        .split_once("pub enum Command {")
        .map(|(_, b)| b)
        .unwrap_or("");
    let mut group = String::new();
    for line in body.lines() {
        if line.starts_with('}') {
            break;
        }
        if let Some(c) = group_line.captures(line) {
            group = c[1]
                .trim_matches(|c: char| c == '─' || c == '-' || c.is_whitespace())
                .to_owned();
        } else if let Some(c) = variant_line.captures(line) {
            groups
                .entry(c[1].to_owned())
                .or_insert_with(|| group.clone());
        }
    }

    en.variants
        .iter()
        .map(|v| Variant {
            name: v.ident.to_string(),
            group: groups
                .get(&v.ident.to_string())
                .cloned()
                .unwrap_or_default(),
            doc: doc_summary(&v.attrs),
        })
        .collect()
}

/// Variante → `arquivo` (ou `arquivo::fn`) do handler, dos braços de todo
/// `match` em `routes.rs` cujo corpo chama `handlers::…`.
fn handlers(root: &Path) -> BTreeMap<String, String> {
    struct Arms(BTreeMap<String, String>);
    impl<'ast> Visit<'ast> for Arms {
        fn visit_arm(&mut self, arm: &'ast syn::Arm) {
            let pat = tokens(&arm.pat);
            let body = tokens(&arm.body);
            let variant = pat.strip_prefix("Command::").map(|r| {
                r.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("")
            });
            if let (Some(variant), Some(i)) = (variant, body.find("handlers::")) {
                let path: String = body[i + "handlers::".len()..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                    .collect();
                let path = path.strip_suffix("::handle").unwrap_or(&path).to_owned();
                self.0.entry(variant.to_owned()).or_insert(path);
            }
            syn::visit::visit_arm(self, arm);
        }
    }
    let text = fs::read_to_string(root.join(ROUTES)).expect("ler routes.rs");
    let file = syn::parse_file(&text).expect("parsear routes.rs");
    let mut arms = Arms(BTreeMap::new());
    arms.visit_file(&file);
    arms.0
}

/// Variante → arquivos (relativos a `prefix`) que a mencionam como Command.
/// `pattern` captura o nome no grupo 1 ou 2.
fn callers(
    root: &Path,
    files: &[String],
    prefix: &str,
    ext: &str,
    comment: &str,
    pattern: &str,
) -> BTreeMap<String, BTreeSet<String>> {
    let re = Regex::new(pattern).unwrap();
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files
        .iter()
        .filter(|p| p.starts_with(prefix) && p.ends_with(&format!(".{ext}")))
    {
        let Ok(text) = fs::read_to_string(root.join(path)) else {
            continue;
        };
        let rel = path[prefix.len()..]
            .trim_end_matches(&format!(".{ext}"))
            .to_owned();
        for line in text
            .lines()
            .filter(|l| !l.trim_start().starts_with(comment))
        {
            for c in re.captures_iter(line) {
                let name = c.get(1).or(c.get(2)).unwrap().as_str();
                out.entry(name.to_owned()).or_default().insert(rel.clone());
            }
        }
    }
    out
}
