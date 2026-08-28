//! API de agente — um servidor HTTP local que empresta a sessão desta janela.
//!
//! ## O problema que ela resolve
//!
//! O daemon do rustploy já expõe tudo por HTTP (`POST /api/rpc`): dá para
//! conduzir um deploy inteiro por fora da GUI. O que faltava era o **caminho
//! até lá**: quem quer operar um rustploy remoto por agente precisa da URL
//! pública do daemon e do bearer token dele em mãos, na máquina do agente,
//! fora do lugar onde essas credenciais já vivem — que é este app.
//!
//! Aqui a direção se inverte. O app já está logado no daemon remoto (o usuário
//! digitou URL + token na tela de login). Este módulo sobe um servidor HTTP em
//! **loopback** que aceita comandos de um agente rodando na mesma máquina e os
//! encaminha para o daemon remoto usando a sessão da GUI. O agente nunca vê o
//! token do daemon, não precisa saber o endereço do servidor remoto, e o que
//! ele pode alcançar é exatamente o que a janela alcança: trocar de servidor na
//! GUI troca o alvo do agente junto.
//!
//! ```text
//!   agente local ──HTTP──> 127.0.0.1:9800 (este módulo) ──HTTPS──> rustploy remoto
//!                              ▲                                    (POST /api/rpc)
//!                              └── sessão (url + token) lida do contexto da GUI
//! ```
//!
//! ## Como um agente descobre isto
//!
//! Um arquivo de handoff (ver [`handoff`]) é gravado no data dir do usuário com
//! a URL local, o token de acesso e o PID. É o único passo de descoberta: ler o
//! arquivo, e depois `GET /agent/schema` para o catálogo de rotas e comandos.
//!
//! ## Superfície
//!
//! Além do passthrough cru (`POST /agent/rpc`, que aceita qualquer `Command` do
//! protocolo), as rotas de conveniência existem para responder em **uma** ida e
//! volta o que o protocolo cru responde em várias — e, em especial, para
//! resolver a pergunta que motivou tudo: *este deploy funcionou ou falhou, e
//! por quê?* (`POST /agent/deploys` com `wait`, `GET /agent/deploys`). Ver
//! [`catalog`] e `docs/api-agente-no-gui.md`.
//!
//! ## Limites deliberados
//!
//! - **Só loopback.** O bind é sempre 127.0.0.1; não há opção de expor na rede.
//!   Isto é uma ponte para processos da mesma máquina, não um segundo daemon.
//! - **Token mesmo assim.** Loopback não é fronteira de segurança num desktop
//!   multiusuário, e o que está do outro lado da ponte derruba produção. O
//!   arquivo de handoff nasce 0600.
//! - **Sem escopo.** Quem tem o token do handoff tem o mesmo poder que a janela
//!   — que é o poder do bearer do daemon, hoje sem escopo nenhum. Enquanto o
//!   daemon não tiver tokens com escopo (ver a nota de segurança em
//!   `docs/plano-erro-de-deploy-invisivel.md`), esta ponte não tem como
//!   inventar um.

mod actions;
mod catalog;
mod client;
mod handoff;
mod routes;
mod servers;
mod session;
mod ui;

pub(crate) use session::SharedSession;

use std::net::SocketAddr;

use glacier_ui::ExternalSender;
use std::sync::atomic::{AtomicBool, Ordering};

/// Se esta execução chegou a subir o servidor.
///
/// Duas razões, ambas concretas: o gancho `.main()` do `GlacierDaemon` (de onde
/// [`spawn`] é chamado) roda de novo quando a janela principal é REABERTA pela
/// bandeja, e um segundo lançamento do app — que o `single_instance` faz sair
/// sem abrir janela — não pode limpar o handoff da instância que está viva.
static STARTED: AtomicBool = AtomicBool::new(false);

/// Endereço padrão. Porta alta e fixa para o arquivo de handoff ser previsível;
/// se estiver ocupada, o servidor cai para uma porta efêmera e o handoff diz
/// qual foi (por isso o agente lê o arquivo em vez de assumir a porta).
const DEFAULT_ADDR: &str = "127.0.0.1:9800";

/// Variável que desliga a API (`RUSTPLOY_AGENT_API=off`) ou troca o endereço
/// (`RUSTPLOY_AGENT_API=127.0.0.1:9910`).
const ENV_VAR: &str = "RUSTPLOY_AGENT_API";

