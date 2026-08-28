//! Cliente HTTP para o daemon rustploy remoto.
//!
//! hyper cru (via o `Client` legado do `hyper-util`), não reqwest: é a mesma
//! regra que vale no daemon — um cliente HTTP só no workspace, e ele é o hyper.
//! O que precisamos aqui é pequeno e conhecido: um POST de JSON com bearer.
//!
//! O daemon comprime a resposta do `/api/rpc` com gzip **quando o cliente pede**
//! (`Accept-Encoding: gzip`). Este cliente não pede de propósito: o ganho é de
//! link remoto lento e o custo seria carregar um descompressor aqui para nada.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use hyper::{Method, Request, StatusCode};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;

use super::session::Session;

/// Teto por requisição ao daemon. Um `Command` pesado (o `Snapshot` faz várias
/// idas ao Docker) leva segundos; o que este limite protege é o caso do daemon
/// inalcançável, para o agente receber um erro em vez de pendurar.
const TIMEOUT: Duration = Duration::from_secs(60);

type HttpsClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

/// Falha ao falar com o daemon remoto, já separada no que o agente precisa
/// distinguir: problema de transporte (não chegou lá) × resposta de erro do
/// próprio daemon (chegou e ele recusou).
#[derive(Debug)]
pub(crate) enum RemoteError {
    /// Não deu para falar com o daemon (DNS, TLS, conexão recusada, timeout).
    Transport(String),
    /// O daemon respondeu, mas com status de erro (401 de token vencido, 404…).
    Status { status: u16, body: String },
    /// A resposta chegou mas não é o JSON esperado.
    Decode(String),
}

impl std::fmt::Display for RemoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "falha ao falar com o daemon: {e}"),
            Self::Status { status, body } => {
                write!(f, "daemon respondeu HTTP {status}: {body}")
            }
            Self::Decode(e) => write!(f, "resposta inválida do daemon: {e}"),
        }
    }
}

/// Cliente reutilizável (mantém o pool de conexões vivo entre requisições).
#[derive(Clone)]
pub(crate) struct Remote {
    http: HttpsClient,
}

impl Remote {
    /// Monta o cliente. Falha só se o provider de cripto do rustls não puder ser
    /// configurado, o que na prática não acontece.
    pub(crate) fn new() -> Result<Self, String> {
        // O glacier-ui já instala o provider `ring` como default do processo,
        // mas depender dessa ordem seria frágil: passamos o provider
        // explicitamente, e aí tanto faz quem instalou o quê antes.
        let provider = Arc::new(rustls::crypto::ring::default_provider());

        let tls = hyper_rustls::HttpsConnectorBuilder::new()
            .with_provider_and_webpki_roots(provider)
            .map_err(|e| format!("TLS: {e}"))?
            // `https_or_http`, não `https_only`: um rustploy de laboratório na
            // rede local roda em HTTP puro, e é a GUI que decide isso ao
            // conectar — não cabe a esta ponte recusar o que a janela aceitou.
            .https_or_http()
            .enable_http1()
            .build();

        Ok(Self {
            http: Client::builder(TokioExecutor::new()).build(tls),
        })
    }

    /// Executa um `Command` no daemon: `POST /api/rpc`. Devolve a `Response`
    /// como JSON cru — quem chama decide o que fazer com `{"Err":{…}}`, que é
    /// resposta 200 do ponto de vista HTTP.
    pub(crate) async fn rpc(
        &self,
        session: &Session,
        cmd: &serde_json::Value,
    ) -> Result<serde_json::Value, RemoteError> {
        let corpo = serde_json::to_vec(cmd)
            .map_err(|e| RemoteError::Decode(format!("comando não serializável: {e}")))?;

        let uri = format!("{}/api/rpc", session.base_url);
        let mut req = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json");

        if let Some(token) = &session.token {
            req = req.header(AUTHORIZATION, format!("Bearer {token}"));
        }

        let req = req
            .body(Full::new(Bytes::from(corpo)))
            .map_err(|e| RemoteError::Transport(format!("requisição inválida para {uri}: {e}")))?;

        let resp = tokio::time::timeout(TIMEOUT, self.http.request(req))
            .await
            .map_err(|_| RemoteError::Transport(format!("timeout após {}s", TIMEOUT.as_secs())))?
            .map_err(|e| RemoteError::Transport(e.to_string()))?;

        let status = resp.status();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| RemoteError::Transport(format!("corpo truncado: {e}")))?
            .to_bytes();

