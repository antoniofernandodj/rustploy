//! Gera `docs/indice/`: um mapa de arquivos e símbolos pensado para um agente
//! achar "onde está a responsabilidade X" lendo pouco.
//!
//! Nada aqui grava número de linha — ele muda a cada edição e deixaria o índice
//! mentindo. Cada coisa é endereçada por caminho + nome; a linha exata sai de
//! `grep -n "fn nome" <arquivo>` na hora. Assim o índice só muda quando a
//! *estrutura* muda (símbolo criado, renomeado ou apagado).
//!
//! A saída é determinística (caminhos em ordem, itens na ordem de declaração)
//! para não gerar diff sem mudança real. Plano: `docs/plano-indice-de-codigo.md`.

mod commands;
mod script;

use quote::ToTokens;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const OUT_DIR: &str = "docs/indice";
/// Índice transversal (não é área de prefixo): ver `commands.rs`.
const COMMANDS_MD: &str = "comandos.md";

/// Áreas do índice de símbolos: (arquivo gerado, título, prefixos cobertos).
/// O primeiro prefixo que casar vence, por isso os mais específicos vêm antes.
const AREAS: &[(&str, &str, &[&str])] = &[
    (
        "shared.md",
        "crates/shared — modelos, protocolo, manifest, templates",
        &["crates/shared/"],
    ),
    (
        "daemon-api.md",
        "daemon: API HTTP e handlers de Command",
        &["crates/daemon/src/api/"],
    ),
    (
        "daemon-deploy.md",
        "daemon: deploy, jobs e manutenção",
        &[
            "crates/daemon/src/deploy/",
            "crates/daemon/src/jobs/",
            "crates/daemon/src/maintenance/",
        ],
    ),
    (
        "daemon-db.md",
        "daemon: persistência (db/)",
        &["crates/daemon/src/db/"],
    ),
    (
        "daemon-docker.md",
        "daemon: Docker/Compose e ingress (proxy, TLS)",
        &["crates/daemon/src/docker/", "crates/daemon/src/ingress/"],
    ),
    (
        "daemon-registry.md",
        "daemon: registry embutido e provedores git",
        &[
            "crates/daemon/src/registry/",
            "crates/daemon/src/git_providers/",
        ],
    ),
    (
        "webui.md",
        "webui (HTML + Alpine.js) servida pelo daemon",
        &["crates/daemon/webui/"],
    ),
    (
        "daemon-core.md",
        "daemon: main, bins, event bus, secrets, métricas e o resto",
        &["crates/daemon/"],
    ),
    (
        "gui-handlers.md",
        "rustploy-gui: handlers Luau (as ações que os .gv disparam)",
        &["crates/rustploy-gui/views/scripts/handlers/"],
    ),
    (
        "gui-scripts.md",
        "rustploy-gui: demais scripts Luau (estado, rede, formatação, janelas)",
        &["crates/rustploy-gui/views/scripts/"],
    ),
    (
        "gui-views.md",
        "rustploy-gui: telas e componentes .gv, estilos",
        &["crates/rustploy-gui/views/"],
    ),
    ("gui.md", "rustploy-gui (Rust)", &["crates/rustploy-gui/"]),
    ("importer.md", "importer", &["crates/importer/"]),
    ("tools.md", "ferramentas do repositório", &["tools/"]),
];

/// Diretórios de dados que viram uma linha só no INDEX.md.
const COLLAPSE: &[(&str, &str)] = &[(
    "crates/shared/templates/blueprints/",
    "catálogo de templates de app (formato Dokploy), compilado pelo build.rs do shared",
)];

/// Diretórios que o INDEX.md lista só por nome, numa linha: a descrição de
/// cada arquivo repetiria um índice que já cobre o diretório melhor.
const NAMES_ONLY: &[(&str, &str)] = &[(
    "crates/daemon/src/api/handlers/",
    "um arquivo por `Command` (exceto `mod.rs`); o que cada um faz está em `comandos.md` e `daemon-api.md`",
)];

