//! Runtime asset location.
//!
//! Every KDL template, stylesheet, icon and blueprint logo is referenced by a
//! path relative to the process' current working directory — both from Rust
//! (`crates/rustploy-gui/styles/app.gss`, `crates/rustploy-gui/views/app.xml`,
//! `crates/shared/views/blueprints/<id>/<logo>`) and from *inside* the KDL
//! themselves (`import ... from="crates/rustploy-gui/views/service.xml"`,
//! `theme "crates/rustploy-gui/styles/theme.json"`, `Svg "crates/rustploy-gui/…"`).
//!
//! Rather than rewrite every literal, we locate the directory that holds those
//! `crates/rustploy-gui/…` and `crates/shared/…` trees once at startup and
//! `chdir` into it. After that, every relative path resolves no matter how the
//! app was launched: `cargo run` from the workspace root, the Windows `.zip`
//! (assets sit next to the `.exe`), or the Debian package (assets under
//! `/usr/share/rustploy`).

use std::path::{Path, PathBuf};

/// A file that must exist under any valid asset base — used as the probe.
const MARKER: &str = "crates/rustploy-gui/views/app.xml";

/// System-wide install prefix used by the Debian package (see the `deb`
/// metadata in `Cargo.toml`).
const SYSTEM_PREFIX: &str = "/usr/share/rustploy";

/// Finds the asset base directory and `chdir`s into it so all the
/// CWD-relative asset paths resolve. Resolution order:
///
/// 1. `$RUSTPLOY_UI_ASSETS` — explicit override.
/// 2. The executable's own directory — portable / Windows `.zip` layout.
/// 3. [`SYSTEM_PREFIX`] — Debian package layout.
/// 4. The current directory, if it already contains the assets — `cargo run`
///    from the workspace root during development (no `chdir` needed).
///
/// Best-effort: if none match, the CWD is left as-is and the app will surface
/// a "stylesheet/template not found" error, which is the clearest signal.
pub fn locate_and_chdir() {
    if let Some(base) = find_base() {
        if let Err(e) = std::env::set_current_dir(&base) {
            eprintln!("assets: falha ao entrar em {}: {e}", base.display());
        }
    }
}

fn find_base() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("RUSTPLOY_UI_ASSETS") {
        let p = PathBuf::from(dir);
        if has_marker(&p) {
            return Some(p);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if has_marker(dir) {
                return Some(dir.to_path_buf());
            }
        }
    }

    let system = PathBuf::from(SYSTEM_PREFIX);
    if has_marker(&system) {
        return Some(system);
    }

    // Already at a valid base (dev run from the workspace root): stay put.
    None
}

fn has_marker(base: &Path) -> bool {
    base.join(MARKER).is_file()
}
