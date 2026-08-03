//! Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa
//! ao client iced (`rustploy-gui`), que estava sendo bloqueado pelo Windows
//! Defender. Fala com o daemon pelo MESMO protocolo HTTP/JSON + SSE que o
//! client iced já usa (`POST /api/rpc`, `GET /api/events`) — este módulo só
//! entrega o app shell (HTML/CSS/JS/ícones), pré-minificado e pré-gzipado em
//! tempo de build (ver `../../build.rs`), sem I/O nem CPU extra em runtime.
//!
//! Roteamento por caminho fixo, plugado em `http_api.rs::handle` ANTES do
//! gate de Bearer token — mesmo tratamento das rotas públicas de webhook/
//! OAuth, porque o usuário ainda não tem o token quando a página carrega (ele
//! loga *depois* de abrir). Isso é seguro: só entrega estático, sem dado
//! nenhum — todo acesso a dado de verdade continua exigindo o Bearer em
//! `/api/rpc` e `/api/events`, como já fazia o client iced.

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{Response, StatusCode};

type ApiBody = BoxBody<Bytes, std::convert::Infallible>;

/// Um arquivo do app shell, já processado (minificado + gzipado) em tempo de
/// build — ver `Asset` gerado por `build.rs` em `$OUT_DIR/webui_assets.rs`.
pub struct Asset {
    pub route: &'static str,
    pub content_type: &'static str,
    /// Hash de conteúdo (FNV-1a hex), usado como `ETag`.
    pub etag: &'static str,
    /// `true` para o HTML de entrada e o service worker: precisam revalidar
    /// sempre, para uma atualização do daemon propagar sem exigir
    /// hard-refresh. Os demais assets são imutáveis (o conteúdo só muda
    /// quando o build muda, e nesse caso são simplesmente outros bytes na
    /// mesma rota — o `ETag` cobre esse caso).
    pub no_cache: bool,
    /// Bytes já comprimidos com gzip.
    pub gz: &'static [u8],
}

static ASSETS: &[Asset] = include!(concat!(env!("OUT_DIR"), "/webui_assets.rs"));

/// Serve `path` se casar com algum asset embutido do app shell; `None` se a
/// rota não pertencer à web UI (o chamador cai no 404 padrão da API).
pub fn serve(path: &str) -> Option<Response<ApiBody>> {
    let asset = ASSETS.iter().find(|a| a.route == path)?;
    let cache_control = if asset.no_cache {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(hyper::header::CONTENT_TYPE, asset.content_type)
            .header(hyper::header::CONTENT_ENCODING, "gzip")
            .header(hyper::header::CACHE_CONTROL, cache_control)
            .header(hyper::header::ETAG, format!("\"{}\"", asset.etag))
            .body(Full::new(Bytes::from_static(asset.gz)).boxed())
            .expect("resposta estática bem-formada"),
    )
}