const BINARY_EXT: &[&str] = &[
    "png", "svg", "webp", "jpg", "jpeg", "gif", "ico", "ttf", "otf", "woff", "woff2",
];

/// Traits cujos métodos são ditados pela própria trait: listar os métodos só
/// gastaria token. Viram uma linha `impl A, B for Tipo`.
const COMMON_TRAITS: &[&str] = &[
    "Display",
    "Debug",
    "From",
    "Into",
    "TryFrom",
    "Default",
    "Clone",
    "Drop",
    "FromStr",
    "Error",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Deref",
    "DerefMut",
    "AsRef",
    "Borrow",
    "Iterator",
    "IntoIterator",
    "Future",
    "Stream",
    "Serialize",
    "Deserialize",
    "Visitor",
    "Service",
    "Body",
    "Send",
    "Sync",
    "Write",
    "Read",
    "AsyncRead",
    "AsyncWrite",
    "IntoResponse",
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(errors) => {
            eprintln!("indexer: {OUT_DIR}/ NÃO foi alterado. Corrija e rode de novo:");
            for e in errors {
                eprintln!("  - {e}");
            }
            ExitCode::FAILURE
        }
    }
}

/// Primeiro gera tudo em memória e checa; só grava se nada falhou. Um índice
/// pela metade (parte nova, parte velha) é pior que o anterior inteiro.
fn run() -> Result<(), Vec<String>> {
    let root = repo_root().map_err(|e| vec![e])?;
    let files = tracked_files(&root).map_err(|e| vec![e])?;
    let mut errors = Vec::new();

    // ── 1. Gera e checa ──────────────────────────────────────────────────────
    let mut areas: Vec<(&str, &str, Vec<&String>)> =
        AREAS.iter().map(|(f, t, _)| (*f, *t, Vec::new())).collect();
    for path in files.iter().filter(|p| is_indexed(p)) {
        if let Some(i) = area_of(path) {
            areas[i].2.push(path);
        }
    }
    let mut outputs: Vec<(String, String)> = Vec::new();
    for (i, (file, title, paths)) in areas.iter().enumerate() {
        // Área vazia = prefixo que não casa mais nada (diretório movido ou
        // renomeado): o índice dela sumiria em silêncio.
        if paths.is_empty() {
            errors.push(format!(
                "área `{file}` ficou vazia: nenhum arquivo em {} (diretório movido? ajuste AREAS)",
                AREAS[i].2.join(", ")
            ));
            continue;
        }
        outputs.push((
            file.to_string(),
            render_area(&root, title, paths, &mut errors),
        ));
    }
    match commands::render(&root, &files) {
        Ok(text) => outputs.push((COMMANDS_MD.into(), text)),
        Err(e) => errors.push(e),
    }
    outputs.push(("INDEX.md".into(), render_index(&root, &files)));
    if !errors.is_empty() {
        return Err(errors);
    }

    // ── 2. Grava ─────────────────────────────────────────────────────────────
    let out = root.join(OUT_DIR);
    fs::create_dir_all(&out).map_err(|e| vec![format!("{OUT_DIR}: {e}")])?;
    for (file, text) in &outputs {
        fs::write(out.join(file), text).map_err(|e| vec![format!("{OUT_DIR}/{file}: {e}")])?;
    }
    // Só depois de tudo gravado: área renomeada/removida não pode deixar um
    // índice velho para trás.
    if let Ok(entries) = fs::read_dir(&out) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".md") && !outputs.iter().any(|(f, _)| *f == name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    // ~4 bytes por token é a estimativa grosseira usual; serve para comparar.
    for (file, text) in &outputs {
        eprintln!(
            "{OUT_DIR}/{file}: {} bytes (~{} tokens)",
            text.len(),
            text.len() / 4
        );
    }
    Ok(())
}

