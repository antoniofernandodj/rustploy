//! Rustploy (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. Toda a lógica de
//! rede vive em Luau (`views/scripts/app.luau`), falando HTTP/JSON + SSE com o
//! daemon; este módulo Rust é só a casca da janela (chrome + persistência local).
//!
//! Desde glacier-ui 0.38 este arquivo é **só configuração**: o runner
//! [`GlacierDaemon`] cuida do loop `iced::daemon`, de um motor por janela, das
//! janelas-filhas (`open_window(...)` na Luau), dos broadcasts entre elas e das
//! ações `window:*` da titlebar custom. O que é específico do rustploy entra por
//! ganchos: `main_window`/`child_window` (chrome borderless, ícone, geometria),
//! `on_message` (persistir o login lembrado) e `on_close` (persistir a
//! geometria).
//!
//! Até a 0.37 nada disso era alcançável pelo builder, e a casca aqui era um
//! runtime `iced::daemon` **inteiro reimplementado** — ~250 linhas duplicando o
//! roteamento por janela, os listeners globais e a abertura de filhas, só para
//! poder embutir uma fonte e desenhar a própria titlebar.

mod store;

use std::time::Duration;

use glacier_ui::{
    window, EngineMessage, Font, GlacierDaemon, GlacierUI, Point, Size, WindowGeometry,
};

/// Fontes embutidas (JetBrains Mono): registradas no builder do daemon e usadas
/// como `default_font` de todas as janelas.
const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

/// Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.
/// Chamado por `main` depois de `assets::locate_and_chdir()`.
pub(crate) fn run() -> iced::Result {
    GlacierDaemon::new()
        .title("Rustploy")
        .font(FONT_REGULAR)
        .font(FONT_BOLD)
        .default_font(Font::with_name("JetBrains Mono"))
        .main_window(main_window_settings())
        // Janelas-filhas (ex.: "Novo projeto") também são borderless: o template
        // delas traz a própria titlebar, e sem isto o SO desenharia a nativa por
        // baixo e a janela destoaria da principal.
        .child_window(|_spec, settings| {
            settings.decorations = false;
            settings.platform_specific = platform_specific();
        })
        .main(|motor| {
            if let Err(e) = motor.register_component("app", "crates/rustploy-gui/views/app.xml") {
                // O Display do GlacierError já traz arquivo:linha:coluna, o
                // trecho e a dica — não vale reembrulhar.
                eprintln!("{e}");
            }
            seed_prefs(motor);
            motor.set_initial_screen("app");
        })
        // Persistência do login lembrado: o formulário vive na janela principal e
        // a camada Luau não tem I/O de arquivo, então o script grava url/token no
        // contexto e nós lemos o contexto e escrevemos no disco.
        .on_message(|msg, motor| {
            if should_persist(msg) {
                persist_prefs(motor);
            }
        })
        .on_close(|_motor, geometry| save_geometry(geometry))
        .toast_period(Duration::from_millis(250))
        .run()
}

/// Grava [`store::Prefs`] a partir do contexto: só guarda url/token quando o
/// respectivo "lembrar" está ligado (senão limpa o campo).
fn persist_prefs(motor: &GlacierUI) {
    let g = |k: &str| motor.get_data(k).cloned().unwrap_or_default();
    let remember_url = g("remember_url") == "true";
    let remember_token = g("remember_token") == "true";
    store::Prefs {
        remember_url,
        remember_token,
        url: if remember_url { Some(g("url")) } else { None },
        token: if remember_token { Some(g("token")) } else { None },
    }
    .save();
}

/// Decide se a ação deve disparar a persistência das Prefs de login. O
/// `login.xml` é IMPORTADO em `app.xml` (`<link rel=import as=Login>`), então as
/// ações chegam com namespace do owner (`Login::connect`,
/// `Login::toggle_remember_url`); comparamos só o sufixo (após `::`).
fn should_persist(msg: &EngineMessage) -> bool {
    let action = match msg {
        EngineMessage::UiClick(a) => a.as_str(),
        EngineMessage::UiSubmit { action, .. } => action.as_str(),
        EngineMessage::UiInputChanged { action, .. } => action.as_str(),
        _ => return false,
    };
    let bare = action.rsplit("::").next().unwrap_or(action);
    bare == "connect" || bare.starts_with("toggle_remember")
}

/// Semeia o contexto do glacier com as Prefs de login salvas, para o formulário
/// nascer preenchido. Os nomes de chave batem com os `formControl`/`checked` do
/// `login.xml` (`url`/`token`/`remember_url`/`remember_token`).
fn seed_prefs(motor: &mut GlacierUI) {
    let prefs = store::Prefs::load();
    motor.define_data("remember_url", if prefs.remember_url { "true" } else { "false" });
    motor.define_data("remember_token", if prefs.remember_token { "true" } else { "false" });
    if let Some(url) = prefs.url.filter(|_| prefs.remember_url) {
        motor.define_data("url", &url);
    }
    if let Some(token) = prefs.token.filter(|_| prefs.remember_token) {
        motor.define_data("token", &token);
    }
}

/// Persiste a geometria da janela principal ao fechar, para o app reabrir onde
/// parou. `position` é sempre `None` no Wayland (o protocolo não expõe a posição
/// da janela ao cliente — não é contornável do lado do cliente), então na prática
/// só o tamanho é restaurado lá; `x`/`y` ficam sem valor.
fn save_geometry(geometry: WindowGeometry) {
    store::WindowState {
        width: geometry.size.width,
        height: geometry.size.height,
        x: geometry.position.map(|p| p.x),
        y: geometry.position.map(|p| p.y),
    }
    .save();
}

/// Builds the main window's settings, restoring the last remembered size/position
/// ([`store::WindowState`]) so the app reopens where it was left. Falls back to
/// the default at the platform-default placement on first launch, or when no
/// position was ever saved (e.g. Wayland, which never reports one to restore).
/// Borderless (`decorations: false`) — the OS titlebar is replaced by a custom
/// one in `views/app.xml`, whose `window:*` actions the daemon drives against
/// this window's own id. `exit_on_close_request: false` routes the WM's own close
/// through the daemon's `on_close` hook, so the geometry is saved before the
/// window actually closes.
fn main_window_settings() -> window::Settings {
    let ws = store::WindowState::load();
    let min = Size::new(480.0, 680.0);
    let position = match (ws.x, ws.y) {
        (Some(x), Some(y)) => window::Position::Specific(Point::new(x, y)),
        _ => window::Position::Default,
    };
    window::Settings {
        size: Size::new(ws.width.max(min.width), ws.height.max(min.height)),
        position,
        min_size: Some(min),
        // Taskbar / dock icon while the app runs (Windows taskbar, X11 dock).
        // Embedded so it works regardless of CWD; on Wayland the dock icon
        // instead comes from the `.desktop` file matched by app id, see the
        // Debian package assets in `Cargo.toml`.
        icon: window::icon::from_file_data(include_bytes!("../../assets/rustploy.png"), None).ok(),
        decorations: false,
        exit_on_close_request: false,
        platform_specific: platform_specific(),
        ..Default::default()
    }
}

/// `application_id` only exists on the Linux (X11/Wayland) variant of
/// `PlatformSpecific`; other platforms expose different fields, so the whole
/// block is gated per target to keep the Windows build compiling.
#[cfg(target_os = "linux")]
fn platform_specific() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific {
        application_id: "rustploy-gui".to_string(),
        ..Default::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific() -> window::settings::PlatformSpecific {
    window::settings::PlatformSpecific::default()
}
