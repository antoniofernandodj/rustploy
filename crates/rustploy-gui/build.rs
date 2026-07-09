//! Build script: embeds the Windows application icon, manifest and version
//! metadata into the `.exe`.
//!
//! Build scripts run on the *host*, so we detect the **target** OS via the
//! `CARGO_CFG_TARGET_OS` variable Cargo sets — `cfg!(windows)` would reflect
//! the host and stay false during the Linux→Windows cross build. The resource
//! is compiled by whatever RC tool `embed-resource` finds; for the cross build
//! that is `llvm-rc` (our `assets/rustploy.rc` has no `#include`s, so no
//! Windows SDK headers are needed).

fn main() {
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
