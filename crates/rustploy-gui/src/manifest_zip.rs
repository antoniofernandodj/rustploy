//! Ponte Lua ↔ Rust para o `.zip` do Infra as Code.
//!
//! O motor glacier-ui traz `zip_dir` (compacta um diretório) mas **não** o
//! inverso, e a camada Lua não tem como abrir um `.zip`. O Infra as Code precisa
//! dos dois sentidos: exportar grava um `.zip` com dois arquivos e nada mais;
//! importar lê um `.zip` e exige **exatamente** um `.yml`/`.yaml` e um `.toml`,
//! na raiz, sem mais nada.
//!
//! Estas duas funções entram como globais de todo `<script>` via
//! [`glacier_ui::GlacierDaemon::lua_extension`] (ver `app::run`), implementadas
//! com o mesmo crate `zip` que o glacier já usa — então o cargo deduplica e não
//! há uma segunda cópia do crate no binário.

use std::fs::File;
use std::io::{Read, Write};

use glacier_ui::mlua::{self, Lua};

/// Os dois nomes de entrada que o export grava e o import espera na raiz do zip.
const YML_ENTRY: &str = "rustploy.yml";
const TOML_ENTRY: &str = "rustploy.vars.toml";

/// Cria um `.zip` em `path` com exatamente `rustploy.yml` e `rustploy.vars.toml`
/// na raiz — sem diretório de staging: escreve os dois direto no arquivo final.
fn write_manifest_zip(path: &str, yaml: &str, toml: &str) -> std::io::Result<()> {
    let opts = || {
        zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
    };
    let mut zip = zip::write::ZipWriter::new(File::create(path)?);
    zip.start_file(YML_ENTRY, opts())?;
    zip.write_all(yaml.as_bytes())?;
    zip.start_file(TOML_ENTRY, opts())?;
    zip.write_all(toml.as_bytes())?;
    zip.finish()?;
    Ok(())
}

/// Valida e lê o `.zip` importado: **exatamente** um `*.yml`/`*.yaml` e um
/// `*.toml`, na raiz, e nada mais. Devolve `(yaml, toml)` ou uma mensagem de
/// erro pronta para exibir.
fn read_manifest_zip(path: &str) -> Result<(String, String), String> {
    let file = File::open(path).map_err(|e| format!("não consegui abrir o zip: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("zip inválido: {e}"))?;

    let mut yaml: Option<String> = None;
    let mut toml: Option<String> = None;
    let mut extras: Vec<String> = Vec::new();

    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("erro ao ler a entrada {i} do zip: {e}"))?;
        let name = entry.name().to_string();

        // "sem nada a mais": diretórios e qualquer subpasta contam como extra.
        if entry.is_dir() || name.contains('/') {
            extras.push(name);
            continue;
        }

        let lower = name.to_ascii_lowercase();
        let is_yaml = lower.ends_with(".yml") || lower.ends_with(".yaml");
        let is_toml = lower.ends_with(".toml");
        if !is_yaml && !is_toml {
            extras.push(name);
            continue;
        }

        let mut content = String::new();
        entry
            .read_to_string(&mut content)
            .map_err(|e| format!("{name} não é texto UTF-8 válido: {e}"))?;

        let slot = if is_yaml { &mut yaml } else { &mut toml };
        if slot.is_some() {
            let tipo = if is_yaml { ".yml" } else { ".toml" };
            return Err(format!("o zip tem mais de um arquivo {tipo}"));
        }
        *slot = Some(content);
    }

    if !extras.is_empty() {
        return Err(format!(
            "o zip só pode ter um .yml e um .toml — sobrou: {}",
            extras.join(", ")
        ));
    }
    let yaml = yaml.ok_or_else(|| "o zip não tem um arquivo .yml".to_string())?;
    let toml = toml.ok_or_else(|| "o zip não tem um arquivo .toml".to_string())?;
    Ok((yaml, toml))
}

