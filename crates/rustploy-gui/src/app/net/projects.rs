//! Project-scoped RPCs: the Projects grid, a single project's service list,
//! and project-level environment variables (inherited by every service in the
//! project at deploy time).

use super::view;
use super::{outcome_toast, DetailCacheHandle, RwpClient};
use glacier_ui::{EffectOutcome, ToastSpec};
use shared::{Command, EnvVar, EnvVarValue, Project, Response, Service};
use std::collections::HashMap;

pub struct Projects {
    client: RwpClient,
}

/// Um edit nas variáveis de ambiente de nível de PROJETO (herdadas por todos
/// os serviços no deploy — as do serviço têm precedência). Mesma família de
/// operações do [`super::services::EnvOp`], menos o import de `.env` (o
/// projeto não guarda comentários).
pub enum ProjectEnvOp {
    Set { key: String, value: String },
    Delete { key: String },
    Reorder(Vec<String>),
}

impl Projects {
    pub fn new(client: RwpClient) -> Self {
        Self { client }
    }

    /// [`Self::fetch_services`] plus a write-through into `cache` on success —
    /// the project-detail counterpart of `Services::fetch_detail_cached`.
    pub async fn fetch_services_cached(
        self,
        cache: DetailCacheHandle,
        project_id: String,
    ) -> EffectOutcome {
        let pairs = self.fetch_services(project_id.clone()).await;
        // Only cache a successful load — the error path carries no `proj_name`.
        if pairs.iter().any(|(k, _)| k == "proj_name")
            && let Ok(mut c) = cache.lock()
        {
            c.insert_project(project_id, pairs.clone());
        }
        EffectOutcome::data(pairs)
    }

    /// One-shot load for the `project_services` detail view: the project's own
    /// name/description plus its service cards. Mirrors `Services::fetch_detail`
    /// (fetch-on-open so the view doesn't wait for the next poll tick; the poll
    /// loop then keeps it live via `selected_project_shared`).
    pub async fn fetch_services(self, project_id: String) -> Vec<(String, String)> {
        match self.fetch_services_inner(&project_id).await {
            Ok(pairs) => pairs,
            Err(e) => vec![
                ("proj_loading".into(), "false".into()),
                ("proj_action_msg".into(), format!("erro: {e}")),
            ],
        }
    }

    async fn fetch_services_inner(&self, project_id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let client = &self.client;
        let list = match client.rpc(Command::ProjectList).await? {
            Response::Projects(list) => list,
            other => anyhow::bail!("resposta inesperada para ProjectList: {other:?}"),
        };
        let proj = list
            .iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| anyhow::anyhow!("projeto não encontrado"))?;

        let svcs = match client.rpc(
            Command::ServiceList { project_id: project_id.to_string() },
        ).await? {
            Response::Services(s) => s,
            other => anyhow::bail!("resposta inesperada para ServiceList: {other:?}"),
        };
        let count = svcs.len();
        let all: Vec<(Service, String)> = svcs.into_iter().map(|s| (s, proj.name.clone())).collect();