/// Sobe o servidor numa thread própria, com um runtime tokio próprio.
///
/// Runtime separado de propósito: o loop do iced é dono da thread principal e o
/// executor dele não é um lugar onde dá para `tokio::spawn` antes de `run()`.
/// Uma thread com um runtime `current_thread` custa quase nada e mantém a API
/// viva independente do que a UI esteja fazendo — inclusive com a janela
/// fechada, quando o app fica recolhido na bandeja (o motor headless continua
/// vivo e a sessão junto).
///
/// Não devolve erro: falhar aqui não pode impedir o app de abrir. Qualquer
/// problema vira aviso no stderr e a GUI segue como sempre foi.
pub(crate) fn spawn(session: SharedSession, ui: ExternalSender) {
    // Idempotente: reabrir a janela pela bandeja reexecuta o `.main()` do
    // glacier, e uma segunda thread tentaria bind na mesma porta e reescreveria
    // o handoff com um token novo — invalidando o que o agente já tem em mãos.
    if STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let addr = match configured_addr() {
        Some(addr) => addr,
        None => {
            eprintln!("[agent-api] desligada por {ENV_VAR}=off");
            return;
        }
    };

    std::thread::Builder::new()
        .name("rustploy-agent-api".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("[agent-api] falha ao criar o runtime: {e}");
                    return;
                }
            };
            if let Err(e) = rt.block_on(routes::serve(addr, session, ui)) {
                eprintln!("[agent-api] encerrada: {e}");
            }
        })
        .map(|_| ())
        .unwrap_or_else(|e| eprintln!("[agent-api] falha ao criar a thread: {e}"));
}

/// Apaga o arquivo de handoff. Chamado quando o app encerra: o token é desta
/// execução e não vale mais nada depois dela.
pub(crate) fn cleanup() {
    // Só quem subiu o servidor apaga o handoff. Sem este guard, um segundo
    // lançamento do app (que o `single_instance` encerra em silêncio) apagaria
    // o arquivo da instância que continua no ar, e o agente perderia o caminho
    // de volta sem nada ter acontecido de fato.
    if STARTED.load(Ordering::SeqCst) {
        handoff::remove();
    }
}

/// Endereço a usar, ou `None` quando a API foi desligada por env var.
fn configured_addr() -> Option<SocketAddr> {
    resolve_addr(std::env::var(ENV_VAR).unwrap_or_default().trim())
}

/// Regra de resolução do endereço, separada da leitura da env var para poder
/// ser testada.
///
/// Um valor não-loopback é recusado e cai no default: o desenho todo supõe que
/// só processos da mesma máquina alcançam esta porta, e uma env var num
/// `.desktop` não é lugar de furar isso sem querer.
fn resolve_addr(raw: &str) -> Option<SocketAddr> {
    if raw.eq_ignore_ascii_case("off") || raw == "0" {
        return None;
    }

    let escolhido = if raw.is_empty() { DEFAULT_ADDR } else { raw };

    match escolhido.parse::<SocketAddr>() {
        Ok(addr) if addr.ip().is_loopback() => Some(addr),
        Ok(addr) => {
            eprintln!(
                "[agent-api] {ENV_VAR}={addr} não é loopback — ignorado, usando {DEFAULT_ADDR}"
            );
            DEFAULT_ADDR.parse().ok()
        }
        Err(e) => {
            eprintln!("[agent-api] {ENV_VAR}={escolhido} inválido ({e}) — usando {DEFAULT_ADDR}");
            DEFAULT_ADDR.parse().ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vazio_usa_o_default() {
        assert_eq!(resolve_addr(""), DEFAULT_ADDR.parse().ok());
    }

    #[test]
    fn off_desliga() {
        assert_eq!(resolve_addr("off"), None);
        assert_eq!(resolve_addr("OFF"), None);
        assert_eq!(resolve_addr("0"), None);
    }

    #[test]
    fn porta_alternativa_em_loopback_e_aceita() {
        assert_eq!(
            resolve_addr("127.0.0.1:9910"),
            "127.0.0.1:9910".parse().ok()
        );
        // ::1 também é loopback.
        assert_eq!(resolve_addr("[::1]:9910"), "[::1]:9910".parse().ok());
    }

    /// O guard que importa: pedir bind público não abre a ponte para a rede,
    /// cai no default de loopback.
    #[test]
    fn endereco_publico_cai_no_default() {
        assert_eq!(resolve_addr("0.0.0.0:9800"), DEFAULT_ADDR.parse().ok());
        assert_eq!(resolve_addr("192.168.1.10:9800"), DEFAULT_ADDR.parse().ok());
    }

    #[test]
    fn lixo_cai_no_default() {
        assert_eq!(resolve_addr("nao-e-endereco"), DEFAULT_ADDR.parse().ok());
    }
}
