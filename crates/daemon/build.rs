//! Gera, em tempo de compilação, os assets estáticos da web UI/PWA
//! (`crates/daemon/webui/`, ver `docs/` e `src/api/web_ui.rs`) que o daemon
//! serve como alternativa ao client iced (bloqueado pelo Defender no
//! Windows). Mesmo padrão de `crates/shared/build.rs` (blueprints): lê os
//! fontes legíveis, processa e embute o resultado em `$OUT_DIR`, incluído via
//! `include!` — zero I/O de asset em runtime.
//!
//! Por arquivo:
//!   1. Minifica (comentários + espaço em branco supérfluo — ver `minify`;
//!      conservador de propósito, não reescreve a sintaxe: HTML/CSS/JS não
//!      têm dependência de build própria e um minificador agressivo teria
//!      risco de introduzir bugs sutis, ex. quebra de ASI em JS).
//!   2. Substitui `__CACHE_VERSION__` (só em `sw.js`) pelo hash agregado de
//!      todo o app shell, para o service worker invalidar o cache antigo a
//!      cada build com conteúdo novo.
//!   3. Comprime com gzip (`flate2`, mesmo código-caminho de
//!      `api/http_api.rs::gzip`) — o daemon serve sempre `Content-Encoding:
//!      gzip`, sem custo de CPU em runtime.
//!   4. Grava os bytes finais em `$OUT_DIR/webui/<n>.gz` e emite uma entrada
//!      `Asset { .. }` em `$OUT_DIR/webui_assets.rs` (struct `Asset` definida
//!      em `src/api/web_ui.rs`, que faz `include!` deste arquivo).

use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let webui_dir = Path::new(&manifest_dir).join("webui");
    println!("cargo:rerun-if-changed={}", webui_dir.display());

    let mut rel_paths = Vec::new();
    collect_files(&webui_dir, &webui_dir, &mut rel_paths);
    rel_paths.sort();

    // Hash agregado do CONTEÚDO BRUTO (antes de minificar) de todo o app
    // shell, em ordem canônica — só usado para nomear o cache do service
    // worker (ver sw.js). Determinístico entre builds do mesmo conteúdo.
    let agg_hash = {
        let mut h = FNV_OFFSET;
        for rel in &rel_paths {
            h = fnv1a(h, fs::read(webui_dir.join(rel)).unwrap().as_slice());
        }
        format!("{:016x}", h)[..8].to_string()
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let assets_bin_dir = out_dir.join("webui");
    fs::create_dir_all(&assets_bin_dir).unwrap();

    let mut code = String::from(
        "// Gerado por build.rs a partir de webui/ — não editar à mão.\n&[\n",
    );

    for (i, rel) in rel_paths.iter().enumerate() {
        let raw = fs::read(webui_dir.join(rel)).unwrap();
        let route = route_for(rel);
        let content_type = content_type_for(rel);
        let is_html = rel.ends_with(".html");
        let is_js = rel.ends_with(".js");
        let is_text_asset = is_html || is_js || rel.ends_with(".css") || rel.ends_with(".webmanifest");

        // Ícones (png/svg) são binários — passam direto pro gzip, sem tentar
        // decodificar como UTF-8 nem minificar.
        let bytes = if is_text_asset {
            let text = String::from_utf8(raw)
                .unwrap_or_else(|_| panic!("webui/{rel}: esperado UTF-8 (é um asset de texto)"));
            let processed = if rel == "sw.js" {
                text.replace("__CACHE_VERSION__", &agg_hash)
            } else {
                text
            };
            minify(&processed, is_html).into_bytes()
        } else {
            raw
        };
        let gz = gzip(&bytes);

        let bin_name = format!("{i}.gz");
        fs::write(assets_bin_dir.join(&bin_name), &gz).unwrap();

        let etag = format!("{:016x}", fnv1a(FNV_OFFSET, &bytes));
        // no-cache para o HTML de entrada e o service worker (precisam
        // revalidar sempre, para uma atualização do daemon propagar sem
        // exigir hard-refresh); o resto é imutável (o route/etag muda quando
        // o conteúdo muda).
        let no_cache = is_html || rel == "sw.js";

        code.push_str(&format!(
            "    Asset {{ route: {route:?}, content_type: {content_type:?}, etag: {etag:?}, no_cache: {no_cache}, gz: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/webui/{bin_name}\")) }},\n"
        ));
    }
    code.push_str("]\n");

    fs::write(out_dir.join("webui_assets.rs"), code).unwrap();
}

/// Percorre `dir` recursivamente, coletando caminhos relativos a `root`
/// (sempre com `/`, mesmo no Windows, pois viram rotas HTTP).
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_str().unwrap().replace('\\', "/");
            out.push(rel);
        }
    }
}

fn route_for(rel: &str) -> String {
    if rel == "index.html" {
        "/".to_string()
    } else {
        format!("/{rel}")
    }
}

fn content_type_for(rel: &str) -> &'static str {
    if rel.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if rel.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if rel.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if rel.ends_with(".webmanifest") {
        "application/manifest+json"
    } else if rel.ends_with(".png") {
        "image/png"
    } else if rel.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

/// Minificação conservadora — SÓ remove comentários `<!-- -->` do HTML
/// (delimitadores inequívocos, o parser HTML não os confunde com nada dentro
/// de atributos/URLs). CSS/JS passam intocados: uma primeira versão também
/// removia `/* */` de CSS/JS, mas um comentário de LINHA (`// 1. screens/*.js
/// ...`) contendo a substring "/*" fazia esse scanner (que não entende `//`)
/// consumir até o próximo "*/" de verdade, apagando código real no meio —
/// exatamente o tipo de bug sutil que um minificador "ingênuo" arrisca sem um
/// parser de verdade (strings, regex literals e `//` versus `/*` exigem
/// entender a sintaxe, não só casar delimitadores). Não vale o risco: o gzip
/// (aplicado depois, ver `gzip`) já extrai a esmagadora maioria do ganho de
/// tamanho em JS/CSS/JSON repetitivo.
fn minify(src: &str, is_html: bool) -> String {
    if !is_html {
        return src.to_string();
    }
    // Opera sobre BYTES (não `char`) para não corromper UTF-8 multibyte
    // (acentos, "●"/"▶", "…" — usados à vontade no HTML). "<!--" é ASCII
    // puro, então comparar prefixos em bytes contra um buffer UTF-8 é seguro:
    // um byte < 0x80 nunca é parte de uma sequência multibyte (propriedade do
    // próprio UTF-8).
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"<!--") {
            if let Some(end) = src[i..].find("-->") {
                i += end + 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("minify: entrada UTF-8 válida deve produzir saída UTF-8 válida")
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn fnv1a(mut h: u64, data: &[u8]) -> u64 {
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}
