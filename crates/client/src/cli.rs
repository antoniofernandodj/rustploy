//! Subcomandos não-interativos de Infra-as-Code: `apply` e `export`.
//!
//! Estes comandos rodam fora da TUI (igual ao `import`): leem/escrevem YAML,
//! falam com o daemon via [`DaemonClient`] e saem.

use anyhow::{anyhow, bail, Context, Result};
use shared::{
    ApplyReport, Command, ProjectEntry, ProjectManifest, Response, ServerManifest,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::transport::DaemonClient;

/// Resolve o socket do daemon testando os candidatos com `ping`.
fn resolve_socket() -> Result<String> {
    let candidates = shared::RustployConfig::global().client_socket_candidates();
    for path in &candidates {
        if DaemonClient::new(path).ping() {
            return Ok(path.clone());
        }
    }
    bail!(
        "daemon não encontrado. Inicie o daemon primeiro: rustployd\nCaminhos tentados: {}",
        candidates.join(", ")
    )
}

// --------------------------------------------------------------------------
// apply
// --------------------------------------------------------------------------

/// `rustploy apply -f <arquivo> [--env-file <.env>] [--dry-run]`
pub fn run_apply(args: &[String]) -> Result<()> {
    let opts = ApplyOpts::parse(args)?;

    let path = PathBuf::from(&opts.file);
    let mut projects =
        load_manifests(&path).with_context(|| format!("ao carregar {}", path.display()))?;
    if projects.is_empty() {
        bail!("nenhum projeto encontrado no manifesto");
    }

    // Interpolação ${VAR}: ambiente do processo tem precedência sobre o --env-file.
    let env_file = match &opts.env_file {
        Some(p) => load_env_file(Path::new(p))?,
        None => HashMap::new(),
    };
    let lookup = |k: &str| std::env::var(k).ok().or_else(|| env_file.get(k).cloned());

    let mut missing = Vec::new();
    for m in &mut projects {
        for var in m.interpolate(&lookup) {
            if !missing.contains(&var) {
                missing.push(var);
            }
        }
    }
    if !missing.is_empty() {
        bail!(
            "variáveis não resolvidas (defina no ambiente ou --env-file): {}",
            missing.join(", ")
        );
    }

    // Os manifestos trafegam como YAML (postcard não suporta os defaults/skips
    // dos structs do manifesto); o daemon faz o parse com serde_yaml.
    let manifests = projects
        .iter()
        .map(serde_yaml::to_string)
        .collect::<Result<Vec<_>, _>>()?;

    if opts.dry_run {
        for y in &manifests {
            println!("---\n{y}");
        }
        println!("✨ dry-run: nada foi enviado ao daemon.");
        return Ok(());
    }

    let socket = resolve_socket()?;
    let client = DaemonClient::new(&socket);
    match client.send(Command::ManifestApply {
        manifests,
        prune: opts.prune,
        deploy: opts.deploy,
    })? {
        Response::ManifestReport(report) => {
            print_report(&report);
            Ok(())
        }
        Response::Err { code, message } => bail!("{code}: {message}"),
        other => bail!("resposta inesperada do daemon: {other:?}"),
    }
}

fn print_report(report: &ApplyReport) {
    let (mut created, mut updated, mut unchanged, mut deleted) = (0u32, 0u32, 0u32, 0u32);
    for a in &report.actions {
        use shared::ActionVerb::*;
        match a.action {
            Created => created += 1,
            Updated => updated += 1,
            Unchanged => unchanged += 1,
            Deleted => deleted += 1,
        }
        println!("  [{}] {} {}", a.action, a.kind, a.name);
    }
    println!(
        "\n🎉 apply concluído: {created} criado(s), {updated} atualizado(s), \
         {unchanged} inalterado(s), {deleted} removido(s)."
    );
    if !report.deployed.is_empty() {
        println!("🚀 deploy disparado para: {}", report.deployed.join(", "));
    }
}

// --------------------------------------------------------------------------
// export
// --------------------------------------------------------------------------

/// `rustploy export <projeto> [-o <arquivo>]` — projeto por nome ou id.
pub fn run_export(args: &[String]) -> Result<()> {
    let opts = ExportOpts::parse(args)?;

    let socket = resolve_socket()?;
    let client = DaemonClient::new(&socket);

    // Resolver nome -> id (aceita id direto também).
    let projects = match client.send(Command::ProjectList)? {
        Response::Projects(p) => p,
        Response::Err { code, message } => bail!("{code}: {message}"),
        other => bail!("resposta inesperada do daemon: {other:?}"),
    };
    let project = projects
        .iter()
        .find(|p| p.id == opts.project || p.name == opts.project)
        .ok_or_else(|| anyhow!("projeto '{}' não encontrado", opts.project))?;

    let yaml = match client.send(Command::ManifestExport {
        project_id: project.id.clone(),
    })? {
        Response::Manifest(y) => y,
        Response::Err { code, message } => bail!("{code}: {message}"),
        other => bail!("resposta inesperada do daemon: {other:?}"),
    };

    match &opts.output {
        Some(path) => {
            std::fs::write(path, &yaml)?;
            println!("💾 manifesto exportado para {path}");
        }
        None => print!("{yaml}"),
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Carregamento de manifestos
// --------------------------------------------------------------------------

/// Lê um arquivo e devolve a lista de projetos: arquivo por-projeto (`project:`)
/// ou manifesto raiz (`projects:` com inline/`include:`).
fn load_manifests(path: &Path) -> Result<Vec<ProjectManifest>> {
    let text = std::fs::read_to_string(path)?;
    let value: serde_yaml::Value = serde_yaml::from_str(&text)?;

    if value.get("projects").is_some() {
        let server: ServerManifest = serde_yaml::from_value(value)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        let mut out = Vec::new();
        for entry in server.projects {
            match entry {
                ProjectEntry::Inline(m) => out.push(m),
                ProjectEntry::Include { include } => {
                    let inc_path = base.join(&include);
                    let mut nested = load_manifests(&inc_path)
                        .with_context(|| format!("ao incluir {}", inc_path.display()))?;
                    out.append(&mut nested);
                }
            }
        }
        Ok(out)
    } else if value.get("project").is_some() {
        let m: ProjectManifest = serde_yaml::from_value(value)?;
        Ok(vec![m])
    } else {
        bail!("manifesto inválido: esperado a chave `project:` ou `projects:` no topo")
    }
}

/// Parser simples de `.env`: linhas `KEY=VALUE`, ignora vazias e `#` comentários.
fn load_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("ao ler env-file {}", path.display()))?;
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'');
            map.insert(k.trim().to_string(), v.to_string());
        }
    }
    Ok(map)
}