        Ok(vec![
            ("proj_loading".into(), "false".into()),
            ("proj_action_msg".into(), String::new()),
            ("proj_name".into(), proj.name.clone()),
            ("proj_description".into(), proj.description.clone().unwrap_or_default()),
            ("proj_can_delete".into(), if count == 0 { "1" } else { "0" }.into()),
            ("project_services".into(), view::service_rows_json(&all, &HashMap::new(), "")),
            // Nomes dos serviços do projeto — usados pelo wizard para pré-checar
            // nome duplicado antes de mandar o ServiceCreate (o backend é a fonte
            // autoritativa; isto é só feedback instantâneo).
            ("proj_service_names".into(), serde_json::Value::Array(
                all.iter().map(|(s, _)| serde_json::Value::String(s.spec.name.clone())).collect()
            ).to_string()),
            // Aba "Variáveis" do projeto (herdadas por todos os serviços no deploy).
            ("proj_env".into(), view::env_json(&proj.env_vars)),
            ("proj_env_count".into(), proj.env_vars.len().to_string()),
        ])
    }

    /// Creates a project (Projects tab "Novo projeto" bar) and immediately
    /// re-fetches the Projects grid on the same connection so the new card
    /// shows up at once, instead of waiting up to 2s for the next poll tick.
    /// `search` keeps the grid filtered by whatever the topbar search box
    /// currently holds.
    pub async fn create(self, name: String, description: String, search: String) -> EffectOutcome {
        let description = if description.trim().is_empty() { None } else { Some(description) };
        let msg = match self.client.rpc(Command::ProjectCreate { name, description }).await {
            Ok(Response::Project(p)) => format!("projeto \"{}\" criado", p.name),
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = vec![
            ("new_proj_msg".into(), msg.clone()),
            ("new_proj_name".into(), String::new()),
            ("new_proj_desc".into(), String::new()),
        ];
        pairs.extend(self.grid_pairs(&search).await);
        outcome_toast(pairs, &msg)
    }

    /// Renames/redescribes the project open in `project_services`.
    pub async fn update(self, id: String, name: String, description: String) -> EffectOutcome {
        let description = if description.trim().is_empty() { None } else { Some(description) };
        match self.client.rpc(Command::ProjectUpdate { id, name, description }).await {
            Ok(Response::Project(p)) => EffectOutcome::data(vec![
                ("proj_action_msg".into(), "salvo".into()),
                ("proj_name".into(), p.name),
                ("proj_description".into(), p.description.unwrap_or_default()),
            ])
            .with_toast(ToastSpec::success("Projeto salvo.")),
            Ok(other) => {
                let msg = view::resp_msg(&other);
                EffectOutcome::data(vec![("proj_action_msg".into(), msg.clone())]).with_toast(ToastSpec::error(msg))
            }
            Err(e) => {
                let msg = format!("erro: {e}");
                EffectOutcome::data(vec![("proj_action_msg".into(), msg.clone())]).with_toast(ToastSpec::error(msg))
            }
        }
    }

    /// Deletes a project — the daemon refuses (with a user-facing message) if
    /// it still has services, so no client-side re-validation is needed here.
    /// Re-fetches the Projects grid on the same connection so the removed card
    /// disappears at once instead of after the next 2s poll tick.
    pub async fn delete(self, id: String, search: String) -> EffectOutcome {
        let (msg, ok) = match self.client.rpc(Command::ProjectDelete { id }).await {
            Ok(Response::Ok) => ("projeto removido".to_string(), true),
            Ok(other) => (view::resp_msg(&other), false),
            Err(e) => (format!("erro: {e}"), false),
        };
        let mut pairs = vec![("proj_action_msg".into(), msg.clone())];
        // Se a remoção veio de DENTRO do projeto (view=project_services), ele não
        // existe mais — volta para a grade de projetos. Deletar pela grade já está
        // em view=projects, então é no-op nesse caso.
        if ok {
            pairs.push(("view".into(), "projects".into()));
        }
        pairs.extend(self.grid_pairs(&search).await);
        outcome_toast(pairs, &msg)
    }

    /// Aplica um [`ProjectEnvOp`] (fetch do projeto → mutação →
    /// `ProjectEnvSet`) e devolve a lista atualizada para a aba "Variáveis" do
    /// projeto.
    pub async fn run_env_op(self, project_id: String, op: ProjectEnvOp) -> EffectOutcome {
        match self.apply_env_op(&project_id, op).await {
            Ok(project) => EffectOutcome::data(vec![
                ("proj_action_msg".into(), "variáveis do projeto atualizadas".into()),
                ("proj_env".into(), view::env_json(&project.env_vars)),
                ("proj_env_count".into(), project.env_vars.len().to_string()),
            ])
            .with_toast(ToastSpec::success("Variáveis do projeto atualizadas.")),
            Err(e) => {
                let msg = format!("erro: {e}");
                EffectOutcome::data(vec![("proj_action_msg".into(), msg.clone())]).with_toast(ToastSpec::error(msg))
            }
        }
    }

    async fn apply_env_op(&self, project_id: &str, op: ProjectEnvOp) -> anyhow::Result<Project> {
        let client = &self.client;
        let list = match client.rpc(Command::ProjectList).await? {
            Response::Projects(list) => list,
            other => anyhow::bail!("resposta inesperada para ProjectList: {other:?}"),
        };
        let proj = list
            .into_iter()
            .find(|p| p.id == project_id)
            .ok_or_else(|| anyhow::anyhow!("projeto não encontrado"))?;
        let mut env_vars = proj.env_vars;
        match op {
            ProjectEnvOp::Set { key, value } => {
                env_vars.retain(|v| v.key != key);
                env_vars.push(EnvVar { key, value: EnvVarValue::Plain(value) });
            }
            ProjectEnvOp::Delete { key } => {
                env_vars.retain(|v| v.key != key);
            }
            ProjectEnvOp::Reorder(keys) => {
                let mut by_key: HashMap<String, EnvVar> =
                    env_vars.drain(..).map(|v| (v.key.clone(), v)).collect();
                let mut reordered: Vec<EnvVar> = keys.iter().filter_map(|k| by_key.remove(k)).collect();
                reordered.extend(by_key.into_values());
                env_vars = reordered;
            }
        }
        match client.rpc(
            Command::ProjectEnvSet { project_id: project_id.into(), env_vars },
        )
        .await?
        {
            Response::Project(p) => Ok(p),
            Response::Err { code, message } => anyhow::bail!("{code}: {message}"),
            other => anyhow::bail!("resposta inesperada para ProjectEnvSet: {other:?}"),
        }
    }

    /// Re-fetches the project/service inventory and returns the Projects-grid
    /// context keys (`projects`/`project_rows` + counts), filtered by
    /// `search`. Lets project create/delete patch the grid immediately instead
    /// of waiting for the next 2s poll tick.
    async fn grid_pairs(&self, search: &str) -> Vec<(String, String)> {
        let client = &self.client;
        let mut projects_raw: Vec<Project> = Vec::new();
        let mut all: Vec<(Service, String)> = Vec::new();
        if let Ok(Response::Projects(list)) = client.rpc(Command::ProjectList).await {
            for p in &list {
                if let Ok(Response::Services(svcs)) =
                    client.rpc(Command::ServiceList { project_id: p.id.clone() }).await
                {
                    for s in svcs {
                        all.push((s, p.name.clone()));
                    }
                }
            }
            projects_raw = list;
        }
        let term = search.trim().to_lowercase();
        vec![
            ("projects_count".into(), projects_raw.len().to_string()),
            ("services_count".into(), all.len().to_string()),
            ("projects".into(), view::projects_json(&projects_raw, &all, &term)),
            ("project_rows".into(), view::project_rows_json(&projects_raw, &all, &term)),
        ]
    }

    /// Runs a lifecycle command against an arbitrary service id from the
    /// `project_services` grid (as opposed to `Services::run_action`, which
    /// acts on the single service open in the detail view). Re-fetches the
    /// project's service grid afterwards so the card's status flips
    /// immediately instead of after the next 2s poll tick.
    pub async fn run_service_action(self, cmd: Command, project_id: String) -> EffectOutcome {
        let msg = match self.client.rpc(cmd).await {
            Ok(Response::Ok) => "ação concluída".to_string(),
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = self.fetch_services(project_id).await;
        pairs.push(("proj_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }

    /// "Parar e remover": stops the service, then deletes it once stopped.
    /// Bails out (without deleting) if the stop itself fails, so a service
    /// that refused to stop isn't silently removed from the DB while its
    /// container lingers.
    pub async fn stop_and_delete_service(self, service_id: String, project_id: String) -> EffectOutcome {
        // On a stop failure the service isn't deleted; still re-fetch the grid so the
        // card reflects reality (it may have stopped despite the error response).
        let msg = match self.client.rpc(Command::ServiceStop { service_id: service_id.clone() }).await {
            Ok(Response::Ok) => match self.client.rpc(Command::ServiceDelete { id: service_id }).await {
                Ok(Response::Ok) => "serviço parado e removido".to_string(),
                Ok(other) => view::resp_msg(&other),
                Err(e) => format!("erro ao remover: {e}"),
            },
            Ok(other) => format!("erro ao parar: {}", view::resp_msg(&other)),
            Err(e) => format!("erro ao parar: {e}"),
        };
        let mut pairs = self.fetch_services(project_id).await;
        pairs.push(("proj_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }
}
