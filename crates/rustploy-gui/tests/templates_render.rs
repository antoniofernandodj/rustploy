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
    m.register_component("new_project_form", "crates/rustploy-gui/views/new_project_form.gv")
        .expect("new_project_form.gv must register");
    m.set_initial_screen("new_project_form");
    m.reevaluate_all().expect("eval new_project_form");
    assert!(m.render("new_project_form").is_ok(), "render new_project_form");
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
    m.register_component("new_job_window", "crates/rustploy-gui/views/new_job_window.gv")
        .expect("new_job_window.gv must register");
    m.set_initial_screen("new_job_window");

    m.define_data("njob_step", "pick_project");
    m.reevaluate_all().expect("eval new_job_window/pick_project");
    assert!(m.render("new_job_window").is_ok(), "render new_job_window/pick_project");

    m.define_data("njob_step", "pick_service");
    m.define_data("njob_project_name", "acme");
    m.define_data("njob_services_filtered", r#"[{"id":"svc_1","name":"web","project_id":"prj_1"}]"#);
    m.reevaluate_all().expect("eval new_job_window/pick_service");
    assert!(m.render("new_job_window").is_ok(), "render new_job_window/pick_service");

    m.define_data("njob_step", "form");
    m.define_data("njob_service_name", "web");
    for kind in ["manual", "interval", "daily", "weekly"] {
        m.define_data("njob_kind", kind);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval new_job_window/form {kind}: {e}"));
        assert!(m.render("new_job_window").is_ok(), "render new_job_window/form {kind}");
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
    m.register_component("new_service_window", "crates/rustploy-gui/views/new_service_window.gv")
        .expect("new_service_window.gv must register");
    m.set_initial_screen("new_service_window");

    // Dados que os passos de banco/template esperam (o init do script tenta o
    // catálogo real, mas o fetch suspende sem executor — semeamos à mão).
    m.define_data("ns_db_has_dbname", "true");
    m.define_data("ns_db_has_user", "true");
    m.define_data("ns_db_has_rootpw", "true");
    m.define_data("ns_db_has_replica", "true");
    m.define_data("ns_dbs", r#"[{"id":"postgres","label":"PostgreSQL","image":"postgres:18"}]"#);
    m.define_data(
        "ns_templates",
        r#"[{"id":"forgejo","name":"Forgejo","description":"git","logo":"crates/shared/templates/blueprints/forgejo/forgejo.svg","logo_kind":"svg"},{"id":"wordpress","name":"WordPress","description":"cms","logo":"crates/shared/templates/blueprints/wordpress/wordpress.png","logo_kind":"img"}]"#,
    );
    m.define_data("ns_template_vars", r#"[{"idx":"0","label":"Domínio","placeholder":"x"}]"#);

    for step in [
        "pick_type", "pick_db", "app_form", "db_form", "compose_form",
        "pick_template", "template_form",
    ] {
        m.define_data("ns_step", step);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval new_service/{step}: {e}"));
        assert!(m.render("new_service_window").is_ok(), "render new_service/{step}");
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
        "deployments", "projects", "service", "monitoring", "ingress", "docker",
        "settings", "schedules", "support",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", view);
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval view {view}: {e}"));
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
    m.reevaluate_all().unwrap_or_else(|e| panic!("eval deploy_engine: {e}"));
    assert!(m.render("app").is_ok(), "render deploy_engine com fila");

    // Ingress → tabela de portas TCP de host (separada das rotas de domínio).
    m.define_data("view", "ingress");
    m.define_data("host_ports_count", "1");
    m.define_data(
        "host_ports",
        r#"[{"service":"web","project":"acme","host_port":"8081","container_port":"80"}]"#,
    );
    m.reevaluate_all().unwrap_or_else(|e| panic!("eval ingress/host_ports: {e}"));
    assert!(m.render("app").is_ok(), "render ingress/host_ports");

    // Schedules → tabela global de jobs one-shot (todos os projetos).
    m.define_data("view", "schedules");
    m.define_data("jobs_count", "1");
    m.define_data(
        "jobs_summary",
        r#"[{"id":"job_1","name":"backup-db","owner":"acme / postgres","recurrence":"a cada 6h","enabled":true,"enabled_label":"Pausar","last_run_label":"ok","last_run_kind":"ok","last_run_id":"jrun_1","next_run_at":"12/07 03:00"}]"#,
    );
    m.reevaluate_all().unwrap_or_else(|e| panic!("eval schedules: {e}"));
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
    for proj_tab in ["services", "env", "jobs"] {
        m.define_data("view", "project_services");
        m.define_data("proj_tab", proj_tab);
        m.define_data("proj_loading", "false");
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval project_services/{proj_tab}: {e}"));
        assert!(m.render("app").is_ok(), "render project_services/{proj_tab}");
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

    // Settings → Web Server (default tab). A URL pública é derivada pelo daemon
    // (`DaemonSettings.public_base_url`) e exibida só-leitura — não há mais campo
    // de domínio editável aqui.
    m.define_data("settings_tab", "web");
    m.define_data("ss_public_base", "https://rustploy.meusite.com");
    m.reevaluate_all().unwrap_or_else(|e| panic!("eval settings/web: {e}"));
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
    m.reevaluate_all().unwrap_or_else(|e| panic!("eval settings/iac: {e}"));
    assert!(m.render("app").is_ok(), "render settings/iac");

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
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval tab {tab}: {e}"));
        assert!(m.render("app").is_ok(), "render tab {tab}");
    }

    // Webhook, os outros dois estados: serviço ainda sem token (nunca deployado,
    // mostra o aviso em vez da URL) e serviço Compose (sem webhook nenhum).
    m.define_data("tab", "deployments");
    m.define_data("svc_webhook_url", "");
    m.reevaluate_all().expect("eval deployments/webhook sem token");
    assert!(m.render("app").is_ok(), "render deployments/webhook sem token");

    m.define_data("svc_webhook_supported", "false");
    m.reevaluate_all().expect("eval deployments/webhook compose");
    assert!(m.render("app").is_ok(), "render deployments/webhook compose");
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
    let _ = m.dispatch(&EngineMessage::Viewport { width: 731.0, height: 680.0 });

    fn find_texts<'a>(node: &'a glacier_ui::parser::UiNode, out: &mut Vec<&'a glacier_ui::parser::UiNode>) {
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
    assert_eq!(found.len(), 2, "esperava achar os rótulos \"Deploy Engine\" e \"Projects (N)\"");
    for n in &found {
        assert_eq!(n.hidden, Some(true), "rótulo {:?} deveria estar hidden abaixo de 900px", n.kind);
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
    fn count_visible(node: &glacier_ui::parser::UiNode, full: &mut u32, compact: &mut u32, ancestor_hidden: bool) {
        let hidden = ancestor_hidden || node.hidden == Some(true);
        if let glacier_ui::parser::NodeType::Button { text, .. } = &node.kind {
            if !hidden {
                if matches!(text.as_str(), "Deploy" | "Reload" | "Rebuild" | "Stop") { *full += 1; }
                if matches!(text.as_str(), "▶" | "⟳" | "⚙" | "■") { *compact += 1; }
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
    let _ = m.dispatch(&EngineMessage::Viewport { width: 1400.0, height: 820.0 });
    let (mut full, mut compact) = (0, 0);
    count_visible(m.evaluated("app").expect("app"), &mut full, &mut compact, false);
    assert_eq!((full, compact), (4, 0), "em 1400px espera 4 botões de texto e 0 ícones");

    // Estreito: inverte.
    let _ = m.dispatch(&EngineMessage::Viewport { width: 980.0, height: 820.0 });
    let (mut full, mut compact) = (0, 0);
    count_visible(m.evaluated("app").expect("app"), &mut full, &mut compact, false);
    assert_eq!((full, compact), (0, 4), "em 980px espera 0 botões de texto e 4 ícones");
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
        matches!(m.render("Login"), Err(glacier_ui::GlacierError::NotEvaluated(_))),
        "uma view importada não deve ficar avaliada como raiz por conta própria"
    );
}