// --------------------------------------------------------------------------
// Parse de flags (manual — poucos argumentos, evita acoplar ao clap derive)
// --------------------------------------------------------------------------

struct ApplyOpts {
    file: String,
    env_file: Option<String>,
    dry_run: bool,
    prune: bool,
    deploy: bool,
}

impl ApplyOpts {
    fn parse(args: &[String]) -> Result<Self> {
        let mut file = None;
        let mut env_file = None;
        let mut dry_run = false;
        let mut prune = false;
        let mut deploy = false;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "-f" | "--file" => file = it.next().cloned(),
                "--env-file" => env_file = it.next().cloned(),
                "--dry-run" => dry_run = true,
                "--prune" => prune = true,
                "--deploy" => deploy = true,
                "-h" | "--help" => {
                    print_apply_help();
                    std::process::exit(0);
                }
                other => bail!("argumento desconhecido para apply: {other}"),
            }
        }
        Ok(ApplyOpts {
            file: file.ok_or_else(|| anyhow!("faltou -f <arquivo>"))?,
            env_file,
            dry_run,
            prune,
            deploy,
        })
    }
}

struct ExportOpts {
    project: String,
    output: Option<String>,
}

impl ExportOpts {
    fn parse(args: &[String]) -> Result<Self> {
        let mut project = None;
        let mut output = None;
        let mut it = args.iter();
        while let Some(a) = it.next() {
            match a.as_str() {
                "-o" | "--output" => output = it.next().cloned(),
                "-h" | "--help" => {
                    print_export_help();
                    std::process::exit(0);
                }
                other if !other.starts_with('-') => project = Some(other.to_string()),
                other => bail!("argumento desconhecido para export: {other}"),
            }
        }
        Ok(ExportOpts {
            project: project.ok_or_else(|| anyhow!("faltou o nome/id do projeto"))?,
            output,
        })
    }
}

