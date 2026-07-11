//! `rustployd-fw` — helper privilegiado de firewall do rustploy.
//!
//! O daemon (`rustployd`) roda sem privilégios (`NoNewPrivileges=yes`) e não
//! pode tocar no ufw. Este binário roda como root, ativado pelo systemd via
//! socket (`rustployd-fw.socket` → `/run/rustploy/fw.sock`, root:rustploy
//! 0660), e tem superfície mínima por construção:
//!
//! - só dois verbos: liberar/bloquear uma porta TCP;
//! - só portas dentro da faixa `[external_ports]` de
//!   `/etc/rustploy/config.toml` (default 20000-20999) — mesmo um daemon
//!   comprometido não abre a porta 22 nem fecha a 443;
//! - nenhuma string do cliente chega a um shell (argv direto, porta é u16).
//!
//! Protocolo: uma linha JSON por requisição, uma de resposta:
//!   → {"op":"allow","port":20001}
//!   ← {"ok":true,"backend":"ufw"}
//!
//! `backend:"none"` = nenhum firewall ativo no host (nada a liberar, sucesso).
//! Para desenvolvimento sem systemd: `RUSTPLOY_FW_SOCKET=/tmp/fw.sock` faz o
//! próprio helper criar o socket.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::Command;

#[derive(Deserialize)]
struct Request {
    op: String,
    port: u16,
}

#[derive(Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    backend: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    error: String,
}

impl Response {
    fn ok(backend: &str) -> Self {
        Self { ok: true, backend: backend.into(), error: String::new() }
    }
    fn err(msg: String) -> Self {
        Self { ok: false, backend: String::new(), error: msg }
    }
}

#[derive(Deserialize)]
struct PortRange {
    #[serde(default = "default_range_start")]
    range_start: u16,
    #[serde(default = "default_range_end")]
    range_end: u16,
}

fn default_range_start() -> u16 {
    20000
}
fn default_range_end() -> u16 {
    20999
}

impl Default for PortRange {
    fn default() -> Self {
        Self { range_start: default_range_start(), range_end: default_range_end() }
    }
}

/// Lê só a seção `[external_ports]` da config do rustploy — o resto do arquivo
/// não interessa ao helper (e manter o parse mínimo evita depender da crate
/// `shared` inteira num binário root).
fn load_range() -> PortRange {
    #[derive(Deserialize)]
    struct PartialConfig {
        #[serde(default)]
        external_ports: PortRange,
    }
    let path = std::env::var("RUSTPLOY_CONFIG")
        .unwrap_or_else(|_| "/etc/rustploy/config.toml".into());
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str::<PartialConfig>(&s).ok())
        .map(|c| c.external_ports)
        .unwrap_or_default()
}

/// Socket herdado do systemd (LISTEN_FDS, fd 3) ou, em dev,
/// criado pelo próprio helper em `$RUSTPLOY_FW_SOCKET`.
fn listener() -> UnixListener {
    if std::env::var("LISTEN_FDS").ok().as_deref() == Some("1") {
        // SAFETY: contrato do sd_listen_fds — com LISTEN_FDS=1 o primeiro (e
        // único) socket herdado é sempre o fd 3, aberto e em modo listen.
        return unsafe { UnixListener::from_raw_fd(3) };
    }
    let path = std::env::var("RUSTPLOY_FW_SOCKET").unwrap_or_else(|e| {
        eprintln!("rustployd-fw: sem LISTEN_FDS do systemd nem RUSTPLOY_FW_SOCKET ({e})");
        std::process::exit(1);
    });
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path).unwrap_or_else(|e| {
        eprintln!("rustployd-fw: falha ao criar socket {path}: {e}");
        std::process::exit(1);
    })
}

fn main() {
    let range = load_range();
    eprintln!(
        "rustployd-fw: atendendo (faixa gerenciada {}-{})",
        range.range_start, range.range_end
    );
    let listener = listener();
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => handle_conn(stream, &range),
            Err(e) => eprintln!("rustployd-fw: accept: {e}"),
        }
    }
}

fn handle_conn(stream: UnixStream, range: &PortRange) {
    let mut writer = match stream.try_clone() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("rustployd-fw: clone do stream: {e}");
            return;
        }
    };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => handle_request(&req, range),
            Err(e) => Response::err(format!("requisição inválida: {e}")),
        };
        let mut out = serde_json::to_string(&resp).unwrap_or_else(|_| "{\"ok\":false}".into());
        out.push('\n');
        if writer.write_all(out.as_bytes()).is_err() {
            return;
        }
    }
}

fn handle_request(req: &Request, range: &PortRange) -> Response {
    if req.port < range.range_start || req.port > range.range_end {
        return Response::err(format!(
            "porta {} fora da faixa gerenciada ({}-{})",
            req.port, range.range_start, range.range_end
        ));
    }
    let allow = match req.op.as_str() {
        "allow" => true,
        "deny" => false,
        other => return Response::err(format!("operação desconhecida: {other}")),
    };

    // TODO: detectar firewalld (`firewall-cmd --state`) como segundo backend.
    match ufw_active() {
        Some(true) => apply_ufw(req.port, allow),
        // ufw inativo ou ausente: sem firewall filtrando INPUT, a porta já é
        // alcançável — nada a fazer.
        Some(false) | None => Response::ok("none"),
    }
}

/// `Some(true)` = ufw instalado e ativo; `Some(false)` = instalado e inativo;
/// `None` = não instalado.
fn ufw_active() -> Option<bool> {
    let out = Command::new("ufw").arg("status").output().ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    Some(stdout.lines().any(|l| l.trim() == "Status: active"))
}

fn apply_ufw(port: u16, allow: bool) -> Response {
    let rule = format!("{port}/tcp");
    let args: Vec<&str> = if allow {
        vec!["allow", &rule, "comment", "rustploy"]
    } else {
        vec!["--force", "delete", "allow", &rule]
    };
    match Command::new("ufw").args(&args).output() {
        Ok(out) if out.status.success() => {
            eprintln!("rustployd-fw: ufw {} {} ok", if allow { "allow" } else { "delete" }, rule);
            Response::ok("ufw")
        }
        Ok(out) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            // Deny de regra que já não existe = objetivo atingido.
            if !allow && combined.contains("Could not delete non-existent rule") {
                return Response::ok("ufw");
            }
            Response::err(format!("ufw falhou: {}", combined.trim()))
        }
        Err(e) => Response::err(format!("falha ao executar ufw: {e}")),
    }
}
