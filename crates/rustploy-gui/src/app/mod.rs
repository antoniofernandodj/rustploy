//! Rustploy (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. Toda a lógica de
//! rede vive em Luau (`views/scripts/app.luau`), falando HTTP/JSON + SSE com o
//! daemon; este módulo Rust é só a casca da janela.
//!
//! Desde glacier-ui 0.107 a configuração de janela e de aplicativo mora no
//! **markup** de `views/app.gv`: `<screen decorations icon>` (chrome borderless e
//! ícone, também nas janelas-filhas), `<app id single_instance
//! remember_geometry>` (instância única, geometria lembrada e diretório de dados
//! do `storage` do Luau) e `<tray>` (menu da bandeja). O runner
//! [`GlacierDaemon`] os lê antes de abrir qualquer janela.
//!
//! Sobra aqui só o que o markup não expressa: as fontes embutidas, o
//! antialiasing, o período dos toasts, a extensão Luau do `.zip`, o espelho da
//! sessão para a API de agente, o `application_id` do Linux e a fonte de assets
//! embutida em release.
//!
//! Atenção: um `.main(|motor| …)` escrito à mão **desliga** a leitura de `<app>`
//! e `<tray>` (o runner não sabe qual template ele abre) — por isso a principal
//! é registrada por `.main_template(…)`.

use std::time::Duration;

use glacier_ui::{Font, GlacierDaemon, window};

/// Fontes embutidas (JetBrains Mono): registradas no builder do daemon e usadas
/// como `default_font` de todas as janelas.
const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

/// Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.
/// Chamado por `main` depois de `assets::locate_and_chdir()`.
pub(crate) fn run() -> iced::Result {
    // API de agente: servidor HTTP local (hyper) que empresta a sessão desta
    // janela para um agente rodando na mesma máquina operar o rustploy REMOTO.
    // Ver `src/agent/mod.rs` e `docs/api-agente-no-gui.md`.
    let sessao_agente = crate::agent::SharedSession::default();
    // Canal de mão contrária (glacier-ui 0.58.6+): deixa a thread do servidor
    // injetar ações no motor desta janela — é o que torna login, navegação e
    // qualquer botão alcançáveis por HTTP. Criado ANTES de `run()`, que é
    // quando o daemon decide registrar a subscription que o drena.
    let ui_agente = glacier_ui::external::sender();

    let daemon = GlacierDaemon::new()
        // Título, tamanho, decorations e ícone: `<screen>` de `views/app.gv`.
        // Instância única, geometria e diretório de dados: `<app>`. Bandeja:
        // `<tray>`. O `.main_template` (e não `.main`) mantém o `<app>`/`<tray>`
        // lidos; o caminho é relativo ao workspace, onde `assets::locate_and_chdir`
        // deixa o CWD.
        .main_template("crates/rustploy-gui/views/app.gv")
        .font(FONT_REGULAR)
        .font(FONT_BOLD)
        .default_font(Font::with_name("JetBrains Mono"))
        // Extensão da camada Luau: `manifest_zip_read` / `manifest_zip_write`,
        // usadas pelo Infra as Code (Settings). O motor tem `zip_dir` mas não o
        // inverso, e a camada Lua não abre um `.zip` — ver `src/manifest_zip.rs`.
        .lua_extension(crate::manifest_zip::install)
        // Espelha a sessão da GUI (api_url/api_token/connected, escritos no
        // contexto por `handlers/connection.luau`) para a API de agente. O
        // gancho roda depois de CADA dispatch da janela principal, então cobre
        // login, logout e troca de servidor sem precisar conhecer nenhum dos
        // três — e a escrita só acontece quando algo de fato mudou.
        //
        // É também aqui que a API de agente sobe: o `.main()` que a subia saiu
        // (ver o aviso no topo do arquivo). Este gancho só roda na instância
        // PRIMÁRIA — a instância única encerra a segunda antes de qualquer
        // dispatch, então um segundo lançamento não reescreve o handoff da
        // viva — e `spawn` é idempotente (um `AtomicBool`), então chamá-lo a
        // cada dispatch custa uma troca atômica.
        .on_message({
            let sessao = sessao_agente.clone();
            let ui = ui_agente.clone();
            move |_msg, motor| {
                crate::agent::spawn(sessao.clone(), ui.clone());
                sessao.sync_from_context(motor.context());
            }
        })
        // Só o `application_id` (Linux) mora aqui: o glacier não o lê do
        // markup. O resto do chrome (borderless, ícone) vem do `<screen>`, que
        // se aplica por cima destas settings.
        .main_window(window::Settings {
            platform_specific: platform_specific(),
            ..Default::default()
        })
        .child_window(|_spec, settings| {
            settings.platform_specific = platform_specific();
        })
        .toast_period(Duration::from_millis(250))
        // O MSAAx4 default do iced custa caro num fallback 100% por software
        // (sem GPU compatível — `wgpu` recusa adapters não-Vulkan-compliant e
        // cai pra CPU). Telas de formulário/lista não perdem em legibilidade
        // sem antialiasing, então o custo não compensa aqui.
        .antialiasing(false);

    // Release: injeta a fonte de assets embutida — o motor passa a ler
    // templates/estilos/scripts/binários de dentro do binário, e nada do disco.
    // Em dev, o `daemon` fica com o `DiskAssets` default (disco + hot-reload).
    #[cfg(not(debug_assertions))]
    let daemon = daemon.assets(std::sync::Arc::new(crate::embedded::EmbeddedAssets));

    // `run` só volta quando o app encerra de verdade (fechar a janela apenas
    // recolhe para a bandeja). É aqui que o handoff da API de agente deixa de
    // valer: o token morre com o processo, então um arquivo sobrevivente só
    // confundiria quem o lesse depois.
    let resultado = daemon.run();
    crate::agent::cleanup();
    resultado
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
