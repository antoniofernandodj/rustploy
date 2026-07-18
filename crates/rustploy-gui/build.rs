//! Build script:
//! 1. Stages os **logos** dos blueprints (só imagens) para `$OUT_DIR` — o
//!    `src/embedded.rs` os embute daí no binário standalone de release. Os logos
//!    **raster** são reduzidos (Lanczos3) para no máx [`LOGO_MAX_DIM`]px na
//!    maior dimensão e re-codificados como PNG: os originais são ~512×512 e
//!    aparecem a ~30px (`template_row.gv`), então sem isto a GPU faria um
//!    downscale de ~17x por quadro (serrilhado) e o binário carregaria ~12 MB de
//!    logo. SVGs são vetor — copiados intactos.
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

/// Alvo do redimensionamento dos logos raster: a maior dimensão é reduzida para
/// no máximo isto, preservando a proporção. Os logos aparecem a ~30px lógicos
/// (`template_row.gv`); 96px cobre telas HiDPI (até ~3x) e ainda corta os 512×512
/// originais em ordens de grandeza. Só **reduz** — imagens já menores (ou vetor)
/// passam intactas, para não borrar quem já é pequeno.
const LOGO_MAX_DIM: u32 = 96;

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

/// Leva só os logos de `crates/shared/templates/blueprints/**` para
/// `$OUT_DIR/blueprint_logos/**`, **espelhando a estrutura `<id>/<arquivo>`** (o
/// caminho por onde o `EmbeddedAssets` os serve) e **reduzindo os raster** (ver
/// [`copy_images`]/[`downscale_png`]). Assim o binário de release embute só os
/// logos — já encolhidos —, e não os `docker-compose.yml`/`template.toml` (que a
/// GUI nunca lê e já vivem embutidos no daemon, via `crates/shared/build.rs`).
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

/// Percorre recursivamente `src` e leva os logos para `dst`, preservando o
/// caminho relativo. Raster passa por [`stage_raster`] (reduz + re-encoda);
/// vetor (`.svg`) é copiado intacto.
fn copy_images(src: &Path, dst: &Path) {
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => panic!("lendo {}: {e}", src.display()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            copy_images(&path, &dst.join(entry.file_name()));
            continue;
        }
        // O nome do arquivo no staging tem de ser IDÊNTICO ao original: o daemon
        // devolve o caminho `<id>/<arquivo>` e o `EmbeddedAssets` o serve por
        // essa chave. Re-encodar um raster para PNG mantendo, digamos, um nome
        // `.jpg` é inofensivo — o iced/`image` decodifica por conteúdo (magic
        // bytes), não pela extensão.
        let out = dst.join(entry.file_name());
        if is_raster(&path) {
            std::fs::create_dir_all(dst).expect("criar diretório de staging");
            stage_raster(&path, &out);
        } else if is_logo(&path) {
            // Vetor (`.svg`) e formatos que o `image` não cobre (`.ico`/`.avif`):
            // copiados intactos — svg é crisp em qualquer tamanho e os demais
            // ainda são logos válidos, só não passam pelo resize.
            std::fs::create_dir_all(dst).expect("criar diretório de staging");
            std::fs::copy(&path, &out)
                .unwrap_or_else(|e| panic!("copiando {}: {e}", path.display()));
        }
    }
}

/// Lê um logo raster, reduz para no máx [`LOGO_MAX_DIM`] (Lanczos3) re-encodando
/// como PNG, e grava em `dst`. Degrada: se não conseguir decodificar (ou a
/// imagem já for pequena), copia os bytes originais em vez de derrubar o build.
fn stage_raster(src: &Path, dst: &Path) {
    let bytes = std::fs::read(src).unwrap_or_else(|e| panic!("lendo {}: {e}", src.display()));
    let staged = downscale_png(&bytes).unwrap_or(bytes);
    std::fs::write(dst, staged).unwrap_or_else(|e| panic!("gravando {}: {e}", dst.display()));
}

/// Decodifica `bytes`, e — se a maior dimensão passar de [`LOGO_MAX_DIM`] —
/// reduz preservando a proporção (Lanczos3) e re-encoda como PNG. Devolve `None`
/// (→ mantém o original) quando não decodifica ou quando a imagem já é pequena
/// (re-encodar não valeria o custo/risco de mexer no que já está bom).
fn downscale_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory(bytes).ok()?;
    if img.width().max(img.height()) <= LOGO_MAX_DIM {
        return None;
    }
    let resized = img.resize(LOGO_MAX_DIM, LOGO_MAX_DIM, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    resized
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .ok()?;
    Some(out)
}

fn ext_lower(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// Qualquer arquivo de logo (raster, vetor ou os formatos raros) — decide o que
/// entra no staging. Os que não são [`is_raster`] são copiados intactos.
fn is_logo(path: &Path) -> bool {
    ext_lower(path).is_some_and(|e| LOGO_EXTS.contains(&e.as_str()))
}

/// Um logo raster que o `image` sabe decodificar (as features habilitadas no
/// `Cargo.toml`). `ico`/`avif` ficam de fora do resize e caem no ramo de cópia
/// intacta — continuam sendo logos válidos.
fn is_raster(path: &Path) -> bool {
    matches!(
        ext_lower(path).as_deref(),
        Some("png" | "webp" | "jpg" | "jpeg" | "gif" | "bmp")
    )
}