fn print_apply_help() {
    println!(
        "rustploy apply — aplica um manifesto declarativo (Infra-as-Code)\n\n\
         USO:\n  rustploy apply -f <arquivo.yml> [--env-file <.env>] [--prune] [--deploy] [--dry-run]\n\n\
         OPÇÕES:\n  \
         -f, --file <arquivo>   Manifesto (projeto único ou raiz)\n  \
             --env-file <.env>  Arquivo de variáveis para interpolar ${{VAR}}\n  \
             --prune            Remove serviços do projeto ausentes no manifesto\n  \
             --deploy           Dispara deploy dos serviços criados/alterados\n  \
             --dry-run          Imprime o manifesto resolvido sem aplicar"
    );
}

fn print_export_help() {
    println!(
        "rustploy export — exporta um projeto como manifesto YAML\n\n\
         USO:\n  rustploy export <projeto> [-o <arquivo.yml>]\n\n\
         OPÇÕES:\n  \
         -o, --output <arquivo>  Grava no arquivo (padrão: stdout)"
    );
}

// --------------------------------------------------------------------------
// env-backup

pub fn run_env_backup(args: &[String]) -> Result<()> {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("list");
    match sub {
        "list" => cmd_backup_list(),
        "restore" => {
            let snapshot = args
                .get(1)
                .cloned()
                .ok_or_else(|| anyhow!("uso: rustploy env-backup restore <snapshot>"))?;
            cmd_backup_restore(snapshot)
        }
        "--help" | "-h" | "help" => {
            print_env_backup_help();
            Ok(())
        }
        _ => {
            print_env_backup_help();
            bail!("subcomando desconhecido: {sub}")
        }
    }
}

fn cmd_backup_list() -> Result<()> {
    let sock = resolve_socket()?;
    let client = DaemonClient::new(&sock);
    match client.send(Command::EnvBackupList)? {
        Response::EnvBackupSnapshots(names) => {
            if names.is_empty() {
                println!("Nenhum snapshot disponível.");
            } else {
                println!("Snapshots disponíveis ({}):", names.len());
                for name in &names {
                    println!("  {name}");
                }
            }
            Ok(())
        }
        Response::Err { message, .. } => bail!("{message}"),
        _ => bail!("resposta inesperada"),
    }
}

fn cmd_backup_restore(snapshot: String) -> Result<()> {
    let sock = resolve_socket()?;
    let client = DaemonClient::new(&sock);
    match client.send(Command::EnvBackupRestore { snapshot: snapshot.clone() })? {
        Response::Ok => {
            println!("✓ Env vars restauradas a partir de: {snapshot}");
            println!("  Reinicia os serviços afectados para aplicar as novas vars.");
            Ok(())
        }
        Response::Err { message, .. } => bail!("{message}"),
        _ => bail!("resposta inesperada"),
    }
}

fn print_env_backup_help() {
    println!(
        "rustploy env-backup — gerir snapshots de env vars\n\n\
         USO:\n  \
         rustploy env-backup list               Lista snapshots disponíveis\n  \
         rustploy env-backup restore <snapshot> Restaura um snapshot específico\n\n\
         Os snapshots são gravados automaticamente a cada minuto em <db_path>/env_backups/\n\
         e podem ser configurados em /etc/rustploy/config.toml:\n\n  \
         [env_backup]\n  \
         dir = \"/caminho/para/backups\"  # opcional\n  \
         interval_secs = 60             # padrão"
    );
}