        if status != StatusCode::OK {
            return Err(RemoteError::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).chars().take(500).collect(),
            });
        }

        serde_json::from_slice(&bytes).map_err(|e| RemoteError::Decode(e.to_string()))
    }

    /// Sobe um zip para `POST /api/services/<id>/archive`.
    ///
    /// Merece método próprio porque **não é um `Command`**: é rota HTTP com
    /// corpo binário, e um agente que só leu `protocol.rs` não descobre que ela
    /// existe. Era o último caminho do fluxo "criar serviço Archive → deployar"
    /// que a ponte não alcançava.
    pub(crate) async fn upload_archive(
        &self,
        session: &Session,
        service_id: &str,
        filename: &str,
        zip: Vec<u8>,
    ) -> Result<serde_json::Value, RemoteError> {
        let uri = format!("{}/api/services/{}/archive", session.base_url, service_id);

        let mut req = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header(CONTENT_TYPE, "application/zip")
            // O daemon lê o nome original daqui (não há multipart): é o que
            // aparece depois na aba do serviço.
            .header("X-Rustploy-Filename", filename);

        if let Some(token) = &session.token {
            req = req.header(AUTHORIZATION, format!("Bearer {token}"));
        }

        let req = req
            .body(Full::new(Bytes::from(zip)))
            .map_err(|e| RemoteError::Transport(format!("requisição inválida para {uri}: {e}")))?;

        // Sem o timeout curto do `rpc`: um zip de projeto pode levar bem mais
        // que uma chamada de protocolo, e o custo aqui é de rede, não de espera
        // por um daemon travado.
        let resp = tokio::time::timeout(Duration::from_secs(600), self.http.request(req))
            .await
            .map_err(|_| RemoteError::Transport("timeout no upload do zip".into()))?
            .map_err(|e| RemoteError::Transport(e.to_string()))?;

        let status = resp.status();
        let bytes = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| RemoteError::Transport(format!("corpo truncado: {e}")))?
            .to_bytes();

        if status != StatusCode::OK {
            return Err(RemoteError::Status {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).chars().take(500).collect(),
            });
        }

        serde_json::from_slice(&bytes).map_err(|e| RemoteError::Decode(e.to_string()))
    }
}

/// Nome da variante de uma `Response` externamente tagueada (`{"Projects":[…]}`
/// → `"Projects"`), ou da forma nua de variante unitária (`"Ok"` → `"Ok"`).
///
/// O protocolo do rustploy é serde externally-tagged, então "que resposta é
/// essa?" é sempre a única chave do objeto — menos quando a variante não tem
/// campos, que o serde serializa como string pura. Esquecer o segundo caso é o
/// erro clássico de quem escreve cliente para esta API.
pub(crate) fn response_kind(v: &serde_json::Value) -> Option<&str> {
    match v {
        serde_json::Value::String(s) => Some(s.as_str()),
        serde_json::Value::Object(m) if m.len() == 1 => m.keys().next().map(String::as_str),
        _ => None,
    }
}

/// Payload de uma `Response` com campos, ou `None` para variante unitária.
pub(crate) fn response_payload(v: &serde_json::Value) -> Option<&serde_json::Value> {
    v.as_object()
        .filter(|m| m.len() == 1)
        .and_then(|m| m.values().next())
}

/// `Some((code, message))` quando a resposta é `Response::Err`.
pub(crate) fn response_error(v: &serde_json::Value) -> Option<(String, String)> {
    let err = v.get("Err")?;
    Some((
        err.get("code").and_then(|c| c.as_str()).unwrap_or("Err").to_string(),
        err.get("message")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn variante_com_campos_e_reconhecida() {
        let v = json!({ "Projects": [] });
        assert_eq!(response_kind(&v), Some("Projects"));
        assert_eq!(response_payload(&v), Some(&json!([])));
    }

    /// `Response::Ok` vira a string nua `"Ok"`, não um objeto — o caso que
    /// quebra cliente ingênuo.
    #[test]
    fn variante_unitaria_e_string_nua() {
        let v = json!("Ok");
        assert_eq!(response_kind(&v), Some("Ok"));
        assert_eq!(response_payload(&v), None);
    }

    #[test]
    fn erro_do_daemon_e_extraido() {
        let v = json!({ "Err": { "code": "NotFound", "message": "service not found" } });
        let (code, msg) = response_error(&v).unwrap();
        assert_eq!(code, "NotFound");
        assert_eq!(msg, "service not found");
        assert!(response_error(&json!({ "Projects": [] })).is_none());
    }
}
