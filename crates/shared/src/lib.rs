//! Tipos compartilhados entre daemon e GUI (modelos, protocolo, config,
//! manifest, templates) e os nomes Docker derivados de um serviço.

pub mod config;
pub mod connection;
pub mod manifest;
pub mod models;
pub mod protocol;
pub mod templates;
pub mod wizard;

pub use config::{ApiConfig, RegistryConfig, RustployConfig, fallback_data_dir, user_home};

/// Nome de stack Compose no formato **legado**, derivado a cada uso: primeiros
/// 8 caracteres do ID (timestamp — não é único) + nome do serviço (muda no
/// rename). Só a migração de preenchimento e o fallback de
/// [`Service::compose_project_name`] o usam; serviço novo recebe o nome gravado
/// por [`new_compose_project_name`]. Ver `docs/plano-nome-gravado-rede-e-stack.md`.
pub fn compose_project_name(svc_id: &str, svc_name: &str) -> String {
    let id_part = svc_id
        .strip_prefix("svc_")
        .unwrap_or(svc_id)
        .get(..8)
        .unwrap_or(svc_id)
        .to_lowercase();
    format!("rp_{id_part}_{}", compose_safe(svc_name))
}

/// Nome de stack Compose de um serviço **novo**: `rp_<últimos 8 chars do ID>_<nome>`
/// (a parte aleatória do ULID, como em [`app_container_base`]), com o nome que o
/// serviço tem na criação. Vira `com.docker.compose.project` e prefixo dos
/// volumes; é gravado em `service.compose_project` e nunca mais recalculado.
pub fn new_compose_project_name(svc_id: &str, svc_name: &str) -> String {
    let ulid = svc_id.strip_prefix("svc_").unwrap_or(svc_id);
    let id_part = ulid[ulid.len().saturating_sub(8)..].to_lowercase();
    format!("rp_{id_part}_{}", compose_safe(svc_name))
}

/// Variante com o ID inteiro, para quando o índice `UNIQUE` recusa a curta.
pub fn new_compose_project_name_long(svc_id: &str, svc_name: &str) -> String {
    let ulid = svc_id.strip_prefix("svc_").unwrap_or(svc_id).to_lowercase();
    format!("rp_{ulid}_{}", compose_safe(svc_name))
}

/// Nome de rede Docker de um projeto **novo**: `rp_net_<ID inteiro, minúsculo>`.
/// O ID inteiro não colide (os 8 primeiros chars de um ULID são o horário).
pub fn new_project_network_name(project_id: &str) -> String {
    let ulid = project_id.strip_prefix("prj_").unwrap_or(project_id);
    format!("rp_net_{}", ulid.to_lowercase())
}

/// Nome de serviço reduzido ao que o Docker Compose aceita num nome de projeto
/// (ASCII minúsculo, dígitos e `_`).
fn compose_safe(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    safe.trim_matches('_').to_string()
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

    #[test]
    fn stack_legada_e_a_de_sempre() {
        assert_eq!(
            compose_project_name("svc_01JABCDEFGHJKMNPQRSTVWXYZ0", "Meu DB"),
            "rp_01jabcde_meu_db"
        );
    }

    #[test]
    fn stack_nova_usa_a_parte_aleatoria_e_o_nome_da_criacao() {
        assert_eq!(
            new_compose_project_name("svc_01JABCDEFGHJKMNPQRSTVWXYZ0", "Meu DB"),
            "rp_stvwxyz0_meu_db"
        );
        assert_eq!(
            new_compose_project_name_long("svc_01JABCDEFGHJKMNPQRSTVWXYZ0", "db"),
            "rp_01jabcdefghjkmnpqrstvwxyz0_db"
        );
        let a = new_compose_project_name("svc_01JABCDEFG0000000000000001", "db");
        let b = new_compose_project_name("svc_01JABCDEFG0000000000000002", "db");
        assert_ne!(a, b);
    }

    #[test]
    fn rede_nova_leva_o_id_inteiro() {
        assert_eq!(
            new_project_network_name("prj_01M3NV213V06RX7CXJ5FDDAP0K"),
            "rp_net_01m3nv213v06rx7cxj5fddap0k"
        );
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
