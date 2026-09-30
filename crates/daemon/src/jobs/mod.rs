//! Jobs one-shot (Schedules): execução (`runner`) e agendamento (`scheduler`).

pub mod runner;
pub mod scheduler;

use crate::db::{self, Db};
use chrono::Utc;
use shared::protocol::LogStream;

/// Fecha, como **interrompidas**, as execuções que ficaram sem fim: um daemon que
/// acabou de subir não tem nenhuma execução viva. Cobre as que rodavam e as que
/// ainda esperavam a vez na fila (a fila é só memória e se perde no restart).
/// Ficam com `success = false`, exit code `-1` e uma linha explicando no log; o
/// agendador dispara o job de novo no horário seguinte. Devolve quantas fechou.
/// Roda no boot, **antes** de o agendador subir.
pub async fn recover_interrupted(db: &Db) -> anyhow::Result<usize> {
    let abertas = db::job_run::list_unfinished(db).await?;
    for run in &abertas {
        let _ = db::job_log::append(
            db,
            &run.id,
            &LogStream::Stderr,
            "==> Execução interrompida: o daemon foi reiniciado antes de ela terminar \
             (ou antes de começar, se ainda esperava a vez na fila)",
            Utc::now(),
        )
        .await;
        db::job_run::finish(db, &run.id, INTERRUPTED_EXIT_CODE).await?;
    }
    Ok(abertas.len())
}

/// Exit code de uma execução interrompida por reinício (o mesmo `-1` que o runner
/// grava quando a execução falha antes de terminar).
pub const INTERRUPTED_EXIT_CODE: i32 = -1;

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
        db::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn execucao_sem_fim_vira_interrompida_com_explicacao_no_log() {
        let db = mem_db().await;
        let esperando = db::job_run::create(&db, "j1").await.unwrap();
        let terminada = db::job_run::create(&db, "j2").await.unwrap();
        db::job_run::finish(&db, &terminada.id, 0).await.unwrap();

        assert_eq!(recover_interrupted(&db).await.unwrap(), 1);

        let r = db::job_run::get(&db, &esperando.id).await.unwrap().unwrap();
        assert!(r.finished_at.is_some(), "ganhou um fim");
        assert_eq!(r.exit_code, Some(INTERRUPTED_EXIT_CODE));
        assert_eq!(r.success, Some(false));
        let log = db::job_log::get_for_run(&db, &esperando.id).await.unwrap();
        assert_eq!(log.len(), 1);
        assert!(log[0].line.contains("interrompida"), "{}", log[0].line);

        // A que já tinha terminado não é tocada.
        let t = db::job_run::get(&db, &terminada.id).await.unwrap().unwrap();
        assert_eq!(t.exit_code, Some(0));
        assert_eq!(t.success, Some(true));
        assert!(
            db::job_log::get_for_run(&db, &terminada.id)
                .await
                .unwrap()
                .is_empty()
        );

        // Idempotente: o segundo boot não acha mais nada.
        assert_eq!(recover_interrupted(&db).await.unwrap(), 0);
    }
}
