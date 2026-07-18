//! Rustploy (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

// On Windows release builds, run under the "windows" subsystem so launching the
// GUI does not pop up (and keep open) a console window behind it. Ignored on
// every other target and in debug builds (where a console is handy for logs).
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

mod app;
// Em dev os assets são lidos do disco (com hot-reload) por caminho relativo ao
// CWD → precisamos entrar na pasta-base. Em release eles são embutidos no
// binário (ver `embedded` + `app::run`), então nem o localizador nem o `chdir`
// existem — o executável roda de qualquer diretório, sozinho.
#[cfg(debug_assertions)]
mod assets;
#[cfg(not(debug_assertions))]
mod embedded;

fn main() -> iced::Result {
    // Dev: entra no diretório-base dos assets antes de qualquer carga. Release:
    // nada a localizar — os assets vivem dentro do binário.
    #[cfg(debug_assertions)]
    assets::locate_and_chdir();

    // Multi-janela sobre `iced::daemon` — a casca custom (fontes, janela
    // borderless, ícone, persistência de geometria) vive em `app::run`.
    app::run()
}
