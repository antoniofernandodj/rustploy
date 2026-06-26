use std::collections::HashMap;
use chrono::Utc;
use ulid::Ulid;
use shared::models::{
    Project, Service, ServiceSpec, ServiceSource, GitSource, ComposeSource,
    EnvVar, EnvVarValue, ResourceLimits, Healthcheck, ServiceStatus,
};
use crate::source::dokploy::DokployData;
use crate::transform::TransformedData;
use crate::warnings::Report;

pub fn transform(data: DokployData, gitea_url: Option<&str>) -> (TransformedData, Report) {
    let mut report = Report::default();
    let mut project_map = HashMap::new(); // Dokploy ID -> Rustploy ID (ULID)
    let mut transformed_projects = Vec::new();

    for dp in data.projects {
        let new_id = Ulid::new().to_string();
        project_map.insert(dp.id.clone(), new_id.clone());

        transformed_projects.push(Project {
            id: new_id,
            name: dp.name,
            description: dp.description,
            env_vars: vec![], // Dokploy doesn't seem to have shared project env vars in the same way
            created_at: Utc::now(),
        });
    }

    let mut transformed_services = Vec::new();

    // Transform Applications
    for da in data.applications {
        let Some(project_id) = project_map.get(&da.project_id) else {
            report.blocking("application", "orphaned", format!("Aplicação '{}' pertence a um projeto inexistente", da.name));
            continue;
        };

        let new_id = Ulid::new().to_string();
        let source = match da.source_type.as_str() {
            "git" | "github" | "gitea" => {
                let url = if da.source_type == "gitea" {
                    let base = gitea_url.unwrap_or("https://gitea.example.com");
                    format!("{}/{}/{}", base, da.gitea_owner.as_deref().unwrap_or(""), da.gitea_repository.as_deref().unwrap_or(""))
                } else if let Some(custom) = da.custom_git_url {
                    custom
                } else {
                    format!("https://github.com/{}/{}", da.owner.as_deref().unwrap_or(""), da.repository.as_deref().unwrap_or(""))
                };

                if da.build_type != "dockerfile" {
                    report.warn("application", "unsupported_build_type", format!("Build type '{}' não suportado nativamente para '{}'", da.build_type, da.name), "Tentando como Dockerfile padrão");
                }

                ServiceSource::Git(GitSource {
                    url,
                    branch: da.custom_git_branch.or(da.branch).unwrap_or_else(|| "main".to_string()),
                    dockerfile_path: da.dockerfile.unwrap_or_else(|| "Dockerfile".to_string()),
                    build_context: normalize_path(da.docker_context_path.as_deref().unwrap_or(".")),
                    build_stage: da.docker_build_stage.filter(|s| !s.is_empty()),
                    ..Default::default()
                })
            }
            _ => {
                report.warn("application", "unsupported_source", format!("Fonte '{}' não suportada para application '{}'", da.source_type, da.name), "Convertido para Git genérico");
                ServiceSource::Git(GitSource::default())
            }
        };

        let env_vars = parse_env(da.env.as_deref());
        
        // Find domain for this application
        let domain_info = data.domains.iter().find(|d| d.application_id.as_deref() == Some(&da.id));
        let (domain, port, tls_enabled) = if let Some(d) = domain_info {
            (Some(d.host.clone()), d.port as u16, d.https)
        } else {
            (None, 80, false)
        };

        transformed_services.push(Service {
            id: new_id,
            spec: ServiceSpec {
                name: da.name,
                project_id: project_id.clone(),
                source,
                port,
                host_port: None,
                domain,
                tls_enabled,
                env_vars,
                volumes: vec![], // TODO: Implement volumes mapping
                healthcheck: Healthcheck::default(),
                replicas: 1,
                resources: ResourceLimits::default(),
                run_command: None,
                run_args: vec![],
                db_kind: None,
            },
            status: ServiceStatus::Stopped,
            live_container_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
    }

    // Transform Composes
    for dc in data.composes {
        let Some(project_id) = project_map.get(&dc.project_id) else {
            report.blocking("compose", "orphaned", format!("Compose '{}' pertence a um projeto inexistente", dc.name));
            continue;
        };

        let new_id = Ulid::new().to_string();
        
        // Clean dokploy-network from compose file
        let cleaned_compose = dc.compose_file.replace("dokploy-network", "rp_net"); // Just a placeholder, daemon handles it

        let env_vars = parse_env(dc.env.as_deref());

        // Find domain for this compose
        let domain_info = data.domains.iter().find(|d| d.compose_id.as_deref() == Some(&dc.id));
        let (domain, port, tls_enabled) = if let Some(d) = domain_info {
            (Some(d.host.clone()), d.port as u16, d.https)
        } else {
            (None, 80, false)
        };

        transformed_services.push(Service {
            id: new_id,
            spec: ServiceSpec {
                name: dc.name,
                project_id: project_id.clone(),
                source: ServiceSource::Compose(ComposeSource { content: cleaned_compose }),
                port,
                host_port: None,
                domain,
                tls_enabled,
                env_vars,
                volumes: vec![],
                healthcheck: Healthcheck::default(),
                replicas: 1,
                resources: ResourceLimits::default(),
                run_command: None,
                run_args: vec![],
                db_kind: None,
            },
            status: ServiceStatus::Stopped,
            live_container_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });
    }

    (
        TransformedData {
            projects: transformed_projects,
            services: transformed_services,
        },
        report,
    )
}

fn parse_env(env_str: Option<&str>) -> Vec<EnvVar> {
    let mut vars = Vec::new();
    if let Some(s) = env_str {
        for line in s.lines() {
            if let Some((key, val)) = line.split_once('=') {
                vars.push(EnvVar {
                    key: key.trim().to_string(),
                    value: EnvVarValue::Plain(val.trim().to_string()),
                });
            }
        }
    }
    vars
}

fn normalize_path(path: &str) -> String {
    let p = path.trim();
    if p == "/" {
        return ".".to_string();
    }
    p.trim_start_matches('/').to_string()
}
