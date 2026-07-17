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
    window,
    // EngineMessage e GlacierUI eram usados só pela persistência do login movida
    // para o Luau (funções comentadas abaixo); reintroduzir se ela voltar.
    Font,
    GlacierDaemon,
    Point,
    Size,
    TrayActions,
    TrayConfig,
    TrayItem,
    WindowGeometry,
    notifications_enabled,
    set_notifications_enabled,
};

/// Ícone da bandeja: os mesmos bytes PNG embutidos usados no ícone da janela
/// (`main_window_settings`), para o app ter a mesma identidade na área de
/// notificação.
const TRAY_ICON: &[u8] = include_bytes!("../../assets/rustploy.png");

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
        // Raiz gravável do global `storage` do Luau (persistência do login
        // lembrado, ver `connection.luau`). Sem isto o `storage` gravaria
        // relativo aos assets — read-only no pacote `.deb` (`/usr/share`). Aponta
        // para o mesmo data dir do usuário que o resto da persistência local usa,
        // então o arquivo cai em `~/.local/share/rustploy/.glacier-storage/`.
        .storage_dir(shared::fallback_data_dir())
        // Ícone de bandeja: com ele, fechar a última janela NÃO encerra o app —
        // ele recolhe para a bandeja, e o menu controla o ciclo de vida. Ver
        // `docs/plano-tray-bandeja-e-ciclo-de-vida.md`.
        .tray(tray_config())
        .on_tray(handle_tray)
        .main_window(main_window_settings())
        // Janelas-filhas (ex.: "Novo projeto") também são borderless: o template
        // delas traz a própria titlebar, e sem isto o SO desenharia a nativa por
        // baixo e a janela destoaria da principal.
        .child_window(|_spec, settings| {
            settings.decorations = false;
            settings.platform_specific = platform_specific();
        })
        .main(|motor| {
            if let Err(e) = motor
                .register_component(
                    "app",
                    "crates/rustploy-gui/views/app.gv"
                ) {
                // O Display do GlacierError já traz arquivo:linha:coluna, o
                // trecho e a dica — não vale reembrulhar.
                    eprintln!("{e}");
                }
            motor.set_initial_screen("app");
        })

        .on_close(
            |_motor, geometry| save_geometry(geometry)
        )
        .toast_period(Duration::from_millis(250))
        .run()
}

/// Menu da bandeja. Os ids (`open`/`notifications`/`quit`) são o que chega ao
/// [`handle_tray`]. O item de notificações começa como "Disable…" porque as
/// notificações começam ligadas (default do glacier).
fn tray_config() -> TrayConfig {
    TrayConfig {
        icon: TRAY_ICON.to_vec(),
        tooltip: "Rustploy".to_string(),
        items: vec![
            TrayItem::button("open", "Open Rustploy"),
            TrayItem::button("notifications", "Disable notifications"),
            TrayItem::separator(),
            TrayItem::button("quit", "Quit Rustploy"),
        ],
    }
}

/// Trata um clique num item da bandeja (ou o clique esquerdo no ícone, no
/// Windows, que o glacier roteia como "abrir"). `open`/`quit` são ações do
/// runner; `notifications` alterna o interruptor global do SO e reflete o novo
/// estado no rótulo do próprio item.
fn handle_tray(id: &str, tray: &mut TrayActions) {
    match id {
        "open" => tray.open_main(),
        "quit" => tray.quit(),
        "notifications" => {
            let on = !notifications_enabled();
            set_notifications_enabled(on);
            tray.set_label(
                "notifications",
                if on {
                    "Disable notifications"
                } else {
                    "Enable notifications"
                },
            );
        }
        _ => {}
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
/// one in `views/app.gv`, whose `window:*` actions the daemon drives against
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
        icon: window::icon::from_file_data(
            include_bytes!(
                "../../assets/rustploy.png"),
            None
        ).ok(),
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
