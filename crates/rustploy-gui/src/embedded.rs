//! Assets embutidos no binário — modo standalone (só em builds de release).
//!
//! Em **release** (`cfg(not(debug_assertions))`) toda a árvore de assets que o
//! motor lê em runtime é embutida no executável via `include_dir!`, e uma
//! [`EmbeddedAssets`] é injetada no [`glacier_ui::GlacierDaemon`] (ver
//! `app::run`). O binário fica **100% desacoplado dos arquivos**: pode ser
//! copiado sozinho para qualquer lugar e rodar, sem a árvore `crates/…` ao lado
//! nem o `chdir` de `assets.rs`.
//!
//! Em **debug** este módulo nem é compilado: o dev continua lendo do disco (com
//! hot-reload) através do [`glacier_ui::DiskAssets`] default, depois do
//! `assets::locate_and_chdir()`.
//!
//! ## Convenções de caminho
//!
//! Os caminhos que chegam aqui são os mesmos que o motor resolve hoje
//! (relativos ao CWD antes do modo standalone), roteados por prefixo para a
//! árvore embutida correspondente:
//!
//! | prefixo de runtime | árvore embutida |
//! |---|---|
//! | `crates/rustploy-gui/views/…` | [`VIEWS`] (`.gv`, `styles/*.gss`, `styles/theme.json`, `scripts/**/*.luau`) |
//! | `crates/rustploy-gui/assets/icons/…` | [`ICONS`] (ícones SVG) |
//! | `crates/shared/templates/blueprints/…` | [`BLUEPRINTS`] (logos dos templates) |

use std::borrow::Cow;
use std::io;
use std::time::SystemTime;

use glacier_ui::AssetSource;
use include_dir::{Dir, File, include_dir};

/// `views/`: templates `.gv`, estilos `styles/*.gss`, `styles/theme.json` e os
/// scripts Luau em `scripts/**/*.luau` (resolvidos por `require`/`<script src>`).
static VIEWS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/views");
/// `assets/icons/`: ícones SVG referenciados por `<svg src="crates/…/icons/…">`.
static ICONS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/icons");
/// Logos dos blueprints, referenciados via o `{logo}` data-driven do catálogo
/// do daemon (`crates/shared/templates/blueprints/<id>/<arquivo>`).
static BLUEPRINTS: Dir<'static> =
    include_dir!("$CARGO_MANIFEST_DIR/../shared/templates/blueprints");

/// Roteia um caminho lógico para a árvore embutida + o caminho relativo a ela
/// (a chave que `include_dir` usa, relativa à raiz do `#[folder]`).
fn route(path: &str) -> Option<&'static File<'static>> {
    // Normaliza separadores (`\`→`/`) e um eventual `./` inicial; as chaves do
    // `include_dir` são sempre relativas com `/`.
    let norm = path.replace('\\', "/");
    let norm = norm.strip_prefix("./").unwrap_or(&norm);

    const ROUTES: &[(&str, &Dir<'static>)] = &[
        ("crates/rustploy-gui/views/", &VIEWS),
        ("crates/rustploy-gui/assets/icons/", &ICONS),
        ("crates/shared/templates/blueprints/", &BLUEPRINTS),
    ];
    for (prefix, dir) in ROUTES {
        if let Some(rest) = norm.strip_prefix(prefix) {
            return dir.get_file(rest);
        }
    }
    None
}

fn not_found(path: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("asset embutido não encontrado: {path}"),
    )
}

/// [`AssetSource`] servindo a árvore de assets embutida no binário.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmbeddedAssets;

impl AssetSource for EmbeddedAssets {
    fn read_bytes(&self, path: &str) -> io::Result<Cow<'static, [u8]>> {
        route(path)
            .map(|f| Cow::Borrowed(f.contents()))
            .ok_or_else(|| not_found(path))
    }

    fn read_to_string(&self, path: &str) -> io::Result<Cow<'static, str>> {
        let file = route(path).ok_or_else(|| not_found(path))?;
        file.contents_utf8().map(Cow::Borrowed).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("asset embutido não é UTF-8 válido: {path}"),
            )
        })
    }

    fn exists(&self, path: &str) -> bool {
        route(path).is_some()
    }

    fn modified(&self, _path: &str) -> Option<SystemTime> {
        // Embutido não muda sob o processo → desliga o hot-reload no motor.
        None
    }
}

// Estes testes só existem em build de release (o módulo inteiro é
// `cfg(not(debug_assertions))`); rode-os com `cargo test --release -p rustploy-gui`.
#[cfg(test)]
mod tests {
    use super::*;

    /// Cada caminho realmente referenciado em runtime resolve na árvore
    /// embutida — as três convenções de prefixo (views/icons/blueprints), texto
    /// e binário.
    #[test]
    fn resolve_os_assets_referenciados() {
        let a = EmbeddedAssets;
        // views: template, estilo, tema, scripts (entrada + módulo `require`d).
        for p in [
            "crates/rustploy-gui/views/app.gv",
            "crates/rustploy-gui/views/styles/app.gss",
            "crates/rustploy-gui/views/styles/theme.json",
            "crates/rustploy-gui/views/scripts/app.luau",
            "crates/rustploy-gui/views/scripts/handlers/connection.luau",
        ] {
            assert!(a.exists(p), "faltou embutir (texto): {p}");
            assert!(a.read_to_string(p).is_ok(), "não leu (texto): {p}");
        }
        // binários: ícone SVG estático + um logo de blueprint (data-driven).
        for p in [
            "crates/rustploy-gui/assets/icons/terminal.svg",
            "crates/shared/templates/blueprints/ackee/logo.png",
        ] {
            assert!(a.exists(p), "faltou embutir (binário): {p}");
            assert!(!a.read_bytes(p).unwrap().is_empty(), "vazio: {p}");
        }
        // Ausente → NotFound / exists=false.
        assert!(!a.exists("crates/rustploy-gui/views/nao_existe.gv"));
        assert!(a.read_to_string("crates/rustploy-gui/views/nao_existe.gv").is_err());
    }

    /// Prova de ponta a ponta, headless (sem janela e **sem `chdir`**): o motor
    /// carrega o app inteiro só da árvore embutida — template + `<link>`
    /// (estilo/tema) + `<script src>` Luau + a cadeia de `require` dos handlers
    /// + render. Se qualquer asset (incl. um módulo `require`d) faltasse, o
    /// `register_component`/`render` falharia.
    #[test]
    fn motor_sobe_o_app_so_do_binario() {
        use std::sync::Arc;
        let mut motor = glacier_ui::GlacierUI::new().with_asset_source(Arc::new(EmbeddedAssets));
        motor
            .register_component("app", "crates/rustploy-gui/views/app.gv")
            .expect("registrar 'app' a partir dos assets embutidos");
        motor.set_initial_screen("app");
        assert!(motor.render_current().is_ok(), "render do app embutido falhou");
    }
}
