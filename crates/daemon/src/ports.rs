//! Alocação automática de portas externas (`ServiceSpec.host_port`).
//!
//! `host_port = Some(0)` é a sentinela "aloque para mim": aqui ela é trocada
//! por uma porta livre da faixa `[external_ports]` da config, persistida no
//! spec (redeploys/restarts reusam a mesma porta para sempre). Portas manuais
//! passam por validação de duplicata contra os demais serviços.

use crate::db::Db;
use shared::config::RustployConfig;
use shared::ServiceSpec;

/// Sentinela em `ServiceSpec.host_port` que pede alocação automática.
pub const AUTO_PORT: u16 = 0;

/// Resolve o `host_port` do spec antes de persistir:
/// - `Some(0)` → aloca uma porta livre da faixa configurada.
/// - `Some(p)` manual → erro se outro serviço (≠ `exclude_id`) já a reserva.
/// - `None` → nada a fazer.
pub async fn resolve_host_port(
    db: &Db,
    spec: &mut ServiceSpec,
    exclude_id: Option<&str>,
) -> Result<(), String> {
    let Some(requested) = spec.host_port else {
        return Ok(());
    };

    let used = used_ports(db, exclude_id).await?;

    if requested == AUTO_PORT {
        spec.host_port = Some(allocate(&used)?);
        return Ok(());
    }

    if used.contains(&requested) {
        return Err(format!(
            "porta externa {requested} já está reservada por outro serviço ou pelo próprio rustploy"
        ));
    }
    Ok(())
}

/// True se `port` ainda é reservada por algum serviço ≠ `exclude_id` (ou pelo
/// próprio daemon) — usado antes de um `deny` no firewall, para nunca fechar
/// uma porta que outro serviço compartilha.
pub async fn port_in_use_by_other(db: &Db, port: u16, exclude_id: Option<&str>) -> bool {
    match used_ports(db, exclude_id).await {
        Ok(used) => used.contains(&port),
        // Na dúvida (falha de DB), não mexe no firewall.
        Err(_) => true,
    }
}

/// Portas de host indisponíveis: `host_port` de todos os serviços (exceto
/// `exclude_id`) + portas do próprio daemon (ingress, API, webhook).
async fn used_ports(db: &Db, exclude_id: Option<&str>) -> Result<Vec<u16>, String> {
    let cfg = RustployConfig::global();
    let mut used = vec![
        cfg.ingress.http_port,
        cfg.ingress.https_port,
        cfg.api.port,
        cfg.daemon.webhook_port,
    ];
    let services = crate::db::services::list_all(db)
        .await
        .map_err(|e| format!("falha ao listar serviços para alocação de porta: {e}"))?;
    for s in services {
        if Some(s.id.as_str()) == exclude_id {
            continue;
        }
        if let Some(p) = s.spec.host_port {
            if p != AUTO_PORT {
                used.push(p);
            }
        }
    }
    Ok(used)
}

/// Varre a faixa configurada e devolve a primeira porta que (a) nenhum serviço
/// reserva e (b) nenhum processo do SO ocupa (sondagem via bind descartado na
/// hora — cobre colisão com software alheio ao rustploy).
fn allocate(used: &[u16]) -> Result<u16, String> {
    let range = &RustployConfig::global().external_ports;
    for port in range.range_start..=range.range_end {
        if used.contains(&port) {
            continue;
        }
        if std::net::TcpListener::bind(("0.0.0.0", port)).is_err() {
            continue;
        }
        return Ok(port);
    }
    Err(format!(
        "faixa de portas externas esgotada ({}-{}); ajuste [external_ports] na config",
        range.range_start, range.range_end
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_pula_portas_reservadas() {
        let range = &RustployConfig::global().external_ports;
        let first = range.range_start;
        let port = allocate(&[first, first + 1]).expect("deve alocar");
        assert!(port > first + 1);
        assert!(range.contains(port));
    }

    #[test]
    fn allocate_pula_porta_ocupada_no_so() {
        let range = &RustployConfig::global().external_ports;
        // Ocupa a primeira porta da faixa no SO e confere que a sondagem a pula.
        let _guard = std::net::TcpListener::bind(("0.0.0.0", range.range_start))
            .expect("primeira porta da faixa livre no ambiente de teste");
        let port = allocate(&[]).expect("deve alocar");
        assert_ne!(port, range.range_start);
    }
}
