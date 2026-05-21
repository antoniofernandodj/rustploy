pub mod stream;

use crate::{
    db::Db,
    docker::DockerClient,
    event_bus::EventBus,
    ingress::IngressController,
    secrets::SecretsManager,
};
use axum::{
    body::Bytes,
    extract::FromRequest,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{de::DeserializeOwned, Serialize};
use shared::{Command, Response as RpResponse};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub db_path: PathBuf,
    pub drain_secs: u64,
    pub started_at: std::time::Instant,
}

pub struct Bincode<T>(pub T);

impl<T: Serialize> IntoResponse for Bincode<T> {
    fn into_response(self) -> Response {
        match bincode::serialize(&self.0) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[axum::async_trait]
impl<S, T> FromRequest<S> for Bincode<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let val = bincode::deserialize(&bytes)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("bincode: {e}")))?;
        Ok(Bincode(val))
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/rpc", post(rpc_handler))
        .route("/stream", get(stream::handler))
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn rpc_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    let response = dispatch(state, cmd).await;
    Bincode(response)
}

async fn dispatch(state: AppState, cmd: Command) -> RpResponse {
    match cmd {
        Command::Ping => RpResponse::Pong {
            uptime_secs: state.started_at.elapsed().as_secs(),
        },

        Command::DaemonStatus => {
            let services = crate::db::services::get_running(&state.db).await.unwrap_or_default();
            let _total: Vec<_> = state.db
                .query("SELECT count() FROM service GROUP ALL")
                .await
                .ok()
                .and_then(|mut r| r.take::<Vec<serde_json::Value>>(0).ok())
                .unwrap_or_default();
            RpResponse::DaemonStatus(shared::DaemonStatus {
                version: env!("CARGO_PKG_VERSION").into(),
                uptime_secs: state.started_at.elapsed().as_secs(),
                services_running: services.len(),
                services_total: 0,
            })
        }

        Command::ProjectCreate { name, description } => {
            match crate::db::projects::create(&state.db, name, description).await {
                Ok(p) => RpResponse::Project(p),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ProjectList => {
            match crate::db::projects::list(&state.db).await {
                Ok(ps) => RpResponse::Projects(ps),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ProjectDelete { id } => {
            match crate::db::projects::delete(&state.db, &id).await {
                Ok(true) => RpResponse::Ok,
                Ok(false) => RpResponse::err("NotFound", "project not found"),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ServiceCreate(spec) => {
            match crate::db::services::create(&state.db, spec).await {
                Ok(s) => RpResponse::Service(s),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ServiceList { project_id } => {
            match crate::db::services::list(&state.db, &project_id).await {
                Ok(ss) => RpResponse::Services(ss),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ServiceGet { id } => {
            match crate::db::services::get(&state.db, &id).await {
                Ok(Some(s)) => RpResponse::Service(s),
                Ok(None) => RpResponse::err("NotFound", "service not found"),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ServiceUpdate { id, spec } => {
            match crate::db::services::update_spec(&state.db, &id, spec).await {
                Ok(Some(s)) => RpResponse::Service(s),
                Ok(None) => RpResponse::err("NotFound", "service not found"),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::ServiceDelete { id } => {
            if let Ok(Some(svc)) = crate::db::services::get(&state.db, &id).await {
                state.ingress.remove_route(&svc.spec.domain);
            }
            match crate::db::services::delete(&state.db, &id).await {
                Ok(true) => RpResponse::Ok,
                Ok(false) => RpResponse::err("NotFound", "service not found"),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::DeployStart { service_id } => {
            use crate::deploy::executor::DeployExecutor;
            use shared::ServiceStatus;

            let svc = match crate::db::services::get(&state.db, &service_id).await {
                Ok(Some(s)) => s,
                Ok(None) => return RpResponse::err("NotFound", "service not found"),
                Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
            };

            if matches!(svc.status, ServiceStatus::Deploying) {
                return RpResponse::err("ServiceAlreadyDeploying", "deploy already in progress");
            }

            let image = match &svc.spec.source {
                shared::ServiceSource::Registry { image } => image.clone(),
                shared::ServiceSource::Git(_) => format!("rp_{}", svc.spec.name),
            };

            let dep = match crate::db::deployments::create(&state.db, &service_id, &image).await {
                Ok(d) => d,
                Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
            };

            let _ = crate::db::services::update_status(
                &state.db, &service_id, &ServiceStatus::Deploying, None,
            ).await;

            state.bus.publish(shared::Event::ServiceStatusChanged {
                service_id: service_id.clone(),
                status: ServiceStatus::Deploying,
            });

            let executor = Arc::new(DeployExecutor {
                db: state.db.clone(),
                docker: state.docker.clone(),
                ingress: state.ingress.clone(),
                bus: state.bus.clone(),
                secrets: state.secrets.clone(),
                db_path: state.db_path.clone(),
                drain_secs: state.drain_secs,
            });
            let dep_id = dep.id.clone();
            tokio::spawn(async move { executor.run(dep_id).await });

            RpResponse::Deployment(dep)
        }

        Command::DeployAbort { deployment_id } => {
            let dep = match crate::db::deployments::get(&state.db, &deployment_id).await {
                Ok(Some(d)) => d,
                Ok(None) => return RpResponse::err("NotFound", "deployment not found"),
                Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
            };
            if dep.state.is_terminal() {
                return RpResponse::err("InvalidState", "deployment already finished");
            }
            match crate::db::deployments::transition(
                &state.db,
                &deployment_id,
                &dep.state,
                shared::DeployState::RollingBack,
                Some("aborted by user".into()),
            ).await {
                Ok(d) => RpResponse::Deployment(d),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::DeployHistory { service_id, limit } => {
            match crate::db::deployments::list_for_service(&state.db, &service_id, limit).await {
                Ok(deps) => RpResponse::Deployments(deps),
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        Command::DeployRollback { service_id } => {
            match crate::db::deployments::list_for_service(&state.db, &service_id, 10).await {
                Ok(history) => {
                    let prev = history.iter().skip(1).find(|d| d.state == shared::DeployState::Live);
                    match prev {
                        Some(d) => RpResponse::Deployment(d.clone()),
                        None => RpResponse::err("NotFound", "no previous successful deploy"),
                    }
                }
                Err(e) => RpResponse::err("DatabaseError", e.to_string()),
            }
        }

        _ => RpResponse::err("NotImplemented", "command not yet implemented"),
    }
}
