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
    // app.gv itself links app.gss (<link rel="stylesheet">, global since
    // glacier-ui 0.23), so register_component picks it up — no separate
    // load_stylesheet call needed here.
    m.register_component("app", "crates/rustploy-gui/views/app.gv")
        .expect("app.gv + imports must register (includes app.gss parsing — an unknown property drops the whole sheet)");
    m.set_initial_screen("app");
    m
}

/// Cd's to the workspace root (idempotent — safe alongside `boot`).
fn cd_ws_root() {
    let crate_dir = env!("CARGO_MANIFEST_DIR");
    let ws_root = std::path::Path::new(crate_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    std::env::set_current_dir(ws_root).expect("cd workspace root");
}

/// A janela "Novo projeto" (`new_project_form.gv` + `new_project_window.luau`) é
/// um motor à parte, aberto por `open_window`; não passa pelo `app.gv` acima,
/// então validamos que registra e renderiza por conta própria — semeando a
/// conexão como `open_window({ data = ... })` faria.
#[test]
fn new_project_form_window_renders() {
    cd_ws_root();
    let mut m = GlacierUI::new();
    m.define_data("api_url", "http://localhost");
    m.define_data("api_token", "t");
    m.register_component(
        "new_project_form",
        "crates/rustploy-gui/views/new_project_form.gv",
    )
    .expect("new_project_form.gv must register");
    m.set_initial_screen("new_project_form");
    m.reevaluate_all().expect("eval new_project_form");
    assert!(
        m.render("new_project_form").is_ok(),
        "render new_project_form"
    );
}

/// A janela "Novo job" (`new_job_window.gv` + `new_job_window.luau`) é um
/// motor à parte, aberto por `open_new_job_window` (handlers/jobs.luau).
/// Semeia projetos/serviços já buscados (como `open_window({ data = ... })`
/// faria) e valida os três passos (escolher projeto → escolher serviço →
/// formulário, com cada tipo de recorrência).
#[test]
fn new_job_window_renders() {
    cd_ws_root();
    let mut m = GlacierUI::new();
    m.define_data("api_url", "http://localhost");
    m.define_data("api_token", "t");
    m.define_data("njob_projects", r#"[{"id":"prj_1","name":"acme"}]"#);
    m.define_data(
        "njob_services",
        r#"[{"id":"svc_1","name":"web","project_id":"prj_1"}]"#,
    );
    m.register_component(
        "new_job_window",
        "crates/rustploy-gui/views/new_job_window.gv",
    )
    .expect("new_job_window.gv must register");
    m.set_initial_screen("new_job_window");

    m.define_data("njob_step", "pick_project");
    m.reevaluate_all()
        .expect("eval new_job_window/pick_project");
    assert!(
        m.render("new_job_window").is_ok(),
        "render new_job_window/pick_project"
    );

    m.define_data("njob_step", "pick_service");
    m.define_data("njob_project_name", "acme");
    m.define_data(
        "njob_services_filtered",
        r#"[{"id":"svc_1","name":"web","project_id":"prj_1"}]"#,
    );
    m.reevaluate_all()
        .expect("eval new_job_window/pick_service");
    assert!(
        m.render("new_job_window").is_ok(),
        "render new_job_window/pick_service"
    );

    m.define_data("njob_step", "form");
    m.define_data("njob_service_name", "web");
    // Chave única "HH:MM" do <timeedit> e a coleção do <radiogroup> de dia da
    // semana — as duas semeadas pelo init() de new_job_window.luau na janela
    // real; aqui o teste faz o papel dele.
    m.define_data("njob_time", "03:00");
    m.define_data(
        "weekdays",
        r#"[{"id":"0","label":"Seg"},{"id":"1","label":"Ter"},{"id":"2","label":"Qua"},{"id":"3","label":"Qui"},{"id":"4","label":"Sex"},{"id":"5","label":"Sáb"},{"id":"6","label":"Dom"}]"#,
    );
    for kind in ["manual", "interval", "daily", "weekly"] {
        m.define_data("njob_kind", kind);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval new_job_window/form {kind}: {e}"));
        assert!(
            m.render("new_job_window").is_ok(),
            "render new_job_window/form {kind}"
        );
    }
}

/// A janela de logs ao vivo (`log_window.gv` + `log_window.luau`) é um motor à
/// parte, aberto por `open_logs_window`; validamos que registra e renderiza por
/// conta própria — semeando a conexão + o serviço + o tail como `open_window`.
#[test]
fn log_window_renders() {
    cd_ws_root();
    let mut m = GlacierUI::new();
    m.define_data("api_url", "http://localhost");
    m.define_data("api_token", "t");
    m.define_data("lw_title", "Logs · api");
    m.define_data("lw_stream_url", "/api/services/svc1/logs");
    m.define_data(
        "lw_seed",
        r#"[{"stream":"Stdout","line":"hello","timestamp":"2026-07-10T23:00:00Z"}]"#,
    );
    m.register_component("log_window", "crates/rustploy-gui/views/log_window.gv")
        .expect("log_window.gv must register");
    m.set_initial_screen("log_window");
    m.reevaluate_all().expect("eval log_window");
    assert!(m.render("log_window").is_ok(), "render log_window");
}

/// O wizard "Novo serviço" (`new_service_window.gv`, que importa `new_service.gv`
/// + `new_service_window.luau`) também é uma janela à parte, aberta por
/// `open_new_service_window`. Validamos que registra e renderiza cada passo do
/// wizard como motor isolado — semeando a conexão/projeto como `open_window`.
#[test]
fn new_service_wizard_window_renders() {
    cd_ws_root();
    let mut m = GlacierUI::new();
    m.define_data("api_url", "http://localhost");
    m.define_data("api_token", "t");
    m.define_data("selected_project_id", "p1");
    m.define_data("proj_name", "demo");
    m.register_component(
        "new_service_window",
        "crates/rustploy-gui/views/new_service_window.gv",
    )
    .expect("new_service_window.gv must register");
    m.set_initial_screen("new_service_window");

    // Dados que os passos de banco/template esperam (o init do script tenta o
    // catálogo real, mas o fetch suspende sem executor — semeamos à mão).
    m.define_data("ns_db_has_dbname", "true");
    m.define_data("ns_db_has_user", "true");
    m.define_data("ns_db_has_rootpw", "true");
    m.define_data("ns_db_has_replica", "true");
    m.define_data(
        "ns_dbs",
        r#"[{"id":"postgres","label":"PostgreSQL","image":"postgres:18"}]"#,
    );
    m.define_data(
        "ns_templates",
        r#"[{"id":"forgejo","name":"Forgejo","description":"git","logo":"crates/shared/templates/blueprints/forgejo/forgejo.svg","logo_kind":"svg"},{"id":"wordpress","name":"WordPress","description":"cms","logo":"crates/shared/templates/blueprints/wordpress/wordpress.png","logo_kind":"img"}]"#,
    );
    m.define_data(
        "ns_template_vars",
        r#"[{"idx":"0","label":"Domínio","placeholder":"x"}]"#,
    );

    for step in [
        "pick_type",
        "pick_db",
        "app_form",
        "db_form",
        "compose_form",
        "pick_template",
        "template_form",
    ] {
        m.define_data("ns_step", step);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval new_service/{step}: {e}"));
        assert!(
            m.render("new_service_window").is_ok(),
            "render new_service/{step}"
        );
    }
}

#[test]
fn all_screens_and_service_tabs_render() {
    let mut m = boot();

    // Login screen.
    m.reevaluate_all().expect("eval login");
    assert!(m.render("app").is_ok(), "login render");

    // Shell views.
    for view in [
        "deployments",
        "projects",
        "service",
        "monitoring",
        "ingress",
        "docker",
        "settings",
        "schedules",
        "support",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", view);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval view {view}: {e}"));
        assert!(m.render("app").is_ok(), "render view {view}");
    }

    // Deploy Engine → painel "NA FILA" (fila global): itens enfileirados
    // (arrastáveis) + estado pausado + botão de retomar.
    m.define_data("view", "deploy_engine");
    m.define_data("eng_queued_count", "2");
    m.define_data("eng_paused", "true");
    m.define_data(
        "eng_queued",
        r#"[{"deployment_id":"dep_1","pos":"1","service":"api","project":"acme"},{"deployment_id":"dep_2","pos":"2","service":"worker","project":"acme"}]"#,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval deploy_engine: {e}"));
    assert!(m.render("app").is_ok(), "render deploy_engine com fila");

    // Ingress → tabela de portas TCP de host (separada das rotas de domínio).
    m.define_data("view", "ingress");
    m.define_data("host_ports_count", "1");
    m.define_data(
        "host_ports",
        r#"[{"service":"web","project":"acme","host_port":"8081","container_port":"80"}]"#,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval ingress/host_ports: {e}"));
    assert!(m.render("app").is_ok(), "render ingress/host_ports");

    // Docker → as 5 sub-abas (o loop de views acima só renderiza o default
    // "containers"; containers/images/volumes/networks/registry têm cada
    // uma seu próprio painel escopado por docker_tab, nunca exercitados).
    m.define_data("view", "docker");
    m.define_data("docker_containers_count", "1");
    m.define_data(
        "docker_containers",
        r#"[{"id_full":"c1","name":"rp_api_live","image":"acme/api:latest","owner":"acme / api","state_kind":"ok","state_label":"running","can_remove":"0"}]"#,
    );
    m.define_data("docker_images_count", "1");
    m.define_data(
        "docker_images",
        r#"[{"id_full":"i1","tags":"acme/api:latest","owner":"acme / api","size":"120 MB","created":"12/07","in_use_kind":"ok","in_use_label":"EM USO"}]"#,
    );
    m.define_data("docker_volumes_count", "1");
    m.define_data(
        "docker_volumes",
        r#"[{"name":"pgdata","owner":"—","size":"1.2 GB","in_use_kind":"ok","in_use_label":"EM USO"}]"#,
    );
    m.define_data("docker_networks_count", "1");
    m.define_data(
        "docker_networks",
        r#"[{"name":"rp_net_acme","owner":"acme","in_use_kind":"ok","in_use_label":"EM USO"}]"#,
    );
    m.define_data("registry_status_label", "ativo em 127.0.0.1:5100");
    m.define_data("registry_status_enabled", "true");
    m.define_data("registry_storage_human", "340 MB");
    m.define_data("registry_repos_count", "1");
    m.define_data(
        "registry_repos",
        r#"[{"name":"acme/api","tag_count":"3","size":"340 MB"}]"#,
    );
    m.define_data("registry_tokens_count", "1");
    m.define_data(
        "registry_tokens",
        r#"[{"name":"ci","scope":"pull","created":"12/07"}]"#,
    );
    m.define_data("registry_selected_repo", "");
    m.define_data("registry_tags_loading", "false");
    m.define_data("registry_tags_count", "0");
    m.define_data("registry_tags", "[]");
    for tab in ["containers", "images", "volumes", "networks", "registry"] {
        m.define_data("docker_tab", tab);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval docker/{tab}: {e}"));
        assert!(m.render("app").is_ok(), "render docker/{tab}");
    }
    // Registry: também o branch "repo selecionado" (lista de tags), não só a
    // lista de repos.
    m.define_data("registry_selected_repo", "acme/api");
    m.define_data("registry_tags_count", "1");
    m.define_data(
        "registry_tags",
        r#"[{"tag":"latest","size":"120 MB","created":"12/07","digest_short":"sha256:abcd1234"}]"#,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval docker/registry com repo selecionado: {e}"));
    assert!(
        m.render("app").is_ok(),
        "render docker/registry com repo selecionado"
    );

    // Schedules → tabela global de jobs one-shot (todos os projetos).
    m.define_data("view", "schedules");
    m.define_data("jobs_count", "1");
    m.define_data(
        "jobs_summary",
        r#"[{"id":"job_1","name":"backup-db","owner":"acme / postgres","recurrence":"a cada 6h","enabled":true,"enabled_label":"Pausar","last_run_label":"ok","last_run_kind":"ok","last_run_id":"jrun_1","next_run_at":"12/07 03:00"}]"#,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval schedules: {e}"));
    assert!(m.render("app").is_ok(), "render schedules com dados");

    // Projeto aberto (project_services): grid de serviços e a aba de
    // variáveis de ambiente de nível de projeto.
    // Nome de env var absurdamente longo: exercita o truncamento de
    // `key_display` (env_var_row em fmt/service_detail.luau) sem quebrar o
    // `key` completo usado por delete/reorder/.env.
    m.define_data(
        "proj_env",
        r##"[{"key":"__c0","value":"# comentário","kind":"comment"},{"key":"A_VERY_LONG_ENVIRONMENT_VARIABLE_NAME_THAT_SHOULD_BE_TRUNCATED","key_display":"A_VERY_LONG_ENVIRONMENT_VARIABLE_NAME_TH…","value":"x","kind":"plain"}]"##,
    );
    m.define_data("proj_jobs_count", "1");
    m.define_data(
        "proj_jobs",
        r#"[{"id":"job_1","name":"backup-db","recurrence":"a cada 6h","enabled":true,"enabled_label":"Pausar","last_run_label":"ok","last_run_kind":"ok","last_run_id":"jrun_1","next_run_at":"12/07 03:00"}]"#,
    );
    m.define_data("proj_secrets_count", "1");
    m.define_data(
        "proj_secrets",
        r#"[{"name":"GITHUB_TOKEN","name_display":"GITHUB_TOKEN"}]"#,
    );
    for proj_tab in ["services", "env", "secrets", "jobs"] {
        m.define_data("view", "project_services");
        m.define_data("proj_tab", proj_tab);
        m.define_data("proj_loading", "false");
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval project_services/{proj_tab}: {e}"));
        assert!(
            m.render("app").is_ok(),
            "render project_services/{proj_tab}"
        );
    }

    // Aba Variáveis no modo "usar secret" (o form troca o campo valor pelo nome
    // do secret e lista os chips) e a lista de secrets vazia (estado inicial).
    m.define_data("proj_tab", "env");
    m.define_data("penv_new_is_secret", "true");
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval project_services/env secret: {e}"));
    assert!(
        m.render("app").is_ok(),
        "render project_services/env modo secret"
    );
    m.define_data("proj_secrets_count", "0");
    m.define_data("proj_secrets", "[]");
    for proj_tab in ["env", "secrets"] {
        m.define_data("proj_tab", proj_tab);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval {proj_tab} sem secrets: {e}"));
        assert!(m.render("app").is_ok(), "render {proj_tab} sem secrets");
    }
    m.define_data("penv_new_is_secret", "false");

    // Settings → Git sub-tab (provider list + connect form, both methods).
    m.define_data("view", "settings");
    m.define_data("gitea_count", "1");
    for mode in ["oauth", "pat"] {
        m.define_data("settings_tab", "git");
        m.define_data("gp_mode", mode);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval settings/git {mode}: {e}"));
        assert!(m.render("app").is_ok(), "render settings/git {mode}");
    }

    // Settings → Web Server (default tab). A URL pública é derivada pelo daemon
    // (`DaemonSettings.public_base_url`) e exibida só-leitura — não há mais campo
    // de domínio editável aqui.
    m.define_data("settings_tab", "web");
    m.define_data("ss_public_base", "https://rustploy.meusite.com");
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval settings/web: {e}"));
    assert!(m.render("app").is_ok(), "render settings/web");

    // Settings → Infra as Code: export panel (yaml+dotenv textareas), the
    // missing-vars error branch, and the applied-report branch.
    m.define_data("settings_tab", "iac");
    m.define_data("iac_has_export", "true");
    m.define_data("iac_yaml", "apiVersion: rustploy/v1\nprojects: []\n");
    m.define_data("iac_dotenv", "[project.acme.env]\nLOG_LEVEL = \"info\"\n");
    m.define_data("iac_has_missing", "true");
    m.define_data("iac_missing_vars", "DB_PASS, API_TOKEN");
    m.define_data("iac_has_report", "true");
    m.define_data(
        "iac_report_lines",
        r#"["[created] project acme","[updated] service acme/web"]"#,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval settings/iac: {e}"));
    assert!(m.render("app").is_ok(), "render settings/iac");

    // Settings → Manutenção (limpeza automática de Docker): as 3 recorrências
    // (cada uma mostra campos diferentes: HOURS pra interval, HORÁRIO pra
    // daily, +WEEKDAY pra weekly) e o toggle geral + os 6 sub-toggles do que
    // limpar.
    m.define_data("settings_tab", "maintenance");
    m.define_data("dc_enabled", "true");
    m.define_data("dc_hours", "6");
    // Chave única "HH:MM" do <timeedit> — antes eram dc_hour + dc_minute.
    m.define_data("dc_time", "03:00");
    m.define_data("dc_weekday", "0");
    // Coleção de opções do <radiogroup> de dia da semana; sem ela o grupo
    // renderiza vazio (é o mesmo contrato do for-each: lê chave, não texto).
    m.define_data(
        "weekdays",
        r#"[{"id":"0","label":"Seg"},{"id":"1","label":"Ter"},{"id":"2","label":"Qua"},{"id":"3","label":"Qui"},{"id":"4","label":"Sex"},{"id":"5","label":"Sáb"},{"id":"6","label":"Dom"}]"#,
    );
    m.define_data("dc_containers", "true");
    m.define_data("dc_images", "true");
    m.define_data("dc_images_all", "false");
    m.define_data("dc_volumes", "false");
    m.define_data("dc_volumes_all", "false");
    m.define_data("dc_networks", "true");
    m.define_data("dc_build_cache", "true");
    m.define_data("dc_next_run_label", "hoje às 03:00");
    m.define_data(
        "dc_last_run_text",
        "12/07 03:00 · 3 removidos · 120 MB liberados",
    );
    m.define_data("dc_running", "false");
    m.define_data("dc_msg", "");
    for kind in ["interval", "daily", "weekly"] {
        m.define_data("dc_kind", kind);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval settings/maintenance {kind}: {e}"));
        assert!(
            m.render("app").is_ok(),
            "render settings/maintenance {kind}"
        );
    }

    // Service detail tabs (the editable forms + log views).
    for tab in [
        "general",
        "connection",
        "environment",
        "domains",
        "deployments",
        "healthcheck",
        "logs",
        "advanced",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", "service");
        m.define_data("tab", tab);
        // Exercise the env editor + build-log panel + Gitea sub-tab branches too.
        m.define_data("env_text_open", "true");
        // Lista de env com um comentário (linha display-only), uma var normal e
        // um nome absurdamente longo (exercita o truncamento de `key_display`).
        m.define_data(
            "svc_env",
            r##"[{"key":"__c0","value":"# comentário","kind":"comment"},{"key":"OLA","key_display":"OLA","value":"mundo","kind":"plain"},{"key":"A_VERY_LONG_ENVIRONMENT_VARIABLE_NAME_THAT_SHOULD_BE_TRUNCATED","key_display":"A_VERY_LONG_ENVIRONMENT_VARIABLE_NAME_TH…","value":"x","kind":"plain"}]"##,
        );
        m.define_data("dep_selected", "abc123");
        // Aba Deployments: bloco de webhook com a URL já emitida (o serviço tem
        // token, ou seja, já foi deployado ao menos uma vez).
        m.define_data("svc_webhook_supported", "true");
        m.define_data(
            "svc_webhook_url",
            "https://rustploy.meusite.com/webhook/svc_01ABC/f4b53d4d9d574a55",
        );
        m.define_data(
            "svc_webhook_url_short",
            "https://rustploy.meusite.com/webhook/svc_01ABC…",
        );
        // Show the Gitea sub-tab and render its picker body.
        m.define_data("gitea_count", "1");
        m.define_data("prov_tab", "gitea");
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval tab {tab}: {e}"));
        assert!(m.render("app").is_ok(), "render tab {tab}");
    }

    // General → provider da origem: as sub-abas Git e Zip do bloco
    // Provider (só "gitea" era exercitada acima) + o editor de Compose
    // (svc_source_kind="Compose" troca o bloco inteiro pelo textarea).
    m.define_data("tab", "general");
    m.define_data("svc_source_kind", "Git");
    m.define_data("erro_f_gen_port", "");
    for prov in ["git", "zip"] {
        m.define_data("prov_tab", prov);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval general/prov_tab={prov}: {e}"));
        assert!(m.render("app").is_ok(), "render general/prov_tab={prov}");
    }
    m.define_data("svc_source_kind", "Compose");
    m.define_data("svc_compose", "services:\n  web:\n    image: nginx\n");
    m.define_data("svc_compose_orig", "services:\n  web:\n    image: nginx\n");
    m.reevaluate_all().expect("eval general/compose");
    assert!(m.render("app").is_ok(), "render general/compose");

    // Webhook, os outros dois estados: serviço ainda sem token (nunca deployado,
    // mostra o aviso em vez da URL) e serviço Compose (sem webhook nenhum).
    m.define_data("tab", "deployments");
    m.define_data("svc_webhook_url", "");
    m.reevaluate_all()
        .expect("eval deployments/webhook sem token");
    assert!(
        m.render("app").is_ok(),
        "render deployments/webhook sem token"
    );

    m.define_data("svc_webhook_supported", "false");
    m.reevaluate_all()
        .expect("eval deployments/webhook compose");
    assert!(
        m.render("app").is_ok(),
        "render deployments/webhook compose"
    );
}

/// Regressão: abaixo de 900px de largura a sidebar vira um trilho de ícones —
/// o rótulo de cada NavItem (ex.: "Deploy Engine", "Projects (N)") precisa
/// sumir (`hidden`), senão não cabe e quebra o layout (era exatamente esse o
/// bug reportado: rótulos longos, sem espaço pra quebrar, bagunçando a
/// sidebar). Descoberto assim: nav_item.gv usava seletor agrupado por vírgula
/// dentro de `@media` (".nav_label_on, .nav_label_off { hidden: true; }") —
/// o GSS não suporta agrupamento por vírgula (nem fora de `@media`); a string
/// inteira virava uma ÚNICA chave, que nunca casava com nenhuma classe real,
/// então a regra nunca era aplicada. Cada seletor precisa da própria
/// declaração (ver nav_item.gv).
#[test]
fn sidebar_nav_label_hidden_below_900px() {
    use glacier_ui::widget::EngineMessage;
    let mut m = boot();
    m.define_data("screen", "shell");
    m.define_data("view", "deployments");
    m.reevaluate_all().expect("eval shell");
    // Mesma largura que reproduziu o bug (persistida em
    // rustploy-gui-window.json de uma sessão real).
    let _ = m.dispatch(&EngineMessage::Viewport {
        width: 731.0,
        height: 680.0,
    });

    fn find_texts<'a>(
        node: &'a glacier_ui::parser::UiNode,
        out: &mut Vec<&'a glacier_ui::parser::UiNode>,
    ) {
        if let glacier_ui::parser::NodeType::Text { content, .. } = &node.kind {
            if content == "Deploy Engine" || content.starts_with("Projects (") {
                out.push(node);
            }
        }
        for child in &node.children {
            find_texts(child, out);
        }
    }

    let ast = m.evaluated("app").expect("app evaluated");
    let mut found = Vec::new();
    find_texts(ast, &mut found);
    assert_eq!(
        found.len(),
        2,
        "esperava achar os rótulos \"Deploy Engine\" e \"Projects (N)\""
    );
    for n in &found {
        assert_eq!(
            n.hidden,
            Some(true),
            "rótulo {:?} deveria estar hidden abaixo de 900px",
            n.kind
        );
    }
}

/// Regressão: as ações da tela de serviço (Deploy/Reload/Rebuild/Stop) têm duas
/// fileiras que se alternam por largura. Acima de 1080px vale a de texto; abaixo,
/// a compacta (ícone + tooltip) — senão os 4 rótulos por extenso não cabem e o
/// título de 30px transborda por baixo deles ("Deploy por cima do nome"). Ambas
/// existem sempre no AST; o que muda é qual está `hidden`.
#[test]
fn service_actions_collapse_to_icons_when_narrow() {
    use glacier_ui::widget::EngineMessage;

    // O rótulo de um botão é `Button { text }`, não um nó Text filho. Conta os
    // botões de ação visíveis por fileira: (n_full, n_compact). Só as 4 ações
    // (svc_deploy/reload/rebuild/stop) usam esses textos, então não há colisão
    // com ícones da sidebar (que são nós <text>, não botões).
    fn count_visible(
        node: &glacier_ui::parser::UiNode,
        full: &mut u32,
        compact: &mut u32,
        ancestor_hidden: bool,
    ) {
        let hidden = ancestor_hidden || node.hidden == Some(true);
        if let glacier_ui::parser::NodeType::Button { text, .. } = &node.kind {
            if !hidden {
                if matches!(text.as_str(), "Deploy" | "Reload" | "Rebuild" | "Stop") {
                    *full += 1;
                }
                if matches!(text.as_str(), "▶" | "⟳" | "⚙" | "■") {
                    *compact += 1;
                }
            }
        }
        for child in &node.children {
            count_visible(child, full, compact, hidden);
        }
    }

    let mut m = boot();
    m.define_data("screen", "shell");
    m.define_data("view", "service");
    m.define_data("tab", "general");
    m.reevaluate_all().expect("eval service");

    // Largo: fileira de texto visível, compacta oculta.
    let _ = m.dispatch(&EngineMessage::Viewport {
        width: 1400.0,
        height: 820.0,
    });
    let (mut full, mut compact) = (0, 0);
    count_visible(
        m.evaluated("app").expect("app"),
        &mut full,
        &mut compact,
        false,
    );
    assert_eq!(
        (full, compact),
        (4, 0),
        "em 1400px espera 4 botões de texto e 0 ícones"
    );

    // Estreito: inverte.
    let _ = m.dispatch(&EngineMessage::Viewport {
        width: 980.0,
        height: 820.0,
    });
    let (mut full, mut compact) = (0, 0);
    count_visible(
        m.evaluated("app").expect("app"),
        &mut full,
        &mut compact,
        false,
    );
    assert_eq!(
        (full, compact),
        (0, 4),
        "em 980px espera 0 botões de texto e 4 ícones"
    );
}

/// A avaliação do glacier é **escopada** (0.38+): só a tela ativa é construída,
/// não todo template registrado. Isso importa aqui mais do que na média dos
/// apps: `app.gv` importa a árvore inteira de views (login, shell, home,
/// service, componentes), e avaliar um template inlina recursivamente tudo que
/// ele usa — então a versão antiga reconstruía a UI completa uma vez **por
/// template importado**, a cada tecla digitada e a cada linha de log que chega
/// pelo SSE.
///
/// Este teste trava o ganho: registrar `app.gv` (que puxa a dúzia de views) e
/// ativá-la deve deixar exatamente UMA árvore avaliada.
#[test]
fn so_a_tela_ativa_e_avaliada() {
    let m = boot();

    // As views importadas estão todas registradas...
    for importado in ["Login", "Shell"] {
        assert!(
            m.is_registered(importado),
            "{importado} deveria ter sido importado por app.gv"
        );
    }
    // ...mas só a tela ativa está avaliada (as demais são inlinadas dentro dela).
    assert!(m.render("app").is_ok(), "a tela ativa renderiza");
    assert!(
        matches!(
            m.render("Login"),
            Err(glacier_ui::GlacierError::NotEvaluated(_))
        ),
        "uma view importada não deve ficar avaliada como raiz por conta própria"
    );
}

/// Logout tem que zerar a RAM da sessão: nada do daemon anterior pode continuar
/// no contexto (nomes de projeto, linhas de log, o próprio api_token) — foi um
/// bug real, porque o `disconnect` antigo limpava só quatro chaves à mão.
/// Hoje ele apaga o `ctx` inteiro e deixa o `init()` semear os defaults, então
/// este teste dispara a ação de verdade (`UiClick`, o mesmo caminho do botão
/// Disconnect) e inspeciona o contexto do motor.
#[test]
fn disconnect_limpa_o_contexto_da_sessao() {
    let mut m = boot();

    // Estado de uma sessão conectada, do trivial ao sensível.
    for (k, v) in [
        ("connected", "true"),
        ("screen", "shell"),
        ("api_url", "https://rustploy.example"),
        ("api_token", "token-secreto"),
        ("projects_count", "7"),
        ("proj_name", "acme"),
        (
            "proj_secrets",
            r#"[{"name":"GITHUB_TOKEN","name_display":"GITHUB_TOKEN"}]"#,
        ),
        (
            "svc_env",
            r#"[{"key":"API_KEY","value":"secret:API_KEY","kind":"secret"}]"#,
        ),
        ("selected_project_id", "prj_1"),
    ] {
        m.define_data(k, v);
    }
    m.reevaluate_all().expect("eval sessão conectada");

    let _ = m.dispatch(&glacier_ui::EngineMessage::UiClick("disconnect".into()));

    let ctx = m.context();
    for k in [
        "api_url",
        "api_token",
        "proj_name",
        "proj_secrets",
        "svc_env",
        "selected_project_id",
    ] {
        assert!(
            ctx.get(k).is_none(),
            "ctx.{k} sobreviveu ao logout: {:?}",
            ctx.get(k)
        );
    }
    // O que o init() repõe: volta ao estado de boot, não ao da sessão.
    assert_eq!(ctx.get("connected").map(String::as_str), Some("false"));
    assert_eq!(ctx.get("screen").map(String::as_str), Some("login"));
    assert_eq!(
        ctx.get("projects_count").map(String::as_str),
        Some("…"),
        "contador deve voltar a 'carregando', não a um 0 mentiroso"
    );
}

/// Regressão: o item "Projects" da sidebar apagava (perdia o fundo azul)
/// assim que você entrava num projeto ou num serviço — `nav_item.gv`
/// comparava `{view}` contra um `target` de UMA view só (`equals`), e
/// `project_services`/`service` não são `"projects"`. Corrigido usando
/// `one_of` (glacier-ui 0.57.8): `target="projects project_services
/// service"` casa com qualquer uma das três. `nav_row_on` é a classe que dá
/// o fundo azul — só que o widget `<button>` do glacier-ui lê a propriedade
/// `color:` do GSS pro fundo (não `background:`, que é ignorada em botões;
/// ver `widget.rs` do glacier-ui, `NodeType::Button { color, .. }`), então o
/// campo que importa é `node.kind`'s `color`, não o `node.background`
/// genérico (esse é para containers/rows).
#[test]
fn nav_item_projects_fica_aceso_nas_sub_telas() {
    use glacier_ui::parser::NodeType;

    // `on_click` chega namespaceado pelo componente que o hospeda
    // (`namespace_action` — ver eval.rs no glacier-ui): mesmo com o valor
    // vindo de um prop (`action="nav_projects"` em shell.gv), o botão vive
    // dentro do template do componente `NavItem`, então o dispatch final é
    // "NavItem::nav_projects".
    fn projects_nav_button_lit<'a>(node: &'a glacier_ui::parser::UiNode) -> Option<bool> {
        if let NodeType::Button {
            on_click, color, ..
        } = &node.kind
            && on_click.as_deref() == Some("NavItem::nav_projects")
        {
            return Some(color.as_deref() == Some("#1F6FEB"));
        }
        node.children.iter().find_map(projects_nav_button_lit)
    }

    let mut m = boot();
    for (view, esperado_aceso) in [
        ("deployments", false),
        ("projects", true),
        ("project_services", true),
        ("service", true),
        ("settings", false),
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", view);
        m.reevaluate_all()
            .unwrap_or_else(|e| panic!("eval view {view}: {e}"));
        let ast = m.evaluated("app").expect("app evaluated");
        let aceso = projects_nav_button_lit(ast).expect("item Projects deveria existir na sidebar");
        assert_eq!(
            aceso,
            esperado_aceso,
            "view={view}: item Projects deveria estar {} ",
            if esperado_aceso { "aceso" } else { "apagado" }
        );
    }
}

/// Título e tamanho das janelas passaram a morar no `<screen>` do próprio
/// template (glacier-ui 0.59): saíram do builder Rust, no caso da principal, e
/// das chamadas `open_window{…}` do Luau, no caso das filhas. Este teste é o que
/// garante que eles não sumiram no caminho — um cabeçalho apagado por engano não
/// quebra nenhum render, só faz a janela nascer com o default do iced.
#[test]
fn janelas_declaram_titulo_e_tamanho_no_proprio_template() {
    cd_ws_root();

    // (arquivo, componente, título esperado, tamanho esperado)
    let janelas = [
        (
            "crates/rustploy-gui/views/app.gv",
            "app",
            Some("Rustploy"),
            (1280.0, 820.0),
        ),
        (
            "crates/rustploy-gui/views/new_project_form.gv",
            "new_project_form",
            Some("Novo projeto — Rustploy"),
            (460.0, 340.0),
        ),
        (
            "crates/rustploy-gui/views/new_job_window.gv",
            "new_job_window",
            Some("Novo job — Rustploy"),
            (560.0, 700.0),
        ),
        (
            "crates/rustploy-gui/views/new_service_window.gv",
            "new_service_window",
            Some("Novo serviço — Rustploy"),
            (560.0, 700.0),
        ),
        (
            "crates/rustploy-gui/views/new_registry_token_window.gv",
            "new_registry_token_window",
            Some("Novo token — Rustploy"),
            (480.0, 420.0),
        ),
        // A janela de logs é a exceção proposital: o título é dinâmico ("Logs —
        // nginx", "Build — abc123") e continua vindo de quem a abre; só o
        // tamanho é do arquivo.
        (
            "crates/rustploy-gui/views/log_window.gv",
            "log_window",
            None,
            (900.0, 560.0),
        ),
    ];

    for (arquivo, nome, titulo, tamanho) in janelas {
        let mut m = GlacierUI::new();
        m.register_component(nome, arquivo)
            .unwrap_or_else(|e| panic!("{arquivo} deve registrar: {e}"));
        m.set_initial_screen(nome);
        let meta = m
            .current_screen_meta()
            .unwrap_or_else(|| panic!("{arquivo} deve declarar um <screen>"));
        assert_eq!(meta.title.as_deref(), titulo, "título de {arquivo}");
        assert_eq!(meta.size, Some(tamanho), "tamanho de {arquivo}");
    }

    // A principal também fixa um mínimo — era o `min_size` do `main_window()`.
    let mut m = GlacierUI::new();
    m.register_component("app", "crates/rustploy-gui/views/app.gv")
        .expect("app.gv deve registrar");
    m.set_initial_screen("app");
    assert_eq!(
        m.current_screen_meta().and_then(|s| s.min_size),
        Some((480.0, 680.0)),
        "o min-size da janela principal"
    );
}

/// Remove os blocos `<!-- … -->` para que "a primeira tag" seja a primeira tag
/// de verdade: todo template daqui abre com um comentário de cabeçalho.
fn comentarios_fora(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut resto = src;
    while let Some(i) = resto.find("<!--") {
        out.push_str(&resto[..i]);
        resto = match resto[i..].find("-->") {
            Some(j) => &resto[i + j + 3..],
            // Comentário não fechado: o parse do glacier reclamaria antes; aqui
            // só paramos de copiar.
            None => "",
        };
    }
    out.push_str(resto);
    out
}

/// Os 21 `.gv` estão 100% na forma com cabeçalho (`<screen>` nas janelas,
/// `<component>` no resto) e é isto que este teste trava.
///
/// Ele precisa existir porque o glacier-ui aceita a forma antiga (declarações
/// soltas na raiz, sem cabeçalho) **para sempre**, por compatibilidade — 33 dos
/// 35 `.gv` dos exemplos do próprio motor ainda estão nela. Então nada no build
/// reclamaria de um arquivo daqui que voltasse à forma antiga: as duas convergem
/// na mesma árvore, e o que se perde é silencioso — numa janela, título e
/// tamanho caem no default do iced; num componente, some a fronteira entre
/// declaração e layout.
///
/// Por que TEXTUAL, e não mais um teste de metadado como
/// `janelas_declaram_titulo_e_tamanho_no_proprio_template`: aquele pergunta ao
/// motor o que a tela declarou, e um `<component>` não declara nada por desenho
/// (o cabeçalho recusa atributos nele). Para 15 dos 21 arquivos não há metadado
/// a inspecionar — só o texto diz em que forma o arquivo está.
///
/// Por que VARRE o diretório, em vez de listar arquivos: o alvo é o `.gv` que
/// ainda não foi escrito. Uma lista à mão não cobre o arquivo novo, e quem
/// esquece o cabeçalho nele é a mesma pessoa que esqueceria de atualizar a lista.
#[test]
fn todo_template_comeca_com_cabecalho() {
    cd_ws_root();

    let raiz = std::path::Path::new("crates/rustploy-gui/views");
    let mut vistos = 0;

    // `views/` mistura janelas (<screen>) e views internas (<component>); só
    // `views/components/` é homogêneo — nada ali é janela.
    for (dir, so_component) in [(raiz.to_path_buf(), false), (raiz.join("components"), true)] {
        let mut arquivos: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("ler {}: {e}", dir.display()))
            .map(|e| e.expect("entry").path())
            .filter(|p| p.extension().is_some_and(|e| e == "gv"))
            .collect();
        arquivos.sort();

        for caminho in arquivos {
            let src = std::fs::read_to_string(&caminho)
                .unwrap_or_else(|e| panic!("ler {}: {e}", caminho.display()));
            let sem_comentarios = comentarios_fora(&src);
            let primeira = sem_comentarios.split('<').nth(1).unwrap_or("").trim_start();

            let e_screen = primeira.starts_with("screen");
            let e_component = primeira.starts_with("component");
            assert!(
                e_screen || e_component,
                "{}: todo .gv começa com <screen> (uma janela) ou <component> (o resto) — \
                 o motor aceita a forma antiga sem cabeçalho, então ninguém além deste \
                 teste avisaria",
                caminho.display()
            );
            assert!(
                !(so_component && e_screen),
                "{}: um arquivo em views/components/ não é janela — <component>, não <screen>",
                caminho.display()
            );
            vistos += 1;
        }
    }

    // Guarda contra o teste passar vazio (um caminho errado torna tudo acima
    // um no-op). Comparação frouxa de propósito: acrescentar um `.gv` não deve
    // obrigar a editar este teste — é justamente o atrito que ele elimina.
    assert!(
        vistos >= 21,
        "o teste não achou os templates: {vistos} arquivos varridos"
    );
}

/// As grades de cards (projetos e serviços) passam o item INTEIRO ao componente
/// via `spread="{c}"` (glacier-ui 0.62). Isso troca um atributo por campo por um
/// só — e move a checagem do contrato para o **dado**: um campo que o
/// `fmt/dashboard.luau` não emitir vira `MissingProp` e derruba a tela inteira,
/// não um `{placeholder}` vazio como antes.
///
/// O caso perigoso é o **filler** (o card vazio que completa a fileira do grid):
/// ele nasce de um único `FILLER` compartilhado pelas duas grades, e em Lua um
/// campo `= nil` simplesmente não existe. Por isso ele carrega a união dos dois
/// contratos como string vazia — e é isto que este teste tranca.
#[test]
fn grades_de_cards_renderizam_com_spread() {
    let mut m = boot();
    m.define_data("screen", "shell");
    // connection.luau semeia data_loading="true" no init, e o <scrollable> da
    // grade fica escondido atrás dele — sem isto o for-each nunca roda e o
    // teste passa sem ter avaliado um card sequer.
    m.define_data("data_loading", "false");

    // Uma fileira com um card real + um filler, exatamente a forma que
    // `M.project_rows` produz quando há 1 projeto numa grade de 2 colunas.
    m.define_data("view", "projects");
    m.define_data(
        "project_rows",
        r##"[{"cards":[
            {"filler":"0","id":"prj_1","name":"acme","description":"loja",
             "service_count":"3","running_count":"2","can_delete":"0"},
            {"filler":"1","id":"","name":"","description":"","service_count":"",
             "running_count":"","can_delete":"","port":"","status_label":"",
             "status_color":"","cpu":"","mem":"","container_name":"",
             "container_id":"","container_extra":"","project":""}
        ]}]"##,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval grade de projetos: {e}"));
    assert!(m.render("app").is_ok(), "render grade de projetos");
    let arv = format!("{:?}", m.evaluated("app").unwrap());
    assert!(
        arv.contains("acme"),
        "a grade tem que ter renderizado o card"
    );

    m.define_data("view", "project_services");
    m.define_data("proj_loading", "false");
    m.define_data(
        "project_services",
        r##"[{"cards":[
            {"filler":"0","id":"svc_1","name":"api","project":"acme","port":"8080",
             "status_label":"Rodando","status_color":"#A6E3A1","cpu":"1.2%","mem":"64 MB",
             "container_name":"acme-api","container_id":"abc123","container_extra":"+1"},
            {"filler":"1","id":"","name":"","description":"","service_count":"",
             "running_count":"","can_delete":"","port":"","status_label":"",
             "status_color":"","cpu":"","mem":"","container_name":"",
             "container_id":"","container_extra":"","project":""}
        ]}]"##,
    );
    m.reevaluate_all()
        .unwrap_or_else(|e| panic!("eval grade de serviços: {e}"));
    assert!(m.render("app").is_ok(), "render grade de serviços");
}
