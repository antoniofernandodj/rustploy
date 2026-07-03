//! Headless validation: every template parses, every screen/tab evaluates and
//! builds an iced element tree without error. Catches malformed KDL and unknown
//! `.gss` properties (which would drop a whole stylesheet) without a display.

use glacier_ui::GlacierUI;

/// Boots the engine the way `main.rs` does, but from the workspace root so the
/// workspace-relative template paths resolve.
fn boot() -> GlacierUI {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ws_root = std::path::Path::new(crate_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    std::env::set_current_dir(ws_root).expect("cd workspace root");

    let mut m = GlacierUI::new();
    m.load_stylesheet("crates/rustploy-gui/styles/app.gss")
        .expect("app.gss must parse (an unknown property drops the whole sheet)");
    m.register_component("app", "crates/rustploy-gui/templates/app.xml")
        .expect("app.xml + imports must register");
    m.set_initial_screen("app");
    m
}

#[test]
fn all_screens_and_service_tabs_render() {
    let mut m = boot();

    // Login screen.
    m.reevaluate_all().expect("eval login");
    assert!(m.render("app").is_ok(), "login render");

    // Shell views.
    for view in [
        "deployments", "projects", "service", "monitoring", "ingress", "docker",
        "settings", "schedules", "support",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", view);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval view {view}: {e}"));
        assert!(m.render("app").is_ok(), "render view {view}");
    }

    // Projeto aberto (project_services): grid de serviços e a aba de
    // variáveis de ambiente de nível de projeto.
    for proj_tab in ["services", "env"] {
        m.define_data("view", "project_services");
        m.define_data("proj_tab", proj_tab);
        m.define_data("proj_loading", "false");
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval project_services/{proj_tab}: {e}"));
        assert!(m.render("app").is_ok(), "render project_services/{proj_tab}");
    }

    // Wizard "Novo serviço": todos os passos.
    for step in [
        "pick_type", "pick_db", "app_form", "db_form", "compose_form",
        "pick_template", "template_form",
    ] {
        m.define_data("view", "new_service");
        m.define_data("ns_step", step);
        // Flags/listas que os passos de banco/template esperam.
        m.define_data("ns_db_has_dbname", "true");
        m.define_data("ns_db_has_user", "true");
        m.define_data("ns_db_has_rootpw", "true");
        m.define_data("ns_db_has_replica", "true");
        m.define_data("ns_dbs", r#"[{"id":"postgres","label":"PostgreSQL","image":"postgres:18"}]"#);
        // Duas linhas do catálogo com logos reais (um vetor, um raster) para
        // exercitar os dois ramos do TemplateRow (Svg vs Image).
        m.define_data(
            "ns_templates",
            r#"[{"id":"forgejo","name":"Forgejo","description":"git","logo":"crates/shared/templates/blueprints/forgejo/forgejo.svg","logo_kind":"svg"},{"id":"wordpress","name":"WordPress","description":"cms","logo":"crates/shared/templates/blueprints/wordpress/wordpress.png","logo_kind":"img"}]"#,
        );
        m.define_data("ns_template_vars", r#"[{"idx":"0","label":"Domínio","placeholder":"x"}]"#);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval new_service/{step}: {e}"));
        assert!(m.render("app").is_ok(), "render new_service/{step}");
    }

    // Settings → Git sub-tab (provider list + connect form, both methods).
    m.define_data("view", "settings");
    m.define_data("gitea_count", "1");
    for mode in ["oauth", "pat"] {
        m.define_data("settings_tab", "git");
        m.define_data("gp_mode", mode);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval settings/git {mode}: {e}"));
        assert!(m.render("app").is_ok(), "render settings/git {mode}");
    }

    // Service detail tabs (the editable forms + log views).
    for tab in [
        "general", "connection", "environment", "domains", "deployments",
        "healthcheck", "logs", "advanced",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", "service");
        m.define_data("tab", tab);
        // Exercise the env editor + build-log panel + Gitea sub-tab branches too.
        m.define_data("env_text_open", "true");
        // Lista de env com um comentário (linha display-only) e uma var normal.
        m.define_data(
            "svc_env",
            r##"[{"key":"__c0","value":"# comentário","kind":"comment"},{"key":"OLA","value":"mundo","kind":"plain"}]"##,
        );
        m.define_data("dep_selected", "abc123");
        // Show the Gitea sub-tab and render its picker body.
        m.define_data("gitea_count", "1");
        m.define_data("prov_tab", "gitea");
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval tab {tab}: {e}"));
        assert!(m.render("app").is_ok(), "render tab {tab}");
    }
}
