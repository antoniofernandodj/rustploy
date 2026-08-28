//! Arquivo de handoff: como um agente na mesma máquina descobre esta API.
//!
//! É o único passo de descoberta. O app grava, ao subir, um JSON com a URL
//! local, o token de acesso e o PID; o agente lê o arquivo e já sabe tudo o que
//! precisa para a primeira requisição (e `GET /agent/schema` conta o resto).
//!
//! Mora no data dir do usuário — o mesmo que a GUI já usa para o `storage` do
//! Luau e a geometria da janela (`shared::fallback_data_dir()`), então não
//! inventa diretório novo nem depende do CWD.
//!
//! **O arquivo é um segredo**: quem lê o token opera o rustploy remoto inteiro.
//! Nasce 0600 no unix. No Windows, onde não há modo POSIX, o diretório de dados
//! do usuário é a fronteira — o que é o mesmo nível de proteção que o resto da
//! persistência local do app já tem.

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

use rand::Rng;

/// Nome do arquivo dentro do data dir.
const FILE: &str = "agent-api.json";

/// Caminho completo do handoff.
pub(crate) fn path() -> PathBuf {
    shared::fallback_data_dir().join(FILE)
}

/// Token de acesso desta execução: 32 bytes de CSPRNG em hex.
///
/// Novo a cada boot de propósito — não há motivo para ele sobreviver ao
/// processo que o serve, e um token efêmero significa que um handoff velho
/// esquecido no disco não vale nada.
pub(crate) fn generate_token() -> String {
    let bytes: [u8; 32] = rand::rng().random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Grava (ou regrava) o handoff. `remote` é o daemon ao qual a janela está
/// conectada agora, ou `None` na tela de login — expor isso poupa do agente uma
/// requisição só para descobrir que ainda não há sessão.
pub(crate) fn write(addr: SocketAddr, token: &str, remote: Option<&str>) -> std::io::Result<()> {
    let doc = serde_json::json!({
        "version": 1,
        "url": format!("http://{addr}"),
        "token": token,
        "pid": std::process::id(),
        "remote_url": remote,
        "connected": remote.is_some(),
        "docs": "GET /agent/schema (Authorization: Bearer <token>)",
        // Escrito por um processo que pode ter morrido sem limpar: o leitor
        // confere o `pid` (ou simplesmente tenta o /agent/health) antes de
        // concluir que a API está no ar.
        "note": "arquivo desta execução do rustploy-gui; confira o pid ou GET /agent/health"
    });

    let destino = path();
    if let Some(dir) = destino.parent() {
        std::fs::create_dir_all(dir)?;
    }

    // Escrita atômica via arquivo temporário + rename: sem isto um agente que
    // leia exatamente durante a regravação (ela acontece a cada login/logout)
    // pode pegar um JSON truncado.
    let temporario = destino.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&temporario)?;
        restrict_permissions(&f)?;
        f.write_all(serde_json::to_string_pretty(&doc).unwrap_or_default().as_bytes())?;
        f.flush()?;
    }
    std::fs::rename(&temporario, &destino)?;
    Ok(())
}

/// Remove o handoff. Chamado ao encerrar o servidor; um handoff órfão não é
/// perigoso (o token morre com o processo), mas confunde quem for ler depois.
pub(crate) fn remove() {
    let _ = std::fs::remove_file(path());
}

#[cfg(unix)]
fn restrict_permissions(f: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_permissions(_f: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_tem_64_hex_e_nao_repete() {
        let a = generate_token();
        let b = generate_token();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