fn repo_root() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("git não rodou: {e}"))?;
    if !out.status.success() {
        return Err("não é um repositório git".into());
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// Arquivos versionados + novos não ignorados, para indexar antes do commit.
fn tracked_files(root: &Path) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .output()
        .map_err(|e| format!("git ls-files não rodou: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git ls-files falhou: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|p| root.join(p).is_file())
        .map(str::to_owned)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

fn ext_of(path: &str) -> &str {
    path.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
}

/// Arquivos cujos símbolos entram nos índices de área.
fn is_indexed(path: &str) -> bool {
    let ext = ext_of(path);
    ext == "rs" || script::EXTENSIONS.contains(&ext)
}

fn area_of(path: &str) -> Option<usize> {
    AREAS
        .iter()
        .position(|(_, _, prefixes)| prefixes.iter().any(|p| path.starts_with(p)))
}

fn split_dir(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(i) => (&path[..=i], &path[i + 1..]),
        None => ("", path),
    }
}

// ── INDEX.md ─────────────────────────────────────────────────────────────────

fn render_index(root: &Path, files: &[String]) -> String {
    let mut s = String::new();
    s.push_str("# Índice do repositório\n\n");
    s.push_str(
        "> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:\n\
         > ache o arquivo aqui, o símbolo no índice da área e a linha com\n\
         > `grep -n \"fn nome\" <arquivo>`. Leia só o trecho, nunca o arquivo grande inteiro.\n\n",
    );
    s.push_str("## Índices de símbolos por área\n\n");
    for (file, title, prefixes) in AREAS {
        let _ = writeln!(s, "- `{file}` — {title} ({})", prefixes.join(", "));
    }
    let _ = writeln!(
        s,
        "- `{COMMANDS_MD}` — cada `Command`: handler no daemon + quem chama na GUI e na webui, e a paridade"
    );
    s.push_str("\n## Árvore\n");

    let mut dirs: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut binaries: BTreeMap<&str, usize> = BTreeMap::new();
    let mut collapsed: BTreeMap<&str, usize> = BTreeMap::new();
    for path in files {
        // O próprio índice já está listado em "Índices de símbolos por área".
        if path.starts_with(OUT_DIR) {
            continue;
        }
        if let Some((prefix, _)) = COLLAPSE.iter().find(|(p, _)| path.starts_with(p)) {
            *collapsed.entry(prefix).or_default() += 1;
            continue;
        }
        let (dir, name) = split_dir(path);
        let ext = name.rsplit('.').next().unwrap_or("");
        if BINARY_EXT.contains(&ext) {
            *binaries.entry(dir).or_default() += 1;
            continue;
        }
        dirs.entry(dir).or_default().push(name);
    }
    // Garante que o diretório-pai de um colapsado e os que só têm imagens
    // apareçam na árvore, mesmo sem nenhum arquivo listável.
    for prefix in collapsed.keys() {
        dirs.entry(split_dir(prefix.trim_end_matches('/')).0)
            .or_default();
    }

    for (dir, names) in &dirs {
        let label = if dir.is_empty() { "(raiz)" } else { dir };
        let _ = write!(s, "\n### {label}");
        let area = names
            .iter()
            .map(|n| format!("{dir}{n}"))
            .filter(|p| is_indexed(p))
            .find_map(|p| area_of(&p));
        if let Some(i) = area {
            let _ = write!(s, " → {}", AREAS[i].0);
        }
        s.push('\n');
        if let Some((_, desc)) = NAMES_ONLY.iter().find(|(p, _)| p == dir) {
            let stems: Vec<&str> = names.iter().map(|n| n.trim_end_matches(".rs")).collect();
            let _ = writeln!(
                s,
                "- {} arquivos, {desc}: {}",
                names.len(),
                stems.join(", ")
            );
            continue;
        }
        for name in names {
            let path = format!("{dir}{name}");
            match describe_file(root, &path) {
                Some(d) => {
                    let _ = writeln!(s, "- {name} — {}", short(&d, 80));
                }
                None => {
                    let _ = writeln!(s, "- {name}");
                }
            }
        }
        for (prefix, desc) in COLLAPSE {
            if split_dir(prefix.trim_end_matches('/')).0 == *dir {
                let n = collapsed.get(prefix).copied().unwrap_or(0);
                let sub = split_dir(prefix.trim_end_matches('/')).1;
                let _ = writeln!(s, "- {sub}/ — {desc} ({n} arquivos, não indexados)");
            }
        }
        if let Some(n) = binaries.get(dir) {
            let _ = writeln!(s, "- (+{n} imagens/fontes)");
        }
    }
    s
}

/// Uma linha de descrição por arquivo, só de fontes baratas e confiáveis.
fn describe_file(root: &Path, path: &str) -> Option<String> {
    let text = fs::read_to_string(root.join(path)).ok()?;
    if path.ends_with(".rs") {
        let file = syn::parse_file(&text).ok()?;
        doc_summary(&file.attrs)
    } else if script::EXTENSIONS.contains(&ext_of(path)) {
        script::describe(path, &text)
    } else if path.ends_with(".md") {
        let line = text.lines().find(|l| l.starts_with("# "))?;
        Some(line[2..].trim().to_owned())
    } else if path.ends_with("Cargo.toml") {
        let line = text.lines().find(|l| l.starts_with("description"))?;
        Some(line.split_once('=')?.1.trim().trim_matches('"').to_owned())
    } else {
        None
    }
}

// ── Áreas ────────────────────────────────────────────────────────────────────

const RUST_NOTATION: &str = "> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;\n\
     > `impl A, B for T` = impls de traits comuns (métodos omitidos).\n";

/// Um arquivo que não dá para ler ou parsear vai para `errors` (e o índice
/// não é gravado), em vez de virar uma entrada quebrada no meio do índice.
fn render_area(root: &Path, title: &str, paths: &[&String], errors: &mut Vec<String>) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Índice: {title}\n");
    s.push_str(
        "> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:\n\
         > `grep -n \"nome\" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.\n",
    );
    let mut exts: Vec<&str> = paths.iter().map(|p| ext_of(p)).collect();
    exts.sort();
    exts.dedup();
    for ext in exts {
        let note = if ext == "rs" {
            Some(RUST_NOTATION)
        } else {
            script::notation(ext)
        };
        s.push_str(note.unwrap_or(""));
    }
    let mut last_dir = "";
    for path in paths {
        let (dir, name) = split_dir(path);
        if dir != last_dir {
            let _ = writeln!(s, "\n## {dir}");
            last_dir = dir;
        }
        let text = match fs::read_to_string(root.join(path)) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{path}: {e}"));
                continue;
            }
        };
        if ext_of(path) != "rs" {
            match script::describe(path, &text) {
                Some(d) => {
                    let _ = writeln!(s, "\n### {name} — {d}");
                }
                None => {
                    let _ = writeln!(s, "\n### {name}");
                }
            }
            s.push_str(&script::render(path, &text));
            continue;
        }
        match syn::parse_file(&text) {
            Ok(file) => {
                let mut r = Renderer::default();
                r.items(&file.items, "");
                match doc_summary(&file.attrs) {
                    Some(d) => {
                        let _ = writeln!(s, "\n### {name} — {d}");
                    }
                    None => {
                        let _ = writeln!(s, "\n### {name}");
                    }
                }
                s.push_str(&r.out);
                if r.tests > 0 {
                    let _ = writeln!(s, "({} testes)", r.tests);
                }
            }
            Err(e) => errors.push(format!("{path}: não parseou: {e}")),
        }
    }
    s
}

