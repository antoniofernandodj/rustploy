//! Build script: embeds the Windows application icon into the `.exe`.
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

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("assets/rustploy.rc", embed_resource::NONE);
    }
}
