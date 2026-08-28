//! Os servidores que o usuário já usou nesta máquina.
//!
//! A camada Luau salva todo login bem-sucedido no `storage` do glacier-ui — um
//! JSON no data dir do usuário (`handlers/connection.luau`, `remember_server`).
//! É de lá que o formulário de login nasce preenchido.
//!
//! Para a API de agente isso resolve um problema concreto: conectar sem
//! precisar do token. O agente pede a URL, a ponte encontra o token salvo e
//! preenche o formulário — o segredo nunca atravessa a rede em nenhum sentido,
//! nem na ida (o agente não o manda) nem na volta (a listagem só diz se existe).

use std::path::PathBuf;

use serde_json::Value;

/// Um servidor conhecido.
pub(super) struct Saved {
    pub url: String,
    pub token: Option<String>,
}

/// Arquivo do `storage` do glacier para a janela principal.
fn storage_path() -> PathBuf {
    shared::fallback_data_dir()
        .join(".glacier-storage")
        .join("app.json")
}

/// Lê a lista salva. Vazia quando o arquivo não existe (nenhum login ainda),
/// está corrompido ou mudou de formato — nunca é erro fatal: a consequência é
/// só o agente ter de informar o token, que é o caminho normal mesmo.
pub(super) fn list() -> Vec<Saved> {
    let Ok(texto) = std::fs::read_to_string(storage_path()) else {
        return Vec::new();
    };
    let Ok(doc) = serde_json::from_str::<Value>(&texto) else {
        return Vec::new();
    };

    doc.get("saved_servers")
        .and_then(Value::as_array)
        .map(|itens| {
            itens
                .iter()
                .filter_map(|s| {
                    let url = s.get("url")?.as_str()?.trim();
                    if url.is_empty() {
                        return None;
                    }
                    Some(Saved {
                        url: url.to_string(),
                        token: s
                            .get("token")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|t| !t.is_empty())
                            .map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Token salvo para uma URL, comparando de forma tolerante à barra final — a
/// mesma normalização que a sessão faz, para "https://x.dev/" e "https://x.dev"
/// não virarem servidores diferentes.
pub(super) fn token_for(url: &str) -> Option<String> {
    let alvo = url.trim().trim_end_matches('/');
    list()
        .into_iter()
        .find(|s| s.url.trim_end_matches('/') == alvo)
        .and_then(|s| s.token)
}

/// A listagem que sai pela API: URL e se há token guardado — nunca o token.
pub(super) fn as_json() -> Value {
    let itens: Vec<Value> = list()
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "url": s.url,
                "has_saved_token": s.token.is_some(),
            })
        })
        .collect();

    serde_json::json!({ "count": itens.len(), "servers": itens })
}
