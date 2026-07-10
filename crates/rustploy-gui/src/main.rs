//! Rustploy (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

// On Windows release builds, run under the "windows" subsystem so launching the
// GUI does not pop up (and keep open) a console window behind it. Ignored on
// every other target and in debug builds (where a console is handy for logs).
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;

fn main() -> iced::Result {
    // Assets (XML templates, styles, icons, blueprint logos) are referenced by
    // CWD-relative paths; enter their base directory before anything loads.
    assets::locate_and_chdir();

    // Multi-janela sobre `iced::daemon` — a casca custom (fontes, janela
    // borderless, ícone, persistência de geometria) vive em `app::run`.
    app::run()
}
