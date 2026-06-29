//! Headless validation: every template parses, every screen/tab evaluates and
//! builds an iced element tree without error. Catches malformed XML and unknown
//! `.iss` properties (which would drop a whole stylesheet) without a display.

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
    m.load_stylesheet("crates/remote-ui/styles/app.iss")
        .expect("app.iss must parse (an unknown property drops the whole sheet)");
    m.register_component("app", "crates/remote-ui/templates/app.xml")
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

    // Service detail tabs (the editable forms + log views).
    for tab in [
        "general", "connection", "environment", "domains", "deployments",
        "healthcheck", "logs", "advanced",
    ] {
        m.define_data("screen", "shell");
        m.define_data("view", "service");
        m.define_data("tab", tab);
        // Exercise the env editor + build-log panel branches too.
        m.define_data("env_text_open", "true");
        m.define_data("dep_selected", "abc123");
        m.reevaluate_all().unwrap_or_else(|e| panic!("eval tab {tab}: {e}"));
        assert!(m.render("app").is_ok(), "render tab {tab}");
    }
}
