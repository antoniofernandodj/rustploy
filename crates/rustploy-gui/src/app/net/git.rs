//! Connected Git providers (Gitea): Settings → Git management, plus the
//! repo/branch pickers used by a service's General tab.

use super::view;
use super::{outcome_toast, RwpClient};
use glacier_ui::EffectOutcome;
use shared::{Command, Response};

pub struct GitProviders {
    client: RwpClient,
}

impl GitProviders {
    pub fn new(client: RwpClient) -> Self {
        Self { client }
    }

    // TODO(reusar): carregamento isolado da lista de providers para a aba General.
    // Ficou órfão quando o load dos providers foi dobrado para dentro de
    // `Services::fetch_detail` (para `svc_loading` gatear os Selects). Mantido
    // para um futuro caminho que precise só dos providers sem o detalhe
    // completo do serviço.
    /// Lists the connected Git providers for the General-tab picker.
    #[allow(dead_code)]
    pub async fn list(self) -> Vec<(String, String)> {
        match self.client.rpc(Command::GitProviderList).await {
            Ok(Response::GitProviders(list)) => {
                let msg = if list.is_empty() {
                    "nenhum provider conectado — configure em Settings".to_string()
                } else {
                    String::new()
                };
                vec![
                    ("gitea_providers".into(), view::git_providers_json(&list)),
                    ("gitea_count".into(), list.len().to_string()),
                    ("gitea_msg".into(), msg),
                ]
            }
            Ok(other) => vec![("gitea_msg".into(), view::resp_msg(&other))],
            Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
        }
    }

    /// Lists the repositories of a provider; resets the repo/branch selection.
    pub async fn fetch_repos(self, provider_id: String) -> EffectOutcome {
        let pid = provider_id.clone();
        EffectOutcome::data(match self.client.rpc(Command::GitRepoList { provider_id }).await {
            Ok(Response::GitRepos(list)) => vec![
                ("gitea_provider_id".into(), pid),
                ("gitea_repos".into(), view::git_repos_json(&list)),
                ("gitea_branches".into(), "[]".into()),
                ("gitea_repo".into(), String::new()),
                ("gitea_msg".into(), format!("{} repositório(s)", list.len())),
            ],
            Ok(other) => vec![("gitea_msg".into(), view::resp_msg(&other))],
            Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
        })
    }

    /// Lists the branches of a repository for the branch picker.
    pub async fn fetch_branches(self, provider_id: String, repo_full_name: String) -> EffectOutcome {
        EffectOutcome::data(match self.client.rpc(Command::GitBranchList { provider_id, repo_full_name }).await {
            Ok(Response::GitBranches(list)) => vec![
                ("gitea_branches".into(), view::git_branches_json(&list)),
                ("gitea_msg".into(), format!("{} branch(es)", list.len())),
            ],
            Ok(other) => vec![("gitea_msg".into(), view::resp_msg(&other))],
            Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
        })
    }

    /// Re-fetches the provider list and returns the context pairs (`gitea_*`)
    /// plus `gp_msg`. Shared by connect/delete/refresh so the list stays in
    /// one place.
    async fn refresh_pairs(&self, msg: String) -> Vec<(String, String)> {
        let mut pairs = vec![("gp_msg".into(), msg)];
        if let Ok(Response::GitProviders(list)) = self.client.rpc(Command::GitProviderList).await {
            pairs.push(("gitea_providers".into(), view::git_providers_json(&list)));
            pairs.push(("gitea_count".into(), list.len().to_string()));
        }
        pairs
    }

