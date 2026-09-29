pub mod config;
pub mod manifest;
pub mod models;
pub mod protocol;
pub mod templates;
pub mod wizard;

pub use config::{ApiConfig, RegistryConfig, RustployConfig, fallback_data_dir, user_home};

/// Unique Docker Compose project name for a rustploy service.
/// Incorporates the first 8 chars of the service ULID to avoid collisions
/// between services with the same user-facing name in different projects.
pub fn compose_project_name(svc_id: &str, svc_name: &str) -> String {
    let id_part = svc_id
        .strip_prefix("svc_")
        .unwrap_or(svc_id)
        .get(..8)
        .unwrap_or(svc_id)
        .to_lowercase();
    let safe: String = svc_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('_');
    format!("rp_{id_part}_{safe}")
}
/// Base do nome de container de um serviço Application: `rp_<id8>_<safe>`.
/// Réplicas e stagings acrescentam sufixos (ver `docker::containers` no
/// daemon).
///
/// Nome de container é único no host inteiro, mas nome de serviço só no
/// projeto; o pedaço do ID impede que o `api` de dois projetos disputem o
/// mesmo container. Ao contrário de [`compose_project_name`], usa os **últimos**
/// 8 caracteres do ULID — a parte aleatória. Os primeiros são timestamp com
/// resolução de ~256 ms, e dois serviços criados juntos (um import de
/// manifesto) os repetiriam.
///
/// O nome curto `rp_<safe>` continua resolvendo dentro do projeto, como alias
/// de rede ([`app_network_alias`]).
pub fn app_container_base(svc_id: &str, svc_name: &str) -> String {
    let ulid = svc_id.strip_prefix("svc_").unwrap_or(svc_id);
    let id_part = ulid[ulid.len().saturating_sub(8)..].to_lowercase();
    format!("rp_{id_part}_{}", normalize_name(svc_name))
}

/// Hostname de um serviço Application **dentro da rede do projeto**:
/// `rp_<safe>`. Foi o nome do container até ele ganhar o ID; hoje é alias de
/// rede, dado a toda réplica live — então é o mesmo nome de sempre para quem
/// o usa em env var (`http://rp_api:8080`), e alias é por rede, não colide
/// entre projetos.
pub fn app_network_alias(svc_name: &str) -> String {
    format!("rp_{}", normalize_name(svc_name))
}

#[cfg(test)]
mod tests_container_names {
    use super::*;

    #[test]
    fn base_usa_a_parte_aleatoria_do_ulid() {
        assert_eq!(
            app_container_base("svc_01JABCDEFGHJKMNPQRSTVWXYZ0", "Minha API"),
            "rp_stvwxyz0_minha_api"
        );
    }

    #[test]
    fn servicos_criados_juntos_nao_colidem() {
        // Mesmo timestamp (10 primeiros chars), aleatório diferente.
        let a = app_container_base("svc_01JABCDEFG0000000000000001", "api");
        let b = app_container_base("svc_01JABCDEFG0000000000000002", "api");
        assert_ne!(a, b);
    }

    #[test]
    fn id_sem_prefixo_de_tipo() {
        assert_eq!(
            app_container_base("01JABCDEFGHJKMNPQRSTVWXYZ0", "api"),
            "rp_stvwxyz0_api"
        );
    }

    #[test]
    fn alias_e_o_nome_curto_de_sempre() {
        assert_eq!(app_network_alias("Minha API"), "rp_minha_api");
    }
}

pub use manifest::{
    ActionVerb, ApplyReport, EnvDoc, GitProviderDoc, ProjectEntry, ProjectEnvDoc, ProjectManifest,
    ResourceAction, ResourceActionKind, ServerManifest, ServiceEnvDoc, ServiceManifest,
    format_dotenv, format_env_doc, parse_dotenv, parse_env_doc,
};
pub use models::*;
pub use protocol::{Command, Event, Response};
pub use wizard::WizardCreateReq;
