//! Headless smoke tests: building the `view` tree runs every widget's
//! constructor (including `Scrollable::validate`), so rendering each screen
//! catches layout misuse like "scrollable content must not fill its axis"
//! without needing a window.

use crate::model::*;
use shared::{
    ComposeSource, EnvVar, EnvVarValue, Healthcheck, ResourceLimits, Service, ServiceSource,
    ServiceSpec, ServiceStatus,
};

fn fake_service(id: &str, project_id: &str, db: bool) -> Service {
    let env_vars = if db {
        vec![
            EnvVar { key: "RUSTPLOY_DB_KIND".into(), value: EnvVarValue::Plain("postgres".into()) },
            EnvVar { key: "POSTGRES_DB".into(), value: EnvVarValue::Plain("app".into()) },
            EnvVar { key: "POSTGRES_USER".into(), value: EnvVarValue::Plain("u".into()) },
            EnvVar { key: "POSTGRES_PASSWORD".into(), value: EnvVarValue::Plain("p".into()) },
        ]
    } else {
        vec![EnvVar { key: "FOO".into(), value: EnvVarValue::Plain("bar".into()) }]
    };
    let source = if db {
        ServiceSource::Compose(ComposeSource { content: "services: {}".into() })
    } else {
        ServiceSource::Git(shared::GitSource::default())
    };
    Service {
        id: id.into(),
        spec: ServiceSpec {
            name: format!("svc-{id}"),
            project_id: project_id.into(),
            source,
            port: 8080,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars,
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
            run_command: None,
            run_args: vec!["--flag".into()],
        },
        status: ServiceStatus::Running,
        live_container_id: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn connected_app() -> App {
    let mut app = App::new("rwp://127.0.0.1:8787".into());
    app.connected = true;
    app.projects = vec![shared::Project {
        id: "p1".into(),
        name: "proj".into(),
        description: Some("d".into()),
        env_vars: vec![EnvVar { key: "K".into(), value: EnvVarValue::Plain("V".into()) }],
        created_at: chrono::Utc::now(),
    }];
    app.active_project_id = Some("p1".into());
    app.ps_name = "proj".into();
    app.services = vec![fake_service("s1", "p1", false), fake_service("s2", "p1", true)];
    app.project_secrets = vec!["TOKEN".into()];
    app
}

/// Rendering must not panic for any sidebar destination.
#[test]
fn renders_all_views() {
    let mut app = connected_app();
    let views = [
        View::HomeDeployments, View::HomeMonitoring, View::HomeSchedules, View::HomeIngress,
        View::HomeDocker, View::HomeDeployEngine, View::HomeRequests, View::Projects,
        View::ProjectDetail, View::SettingsWebServer, View::SettingsProfile, View::SettingsUsers,
        View::SettingsAuditLogs, View::SettingsSshKeys, View::SettingsTags, View::SettingsGit,
        View::SettingsRegistry, View::SettingsS3, View::SettingsCerts, View::SettingsSso,
        View::Account,
    ];
    for v in views {
        app.view = v;
        let _ = crate::view::view(&app);
    }
}

#[test]
fn renders_all_project_tabs() {
    let mut app = connected_app();
    app.view = View::ProjectDetail;
    for tab in ProjectTab::ALL {
        app.project_tab = *tab;
        let _ = crate::view::view(&app);
        // also the inline editors open
    }
    app.p_env_editor.open = true;
    app.secret_editor.open = true;
    app.project_tab = ProjectTab::Environment;
    let _ = crate::view::view(&app);
    app.project_tab = ProjectTab::Secrets;
    let _ = crate::view::view(&app);
}

#[test]
fn renders_all_service_tabs() {
    let mut app = connected_app();
    app.view = View::ServiceDetail;
    // exercise both a git service (s1) and a database service (s2)
    for sid in ["s1", "s2"] {
        app.active_service_id = Some(sid.into());
        let svc = app.services.iter().find(|s| s.id == sid).cloned().unwrap();
        app.conn_info = ConnInfo::from_service(&svc);
        app.general = GeneralForm::from_service(&svc);
        app.health = HealthForm::from_service(&svc);
        app.domains = DomainsForm::from_service(&svc);
        app.advanced = AdvancedForm::from_service(&svc);
        for tab in [
            ServiceTab::General, ServiceTab::Connection, ServiceTab::Environment,
            ServiceTab::Domains, ServiceTab::Deployments, ServiceTab::Healthcheck,
            ServiceTab::Logs, ServiceTab::Patches, ServiceTab::Advanced,
        ] {
            app.service_tab = tab;
            let _ = crate::view::view(&app);
        }
    }
    app.s_env_editor.open = true;
    app.service_tab = ServiceTab::Environment;
    let _ = crate::view::view(&app);
}

#[test]
fn renders_wizard_steps() {
    let mut app = connected_app();
    app.view = View::ProjectDetail;
    let mut ns = NsForm::new("p1".into());
    for step in [
        NsStep::PickType, NsStep::PickDb, NsStep::AppForm, NsStep::ComposeForm,
        NsStep::PickTemplate,
    ] {
        ns.step = step;
        app.ns = Some(ns);
        let _ = crate::view::view(&app);
        ns = app.ns.take().unwrap();
    }
    // database form for each kind
    for db in DbKind::ALL {
        ns.db_kind = Some(*db);
        ns.step = NsStep::DbForm;
        app.ns = Some(ns);
        let _ = crate::view::view(&app);
        ns = app.ns.take().unwrap();
    }
    // template variable form
    let t = shared::templates::all().iter().find(|t| !t.variables.is_empty()).unwrap();
    ns.select_template(t);
    app.ns = Some(ns);
    let _ = crate::view::view(&app);
}

#[test]
fn renders_modals_and_toast() {
    let mut app = connected_app();
    app.new_project_open = true;
    let _ = crate::view::view(&app);
    app.new_project_open = false;
    app.confirm = Some(ConfirmAction::DeleteProject("p1".into()));
    let _ = crate::view::view(&app);
    app.confirm = None;
    app.notify("oi", false);
    let _ = crate::view::view(&app);
}

#[test]
fn normalize_url_adds_rwp_scheme() {
    assert_eq!(normalize_url("127.0.0.1"), "rwp://127.0.0.1");
    assert_eq!(normalize_url("  localhost  "), "rwp://localhost");
    // scheme already present — left as typed (authority not touched)
    assert_eq!(normalize_url("rwp://example.com:9000"), "rwp://example.com:9000");
    assert_eq!(normalize_url(""), "");
}

#[test]
fn connect_target_resolves_host_and_port() {
    // default port filled in
    assert_eq!(connect_target("rwp://127.0.0.1").unwrap(), "127.0.0.1:8787");
    assert_eq!(connect_target("rwp://example.com").unwrap(), "example.com:8787");
    // explicit port kept
    assert_eq!(connect_target("rwp://10.0.0.5:9000").unwrap(), "10.0.0.5:9000");
    // a path past the authority is dropped
    assert_eq!(connect_target("rwp://host:1234/ignored").unwrap(), "host:1234");
    // bracketed IPv6
    assert_eq!(connect_target("rwp://[::1]").unwrap(), "[::1]:8787");
    assert_eq!(connect_target("rwp://[::1]:9000").unwrap(), "[::1]:9000");
    // bare authority (no scheme) tolerated
    assert_eq!(connect_target("127.0.0.1").unwrap(), "127.0.0.1:8787");
    // other schemes rejected
    assert!(connect_target("http://127.0.0.1").is_err());
    assert!(connect_target("rwp://").is_err());
}
