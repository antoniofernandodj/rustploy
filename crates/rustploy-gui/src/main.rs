//! Rustploy (glacier-ui) — desktop client whose UI is described in KDL
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

// On Windows release builds, run under the "windows" subsystem so launching the
// GUI does not pop up (and keep open) a console window behind it. Ignored on
// every other target and in debug builds (where a console is handy for logs).
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;
use app::App;
use glacier_ui::{GlacierApp, Font};


fn main() -> iced::Result {
    // Assets (XML templates, styles, icons, blueprint logos) are referenced by
    // CWD-relative paths; enter their base directory before anything loads.
    assets::locate_and_chdir();

    App::bootstrap()
        .title("Rustploy")
        .theme(App::theme)
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice())
        .default_font(Font::with_name("JetBrains Mono"))
        .exit_on_close_request(false)
        .window(app::window_settings())
        .run()
}
