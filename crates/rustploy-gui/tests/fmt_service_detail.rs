//! O `fmt/service_detail.luau` (`compose_host` e `internal_url`) rodando no
//! motor de verdade.
//!
//! Existe desde o rename de serviço: o hostname interno de um serviço Compose é
//! a chave do YAML, que não muda quando o serviço é renomeado, e o card
//! "Internal URL" da aba Connection depende disso. O mesmo cálculo existe em JS
//! (`webui/fmt.js::composeHost`), coberto pelo teste `renomear_servico_na_aba_general`
//! do daemon; aqui é a metade Luau.

use glacier_ui::GlacierUI;

fn boot() -> GlacierUI {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ws_root = std::path::Path::new(crate_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    std::env::set_current_dir(ws_root).expect("cd workspace root");

    // A fixture mora fora da árvore de scripts do app; o `require("fmt/...")`
    // dela precisa desta raiz extra.
    unsafe {
        std::env::set_var("GLACIER_LUAU_PATH", "crates/rustploy-gui/views/scripts");
    }

    let mut m = GlacierUI::new();
    m.register_component(
        "compose_host",
        "crates/rustploy-gui/tests/fixtures/compose_host.gv",
    )
    .expect("registrar a fixture");
    m.set_initial_screen("compose_host");
    m
}

#[test]
fn compose_host_acha_a_chave_do_servico_no_yaml() {
    let m = boot();
    let g = |k: &str| m.context().get(k).cloned().unwrap_or_default();

    assert_eq!(g("host_primeira"), "rp_banco", "1ª chave, pulando comentário e linha em branco");
    assert_eq!(g("host_ingress"), "kong", "ingress_service ganha da 1ª chave");
    assert_eq!(g("host_sem_services"), "nil");
    assert_eq!(g("host_bloco_vazio"), "nil");
    assert_eq!(g("host_nil"), "nil");
}

#[test]
fn internal_url_usa_o_host_do_compose_ou_rp_nome() {
    let m = boot();
    let g = |k: &str| m.context().get(k).cloned().unwrap_or_default();

    // Compose: o nome do serviço (`meu_banco`) não aparece; vale a chave do YAML.
    assert_eq!(g("url_compose"), "postgresql://rp_banco:5432");
    // Application: `rp_<nome>`. Sem `db_kind` a GUI põe `http://` (a webui, hoje,
    // não põe esquema nenhum — divergência anterior ao rename, não mexida aqui).
    assert_eq!(g("url_app"), "http://rp_api:80");
    assert_eq!(g("url_app_db"), "redis://rp_cache:6379");
}