#[derive(Default)]
struct Renderer {
    out: String,
    tests: usize,
}

impl Renderer {
    fn line(&mut self, indent: &str, body: String, attrs: &[syn::Attribute]) {
        match doc_summary(attrs) {
            Some(d) => {
                let _ = writeln!(self.out, "{indent}{body} — {d}");
            }
            None => {
                let _ = writeln!(self.out, "{indent}{body}");
            }
        }
    }

    fn items(&mut self, items: &[syn::Item], indent: &str) {
        // impls de traits comuns agrupados por tipo, emitidos no fim.
        let mut common: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut consts: Vec<String> = Vec::new();

        for item in items {
            use syn::Item::*;
            match item {
                Fn(f) => {
                    if is_test(&f.attrs) {
                        self.tests += 1;
                    } else {
                        self.line(indent, format!("fn {}", signature(&f.sig)), &f.attrs);
                    }
                }
                Struct(st) => {
                    let body = match &st.fields {
                        syn::Fields::Named(n) => {
                            let names: Vec<String> = n
                                .named
                                .iter()
                                .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
                                .collect();
                            format!("struct {} {{ {} }}", st.ident, names.join(", "))
                        }
                        syn::Fields::Unnamed(u) => {
                            let tys: Vec<String> =
                                u.unnamed.iter().map(|f| tokens(&f.ty)).collect();
                            format!("struct {}({})", st.ident, tys.join(", "))
                        }
                        syn::Fields::Unit => format!("struct {}", st.ident),
                    };
                    self.line(indent, body, &st.attrs);
                }
                Enum(en) => {
                    let names: Vec<String> =
                        en.variants.iter().map(|v| v.ident.to_string()).collect();
                    self.line(
                        indent,
                        format!("enum {} {{ {} }}", en.ident, names.join(", ")),
                        &en.attrs,
                    );
                }
                Union(u) => self.line(indent, format!("union {}", u.ident), &u.attrs),
                Trait(t) => {
                    self.line(indent, format!("trait {}", t.ident), &t.attrs);
                    let inner = format!("{indent}  ");
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(f) = ti {
                            self.line(&inner, format!("fn {}", signature(&f.sig)), &f.attrs);
                        }
                    }
                }
                Type(t) => self.line(
                    indent,
                    format!("type {} = {}", t.ident, short(&tokens(&t.ty), 50)),
                    &t.attrs,
                ),
                Const(c) => self.const_like(indent, c.ident.to_string(), &c.attrs, &mut consts),
                Static(c) => self.const_like(indent, c.ident.to_string(), &c.attrs, &mut consts),
                Macro(m) => {
                    if let Some(id) = &m.ident {
                        self.line(indent, format!("macro_rules! {id}"), &m.attrs);
                    }
                }
                Mod(m) => {
                    if is_cfg_test(&m.attrs) {
                        if let Some((_, inner)) = &m.content {
                            self.tests += count_tests(inner);
                        }
                    } else if let Some((_, inner)) = &m.content {
                        self.line(indent, format!("mod {}", m.ident), &m.attrs);
                        self.items(inner, &format!("{indent}  "));
                    }
                }
                Impl(im) => self.impl_block(im, indent, &mut common),
                _ => {}
            }
        }
        if !consts.is_empty() {
            let _ = writeln!(self.out, "{indent}const {}", consts.join(", "));
        }
        for (ty, traits) in common {
            let _ = writeln!(self.out, "{indent}impl {} for {ty}", traits.join(", "));
        }
    }

    /// Constantes documentadas ganham linha própria; as outras viram uma lista.
    fn const_like(
        &mut self,
        indent: &str,
        name: String,
        attrs: &[syn::Attribute],
        rest: &mut Vec<String>,
    ) {
        if doc_summary(attrs).is_some() {
            self.line(indent, format!("const {name}"), attrs);
        } else {
            rest.push(name);
        }
    }

    fn impl_block(
        &mut self,
        im: &syn::ItemImpl,
        indent: &str,
        common: &mut BTreeMap<String, Vec<String>>,
    ) {
        let ty = tokens(&im.self_ty);
        let fns: Vec<&syn::ImplItemFn> = im
            .items
            .iter()
            .filter_map(|i| {
                if let syn::ImplItem::Fn(f) = i {
                    Some(f)
                } else {
                    None
                }
            })
            .collect();

        let Some((_, trait_path, _)) = &im.trait_ else {
            self.line(indent, format!("impl {ty}"), &im.attrs);
            let inner = format!("{indent}  ");
            for f in fns {
                if is_test(&f.attrs) {
                    self.tests += 1;
                } else {
                    self.line(&inner, format!("fn {}", signature(&f.sig)), &f.attrs);
                }
            }
            return;
        };

        let trait_name = trait_path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        let trait_str = tokens(trait_path);
        if COMMON_TRAITS.contains(&trait_name.as_str()) {
            common.entry(ty).or_default().push(trait_str);
        } else {
            // Trait do projeto: os nomes dos métodos ajudam a achar a
            // implementação concreta; a doc de cada um mora na trait.
            let names: Vec<String> = fns.iter().map(|f| f.sig.ident.to_string()).collect();
            self.line(
                indent,
                format!("impl {trait_str} for {ty}: {}", names.join(", ")),
                &im.attrs,
            );
        }
    }
}

