use std::time::Duration;
use tracing::{debug, warn};

pub async fn check_http(url: &str, expected: u16, timeout: Duration) -> bool {
    use http_body_util::Empty;
    use hyper::Request;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpStream;

    let without_scheme = url.strip_prefix("http://").unwrap_or(url);
    let (authority, path) = without_scheme
        .split_once('/')
        .map(|(h, p)| (h, format!("/{p}")))
        .unwrap_or((without_scheme, "/".into()));

    let connect = TcpStream::connect(authority);
    let stream = match tokio::time::timeout(timeout, connect).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            warn!(url = %url, error = %e, "check_http: conexão falhou");
            return false;
        }
        Err(_) => {
            warn!(url = %url, "check_http: timeout na conexão");
            return false;
        }
    };

    let io = TokioIo::new(stream);
    let Ok((mut sender, conn)) = hyper::client::conn::http1::handshake(io).await else {
        warn!(url = %url, "check_http: handshake HTTP falhou");
        return false;
    };
    tokio::spawn(async move { let _ = conn.await; });

    let req = match Request::builder()
        .method("GET")
        .uri(path.as_str())
        .header("host", authority)
        .body(Empty::<bytes::Bytes>::new())
    {
        Ok(r) => r,
        Err(e) => {
            warn!(url = %url, error = %e, "check_http: erro ao construir request");
            return false;
        }
    };

    match tokio::time::timeout(timeout, sender.send_request(req)).await {
        Ok(Ok(resp)) => {
            let status = resp.status().as_u16();
            if status != expected {
                warn!(url = %url, got = status, expected = expected, "check_http: status inesperado");
            }
            status == expected
        }
        Ok(Err(e)) => {
            warn!(url = %url, error = %e, "check_http: falhou");
            false
        }
        Err(_) => {
            warn!(url = %url, "check_http: timeout na resposta");
            false
        }
    }
}

pub async fn check_tcp(addr: &str, timeout: Duration) -> bool {
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => {
            debug!(addr = %addr, error = %e, "check_tcp: conexão recusada");
            false
        }
        Err(_) => {
            debug!(addr = %addr, "check_tcp: timeout");
            false
        }
    }
}
