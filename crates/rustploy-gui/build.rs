//! Build script:
//! 1. Stages os **logos** dos blueprints (só imagens) para `$OUT_DIR` — o
//!    `src/embedded.rs` os embute daí no binário standalone de release.
//! 2. Embeds the Windows application icon, manifest and version metadata into
//!    the `.exe`.
//!
//! Build scripts run on the *host*, so we detect the **target** OS via the
//! `CARGO_CFG_TARGET_OS` variable Cargo sets — `cfg!(windows)` would reflect
//! the host and stay false during the Linux→Windows cross build. The resource
//! is compiled by whatever RC tool `embed-resource` finds; for the cross build
//! that is `llvm-rc` (our `assets/rustploy.rc` has no `#include`s, so no
//! Windows SDK headers are needed).

use std::path::Path;

/// Extensões de imagem que os logos de blueprint usam (o resto da pasta —
/// `docker-compose.yml`/`template.toml`/`.md` — é do daemon e a GUI nunca lê).
const LOGO_EXTS: &[&str] = &["png", "svg", "webp", "jpg", "jpeg", "gif", "ico", "avif", "bmp"];

fn main() {
    stage_blueprint_logos();

    println!("cargo:rerun-if-changed=assets/rustploy.rc");
    println!("cargo:rerun-if-changed=assets/rustploy.ico");
    println!("cargo:rerun-if-changed=assets/application.manifest");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    // Deriva as macros do VERSIONINFO a partir de CARGO_PKG_VERSION (ex.:
    // "0.1.0" -> comma "0,1,0,0" e string "0.1.0"), passadas como defines para
    // o RC. Assim a versão do .exe nunca sai do lugar em relação ao Cargo.toml.
    let version = std::env::var("CARGO_PKG_VERSION")
        .unwrap_or_else(|_| "0.0.0".into());

    let mut parts: Vec<String> = version
        .split(['.', '-', '+'])
        .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_owned)
        .collect();

    while parts.len() < 4 {
        parts.push("0".into());
    }

    let comma = parts[..4].join(",");

    // llvm-rc recebe os defines via macros do embed-resource. A string precisa
    // das aspas escapadas para chegar como literal entre aspas no .rc.
    let macros = [
        format!("RUSTPLOY_VER_COMMA={comma}"),
        format!("RUSTPLOY_VER_STR=\"{version}\""),
    ];

    let _ = embed_resource::compile("assets/rustploy.rc", &macros);
}

/// Copia só os arquivos de imagem de `crates/shared/templates/blueprints/**`
/// para `$OUT_DIR/blueprint_logos/**`, **espelhando a estrutura `<id>/<arquivo>`**
/// (que é o caminho por onde o `EmbeddedAssets` os serve). Assim o binário de
/// release embute apenas os ~14 MB de logos, e não os ~6 MB de
/// `docker-compose.yml`/`template.toml` — que a GUI nunca lê e que já vivem
/// embutidos no daemon (via `crates/shared/build.rs`).
fn stage_blueprint_logos() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let src = Path::new(&manifest).join("../shared/templates/blueprints");
    let dst = Path::new(&out).join("blueprint_logos");

    // Re-stage quando a árvore de blueprints mudar.
    println!("cargo:rerun-if-changed={}", src.display());

    // Limpa o staging anterior (para não reter logos de blueprints removidos).
    let _ = std::fs::remove_dir_all(&dst);
    std::fs::create_dir_all(&dst).expect("criar staging de logos");

    copy_images(&src, &dst);
}

/// Copia recursivamente os arquivos com extensão de imagem de `src` para `dst`,
/// preservando o caminho relativo.
fn copy_images(src: &Path, dst: &Path) {
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => panic!("lendo {}: {e}", src.display()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy_images(&path, &dst.join(entry.file_name()));
        } else if is_image(&path) {
            std::fs::create_dir_all(dst).expect("criar diretório de staging");
            std::fs::copy(&path, dst.join(entry.file_name()))
                .unwrap_or_else(|e| panic!("copiando {}: {e}", path.display()));
        }
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| LOGO_EXTS.contains(&e.as_str()))
}