fn count_tests(items: &[syn::Item]) -> usize {
    items
        .iter()
        .map(|i| match i {
            syn::Item::Fn(f) if is_test(&f.attrs) => 1,
            syn::Item::Mod(m) => m.content.as_ref().map(|(_, c)| count_tests(c)).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// `#[test]`, `#[tokio::test]` e afins.
fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && tokens(&a.meta).contains("test"))
}

/// `nome(a, b) -> Ret`: só os nomes dos parâmetros (os tipos custam caro e
/// raramente ajudam a achar) e o retorno só quando é curto o bastante.
fn signature(sig: &syn::Signature) -> String {
    let params: Vec<String> = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pt) => Some(match &*pt.pat {
                syn::Pat::Ident(pi) => pi.ident.to_string(),
                other => tokens(other),
            }),
        })
        .collect();
    let mut s = format!("{}({})", sig.ident, params.join(", "));
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        let ret = tokens(ty);
        if ret.len() <= 40 {
            let _ = write!(s, " -> {ret}");
        }
    }
    s
}

/// Tokens → texto compacto (`Result < Vec < T > >` → `Result<Vec<T>>`).
fn tokens<T: ToTokens>(t: &T) -> String {
    let mut s = t.to_token_stream().to_string();
    for (from, to) in [
        (" < ", "<"),
        ("< ", "<"),
        (" <", "<"),
        (" >", ">"),
        (" :: ", "::"),
        (":: ", "::"),
        ("& ", "&"),
        (" ,", ","),
        ("' ", "'"),
        ("( ", "("),
        (" )", ")"),
        ("[ ", "["),
        (" ]", "]"),
    ] {
        s = s.replace(from, to);
    }
    s
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_owned()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

/// Primeira frase da doc (`///` ou `//!`), limitada, numa linha só.
fn doc_summary(attrs: &[syn::Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for a in attrs {
        let syn::Meta::NameValue(nv) = &a.meta else {
            continue;
        };
        if !nv.path.is_ident("doc") {
            continue;
        }
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = &nv.value
        {
            lines.push(s.value());
        }
    }
    summarize(lines.iter().map(String::as_str))
}

/// Primeira frase do primeiro parágrafo de um comentário, limitada, numa
/// linha só. Um título markdown (`# X`) vale como resumo.
fn summarize<'a>(lines: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut para = String::new();
    for l in lines.map(str::trim) {
        if l.is_empty() {
            if para.is_empty() {
                continue;
            }
            break;
        }
        let l = l.trim_start_matches('#').trim();
        // Linha quebrada em "round-" / "a/" continua a mesma palavra.
        if !para.is_empty() && !para.ends_with(['-', '/']) {
            para.push(' ');
        }
        para.push_str(l);
    }
    if para.is_empty() {
        return None;
    }
    // Fim da frase = ". " que não fecha um item numerado ("1. ", "2. ").
    let end = para.match_indices(". ").map(|(i, _)| i).find(|&i| {
        let before = para[..i].trim_end_matches(|c: char| c.is_ascii_digit());
        before.len() == i || !(before.is_empty() || before.ends_with(char::is_whitespace))
    });
    let sentence = match end {
        Some(i) => &para[..=i],
        None => &para,
    };
    Some(short(sentence.trim(), 100))
}