    /// Registers a new Gitea provider (Settings → Git). On OAuth it then
    /// starts the authorization flow and opens the browser; on PAT the
    /// account is usable at once. Clears the form fields and refreshes the
    /// connected list.
    #[allow(clippy::too_many_arguments)]
    pub async fn connect(
        self,
        name: String,
        base_url: String,
        mode: String,
        client_id: String,
        client_secret: String,
        pat: String,
    ) -> EffectOutcome {
        if base_url.trim().is_empty() {
            return EffectOutcome::data(vec![("gp_msg".into(), "informe a Base URL do Gitea".into())]);
        }
        let name = if name.trim().is_empty() { "Gitea".to_string() } else { name.trim().to_string() };
        let is_oauth = mode != "pat";

        let cmd = if is_oauth {
            if client_id.trim().is_empty() || client_secret.trim().is_empty() {
                return EffectOutcome::data(vec![("gp_msg".into(), "Client ID e Client Secret são obrigatórios".into())]);
            }
            Command::GitProviderCreate {
                kind: shared::GitProviderKind::Gitea,
                name,
                base_url: base_url.trim().to_string(),
                auth_mode: shared::GitAuthMode::OAuth,
                oauth_client_id: Some(client_id.trim().to_string()),
                oauth_client_secret: Some(client_secret.clone()),
                pat: None,
            }
        } else {
            if pat.trim().is_empty() {
                return EffectOutcome::data(vec![("gp_msg".into(), "informe o Personal Access Token".into())]);
            }
            Command::GitProviderCreate {
                kind: shared::GitProviderKind::Gitea,
                name,
                base_url: base_url.trim().to_string(),
                auth_mode: shared::GitAuthMode::Pat,
                oauth_client_id: None,
                oauth_client_secret: None,
                pat: Some(pat.clone()),
            }
        };

        let provider_id = match self.client.rpc(cmd).await {
            Ok(Response::GitProviderInfo(p)) => p.id,
            Ok(other) => return EffectOutcome::data(vec![("gp_msg".into(), view::resp_msg(&other))]),
            Err(e) => return EffectOutcome::data(vec![("gp_msg".into(), format!("erro: {e}"))]),
        };

        // OAuth needs a browser round-trip; PAT is immediately usable.
        let msg = if is_oauth {
            match self.client.rpc(Command::GitOAuthStart { provider_id }).await {
                Ok(Response::OAuthUrl(url)) => {
                    if open_in_browser(&url) {
                        "navegador aberto — autorize e clique em Atualizar lista".to_string()
                    } else {
                        format!("abra para autorizar: {url}")
                    }
                }
                Ok(other) => view::resp_msg(&other),
                Err(e) => format!("erro: {e}"),
            }
        } else {
            "conta Gitea conectada ✓".to_string()
        };

        let mut pairs = self.refresh_pairs(msg.clone()).await;
        // Clear the connect form.
        for k in ["gp_name", "gp_base_url", "gp_client_id", "gp_client_secret", "gp_pat"] {
            pairs.push((k.into(), String::new()));
        }
        outcome_toast(pairs, &msg)
    }

    /// Removes a connected provider and refreshes the list.
    pub async fn delete(self, id: String) -> EffectOutcome {
        let msg = match self.client.rpc(Command::GitProviderDelete { id }).await {
            Ok(Response::Ok) => "provider removido".to_string(),
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let pairs = self.refresh_pairs(msg.clone()).await;
        outcome_toast(pairs, &msg)
    }

    /// One-shot provider list refresh for the "Atualizar lista" button.
    pub async fn refresh(self) -> EffectOutcome {
        EffectOutcome::data(self.refresh_pairs(String::new()).await)
    }
}

/// The Gitea OAuth callback URI the user must register in their Gitea app:
/// `{domain}/oauth/gitea/callback` (matches the daemon's webhook server).
/// Empty domain yields a hint placeholder.
pub fn oauth_redirect_uri(domain: &str) -> String {
    let d = domain.trim().trim_end_matches('/');
    if d.is_empty() {
        "<configure o domínio em Web Server>/oauth/gitea/callback".to_string()
    } else {
        format!("{d}/oauth/gitea/callback")
    }
}

/// Best-effort: opens `url` in the user's default browser (`xdg-open`).
fn open_in_browser(url: &str) -> bool {
    std::process::Command::new("xdg-open")
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}