/// Instala os globais `manifest_zip_write` e `manifest_zip_read` na VM Luau.
/// Passada a `GlacierDaemon::lua_extension` em [`crate::app::run`].
///
/// - `manifest_zip_write(zip_path, yaml, toml) -> (ok: boolean, err: string?)`
/// - `manifest_zip_read(zip_path) -> { ok: boolean, yaml: string?, toml: string?, error: string? }`
pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let write = lua.create_function(|_, (path, yaml, toml): (String, String, String)| {
        match write_manifest_zip(&path, &yaml, &toml) {
            Ok(()) => Ok((true, None)),
            Err(e) => Ok((false, Some(e.to_string()))),
        }
    })?;
    lua.globals().set("manifest_zip_write", write)?;

    let read = lua.create_function(|lua, path: String| {
        let t = lua.create_table()?;
        match read_manifest_zip(&path) {
            Ok((yaml, toml)) => {
                t.set("ok", true)?;
                t.set("yaml", yaml)?;
                t.set("toml", toml)?;
            }
            Err(e) => {
                t.set("ok", false)?;
                t.set("error", e)?;
            }
        }
        Ok(t)
    })?;
    lua.globals().set("manifest_zip_read", read)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("rustploy_mzip_{}_{name}", std::process::id()));
        p
    }

    #[test]
    fn round_trip() {
        let zip = tmp("ok.zip");
        write_manifest_zip(zip.to_str().unwrap(), "svc: yaml", "vars = 1").unwrap();
        let (y, t) = read_manifest_zip(zip.to_str().unwrap()).unwrap();
        assert_eq!(y, "svc: yaml");
        assert_eq!(t, "vars = 1");
        let _ = std::fs::remove_file(&zip);
    }

    #[test]
    fn rejects_extra_file() {
        let zip = tmp("extra.zip");
        {
            let opts = zip::write::SimpleFileOptions::default();
            let mut zw = zip::write::ZipWriter::new(File::create(&zip).unwrap());
            zw.start_file("a.yml", opts).unwrap();
            zw.write_all(b"y").unwrap();
            zw.start_file("b.toml", opts).unwrap();
            zw.write_all(b"t").unwrap();
            zw.start_file("readme.txt", opts).unwrap();
            zw.write_all(b"x").unwrap();
            zw.finish().unwrap();
        }
        let err = read_manifest_zip(zip.to_str().unwrap()).unwrap_err();
        assert!(err.contains("readme.txt"), "{err}");
        let _ = std::fs::remove_file(&zip);
    }

    #[test]
    fn rejects_missing_toml() {
        let zip = tmp("noyaml.zip");
        {
            let opts = zip::write::SimpleFileOptions::default();
            let mut zw = zip::write::ZipWriter::new(File::create(&zip).unwrap());
            zw.start_file("only.yml", opts).unwrap();
            zw.write_all(b"y").unwrap();
            zw.finish().unwrap();
        }
        let err = read_manifest_zip(zip.to_str().unwrap()).unwrap_err();
        assert!(err.contains(".toml"), "{err}");
        let _ = std::fs::remove_file(&zip);
    }

    #[test]
    fn rejects_two_yaml() {
        let zip = tmp("twoyaml.zip");
        {
            let opts = zip::write::SimpleFileOptions::default();
            let mut zw = zip::write::ZipWriter::new(File::create(&zip).unwrap());
            zw.start_file("a.yml", opts).unwrap();
            zw.write_all(b"y").unwrap();
            zw.start_file("b.yaml", opts).unwrap();
            zw.write_all(b"y2").unwrap();
            zw.start_file("c.toml", opts).unwrap();
            zw.write_all(b"t").unwrap();
            zw.finish().unwrap();
        }
        let err = read_manifest_zip(zip.to_str().unwrap()).unwrap_err();
        assert!(err.contains(".yml"), "{err}");
        let _ = std::fs::remove_file(&zip);
    }
}
