//! A sessão da GUI (URL + token do daemon remoto), compartilhada com o servidor
//! da API de agente.
//!
//! Quem escreve é a thread da UI, pelo gancho `on_message` do `GlacierDaemon`:
//! toda mensagem despachada na janela principal passa por lá com o motor no
//! estado resultante, e o contexto do motor é onde a camada Luau guarda
//! `api_url`/`api_token`/`connected` ao conectar (`handlers/connection.luau`).
//! Quem lê é a thread do servidor, a cada requisição.
//!
//! Não há um "evento de login" no glacier para assinar, e nem faz falta: o
//! `on_message` roda depois de cada dispatch, então observar o contexto ali é
//! equivalente — e cobre de graça o logout (que apaga o contexto inteiro) e a
//! troca de servidor sem que este módulo precise conhecer nenhum dos dois
//! fluxos.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Conexão viva com um daemon rustploy, do ponto de vista da API de agente.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Session {
    /// Base sem barra final, ex.: `https://rustploy.exemplo.com`.
    pub base_url: String,
    /// Bearer do daemon. `None` quando o daemon roda sem token.
    pub token: Option<String>,
}

/// O que a thread da API enxerga da janela: a sessão (quando conectada) e um
/// espelho do contexto do motor.
#[derive(Default)]
struct Espelho {
    session: Option<Session>,
    /// Cópia do contexto do motor, refeita a cada dispatch da janela principal.
    ///
    /// Existe porque o contexto só é legível de dentro do gancho `on_message`,
    /// na thread do iced — e a thread da API precisa dele para responder "em
    /// que tela a GUI está?", "qual serviço está selecionado?", "qual foi o
    /// erro do último login?". Copiar em vez de referenciar é o que permite às
    /// duas threads seguirem sem trava compartilhada.
    ///
    /// O custo é um clone do mapa por dispatch (~120 chaves, algumas com JSON
    /// de dezenas de KB), a cada 2s no ritmo do snapshot do SSE. Numa aplicação
    /// de desktop isso é ruído; a alternativa (comparar campo a campo para só
    /// copiar o que mudou) custaria a mesma ordem de trabalho.
    context: HashMap<String, String>,
}

/// Handle compartilhado. Sessão `None` = a janela não está conectada a daemon
/// nenhum (tela de login, ou logout) — e aí a API de agente responde 503
/// dizendo exatamente isso, em vez de falhar de um jeito que pareça bug de rede.
#[derive(Clone, Default)]
pub(crate) struct SharedSession(Arc<RwLock<Espelho>>);

impl SharedSession {
    pub(crate) fn get(&self) -> Option<Session> {
        self.0.read().ok().and_then(|g| g.session.clone())
    }

    /// Uma chave do contexto da janela, como o motor a tem agora.
    pub(crate) fn context_key(&self, key: &str) -> Option<String> {
        self.0.read().ok().and_then(|g| g.context.get(key).cloned())
    }

    /// Cópia do contexto inteiro.
    pub(crate) fn context(&self) -> HashMap<String, String> {
        self.0.read().map(|g| g.context.clone()).unwrap_or_default()
    }

    /// Relê o contexto do motor, espelha-o e atualiza a sessão se algo mudou.
    ///
    /// Devolve `true` quando a SESSÃO mudou — o chamador usa isso para regravar
    /// o arquivo de handoff sem escrever em disco a cada tick de snapshot do
    /// SSE (que dispara um dispatch a cada 2s, e portanto uma chamada aqui). O
    /// espelho do contexto é atualizado sempre, mudando ou não a sessão.
    pub(crate) fn sync_from_context(&self, ctx: &HashMap<String, String>) -> bool {
        let nova = Session::from_context(ctx);

        match self.0.write() {
            Ok(mut e) => {
                let mudou = e.session != nova;
                e.session = nova;
                e.context = ctx.clone();
                mudou
            }
            Err(_) => false,
        }
    }
}

impl Session {
    /// Extrai a sessão do contexto do motor, ou `None` se não houver conexão.
    ///
    /// `connected` é o que a camada Luau usa para dizer que o `DaemonStatus` de
    /// validação passou (`handlers/connection.luau::connect`); sem esse gate a
    /// API de agente aceitaria requisições enquanto a tela de login ainda
    /// mostra credenciais que o usuário está digitando.
    fn from_context(ctx: &HashMap<String, String>) -> Option<Self> {
        if ctx.get("connected").map(String::as_str) != Some("true") {
            return None;
        }

        let base_url = ctx.get("api_url")?.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return None;
        }

        let token = ctx
            .get("api_token")
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(str::to_owned);

        Some(Self { base_url, token })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(pares: &[(&str, &str)]) -> HashMap<String, String> {
        pares
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn sem_conexao_nao_ha_sessao() {
        // Tela de login: o usuário já digitou a URL, mas o Connect ainda não
        // validou nada. A API de agente não pode usar isso.
        let s = SharedSession::default();
        s.sync_from_context(&ctx(&[("api_url", "https://x.dev"), ("api_token", "t")]));
        assert!(s.get().is_none());
    }

    #[test]
    fn conexao_valida_vira_sessao_com_a_barra_final_aparada() {
        let s = SharedSession::default();
        assert!(s.sync_from_context(&ctx(&[
            ("connected", "true"),
            ("api_url", "https://rustploy.exemplo.com/"),
            ("api_token", "  segredo  "),
        ])));
        let sess = s.get().unwrap();
        assert_eq!(sess.base_url, "https://rustploy.exemplo.com");
        assert_eq!(sess.token.as_deref(), Some("segredo"));
    }

    #[test]
    fn daemon_sem_token_vira_sessao_sem_token() {
        let s = SharedSession::default();
        s.sync_from_context(&ctx(&[
            ("connected", "true"),
            ("api_url", "http://127.0.0.1:9797"),
            ("api_token", ""),
        ]));
        assert_eq!(s.get().unwrap().token, None);
    }

    /// O tick de snapshot do SSE dispara um dispatch a cada 2s: repetir o mesmo
    /// contexto não pode contar como mudança (senão o handoff seria reescrito
    /// em disco o tempo todo).
    #[test]
    fn contexto_repetido_nao_conta_como_mudanca() {
        let s = SharedSession::default();
        let c = ctx(&[
            ("connected", "true"),
            ("api_url", "https://x.dev"),
            ("api_token", "t"),
        ]);
        assert!(s.sync_from_context(&c));
        assert!(!s.sync_from_context(&c));
    }

    /// Logout apaga o contexto inteiro; a sessão tem que cair junto — é o que
    /// impede o agente de continuar operando um servidor do qual o usuário
    /// acabou de sair.
    #[test]
    fn logout_derruba_a_sessao() {
        let s = SharedSession::default();
        s.sync_from_context(&ctx(&[
            ("connected", "true"),
            ("api_url", "https://x.dev"),
            ("api_token", "t"),
        ]));
        assert!(s.get().is_some());

        assert!(s.sync_from_context(&ctx(&[("connected", "false")])));
        assert!(s.get().is_none());
    }

    /// Trocar de servidor na GUI troca o alvo do agente — sem o agente saber.
    #[test]
    fn trocar_de_servidor_troca_o_alvo() {
        let s = SharedSession::default();
        s.sync_from_context(&ctx(&[
            ("connected", "true"),
            ("api_url", "https://a.dev"),
            ("api_token", "ta"),
        ]));
        assert!(s.sync_from_context(&ctx(&[
            ("connected", "true"),
            ("api_url", "https://b.dev"),
            ("api_token", "tb"),
        ])));
        assert_eq!(s.get().unwrap().base_url, "https://b.dev");
    }
}
