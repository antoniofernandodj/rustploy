//! Fila **global** de deploys: no máximo um deploy rodando por vez no daemon.
//!
//! Antes, cada `deploy_start` spawnava um `DeployExecutor` na hora e N pedidos
//! rodavam concorrentes. Agora `deploy_start` só **enfileira** o `deployment_id`
//! (o deployment nasce em [`DeployState::Pending`] e o serviço em
//! [`ServiceStatus::Queued`]) e um **único worker** ([`run_worker`]) puxa um por
//! vez, roda até terminar e só então pega o próximo.
//!
//! A ordem "verdadeira" da fila vive na `VecDeque` em memória (não no banco);
//! num restart ela é reconstruída da ordem de criação dos `Pending`
//! (`recovery::recover`). O worker roda o executor como *task* (não `await`
//! inline) para preservar o mecanismo de abort via `active_deploys`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use shared::{Event, ServiceStatus};
use tokio::sync::Notify;
use tracing::{info, warn};

use crate::api::AppState;
use crate::deploy::executor::DeployExecutor;

#[derive(Default)]
struct QueueInner {
    /// deployment_ids esperando, em ordem de execução (frente = próximo).
    queued: VecDeque<String>,
    /// deployment_id rodando agora (ou `None`).
    running: Option<String>,
    /// Fila pausada — o worker não puxa o próximo até retomar.
    paused: bool,
}

/// Handle compartilhado da fila (fica no `AppState`). Sem `AppState` dentro,
/// para não criar ciclo de tipos: o worker recebe o `AppState` por fora.
pub struct DeployQueue {
    inner: Mutex<QueueInner>,
    notify: Notify,
}

impl DeployQueue {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(QueueInner::default()),
            notify: Notify::new(),
        })
    }

    /// Enfileira um deployment e acorda o worker. Idempotente: ignora se o id já
    /// está na fila ou rodando.
    pub fn enqueue(&self, dep_id: String) {
        {
            let mut g = self.inner.lock().unwrap();
            if g.running.as_deref() == Some(dep_id.as_str()) || g.queued.contains(&dep_id) {
                return;
            }
            g.queued.push_back(dep_id);
        }
        self.notify.notify_one();
    }

    /// Remove um deployment **enfileirado** (não afeta o que está rodando —
    /// abortar o running é papel do `active_deploys`). Retorna `true` se removeu.
    pub fn remove_queued(&self, dep_id: &str) -> bool {
        let mut g = self.inner.lock().unwrap();
        if let Some(pos) = g.queued.iter().position(|d| d == dep_id) {
            g.queued.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move um enfileirado para o início da fila ("furar fila").
    pub fn promote(&self, dep_id: &str) {
        let mut g = self.inner.lock().unwrap();
        if let Some(pos) = g.queued.iter().position(|d| d == dep_id) {
            if let Some(d) = g.queued.remove(pos) {
                g.queued.push_front(d);
            }
        }
    }

    /// Reordena a fila para a ordem dada. Ids desconhecidos são ignorados;
    /// enfileirados não citados vão ao fim, preservando a ordem relativa.
    pub fn reorder(&self, order: &[String]) {
        let mut g = self.inner.lock().unwrap();
        let mut remaining: VecDeque<String> = std::mem::take(&mut g.queued);
        let mut next = VecDeque::with_capacity(remaining.len());
        for id in order {
            if let Some(pos) = remaining.iter().position(|d| d == id) {
                if let Some(d) = remaining.remove(pos) {
                    next.push_back(d);
                }
            }
        }
        next.extend(remaining);
        g.queued = next;
    }

    /// Pausa/retoma a fila. Ao retomar, acorda o worker.
    pub fn set_paused(&self, paused: bool) {
        {
            let mut g = self.inner.lock().unwrap();
            g.paused = paused;
        }
        if !paused {
            self.notify.notify_one();
        }
    }

    /// Snapshot para o handler de status: `(running, queued_em_ordem, paused)`.
    pub fn snapshot(&self) -> (Option<String>, Vec<String>, bool) {
        let g = self.inner.lock().unwrap();
        (
            g.running.clone(),
            g.queued.iter().cloned().collect(),
            g.paused,
        )
    }

    /// Tira o próximo da fila e marca como running. `None` se vazia OU pausada.
    fn take_next(&self) -> Option<String> {
        let mut g = self.inner.lock().unwrap();
        if g.paused {
            return None;
        }
        let next = g.queued.pop_front();
        if let Some(id) = &next {
            g.running = Some(id.clone());
        }
        next
    }

    fn clear_running(&self) {
        self.inner.lock().unwrap().running = None;
    }

    async fn wait(&self) {
        self.notify.notified().await;
    }
}

/// Worker único da fila global. Spawnado uma vez no startup. Puxa um deploy por
/// vez, roda até terminar (ou ser abortado) e só então pega o próximo.
pub async fn run_worker(state: AppState) {
    let queue = state.deploy_queue.clone();
    info!("deploy queue worker iniciado");
    loop {
        // Espera até ter algo para rodar e a fila não estar pausada. `Notify`
        // guarda um permit se `notify_one` chegar antes do `wait` — sem wakeup
        // perdido entre `take_next()` e `wait()`.
        let dep_id = loop {
            if let Some(id) = queue.take_next() {
                break id;
            }
            queue.wait().await;
        };

        // running mudou → avisa a GUI.
        state.bus.publish(Event::DeployQueueChanged);
        run_one(&state, &dep_id).await;
        queue.clear_running();
        state.bus.publish(Event::DeployQueueChanged);
    }
}

/// Roda um deployment: marca o serviço como `Deploying`, spawna o executor como
/// task (guardando o `AbortHandle` em `active_deploys` para o `deploy_abort`) e
/// aguarda terminar.
async fn run_one(state: &AppState, dep_id: &str) {
    // Marca o serviço como Deploying (deploy_start deixou em Queued).
    if let Ok(Some(dep)) = crate::db::deployments::get(&state.db, dep_id).await {
        let _ = crate::db::services::update_status(
            &state.db,
            &dep.service_id,
            &ServiceStatus::Deploying,
            None,
        )
        .await;
        state.bus.publish(Event::ServiceStatusChanged {
            service_id: dep.service_id.clone(),
            status: ServiceStatus::Deploying,
        });
    } else {
        warn!(deployment_id = %dep_id, "deploy queue: deployment sumiu antes de rodar");
        return;
    }

    let executor = Arc::new(DeployExecutor {
        db: state.db.clone(),
        docker: state.docker.clone(),
        ingress: state.ingress.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        tls: state.tls.clone(),
        db_path: state.db_path.clone(),
        drain_secs: state.drain_secs,
        registry_internal_token: state.registry_internal_token.clone(),
    });

    let dep_owned = dep_id.to_string();
    let handle = tokio::spawn(async move { executor.run(dep_owned).await });
    if let Ok(mut map) = state.active_deploys.lock() {
        map.insert(dep_id.to_string(), handle.abort_handle());
    }
    // Aguarda concluir (ou ser abortado via active_deploys → JoinError).
    let _ = handle.await;
    if let Ok(mut map) = state.active_deploys.lock() {
        map.remove(dep_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queued(q: &DeployQueue) -> Vec<String> {
        q.snapshot().1
    }

    #[test]
    fn enqueue_orders_fifo_and_dedups() {
        let q = DeployQueue::new();
        q.enqueue("a".into());
        q.enqueue("b".into());
        q.enqueue("a".into()); // duplicata: ignorada
        assert_eq!(queued(&q), vec!["a", "b"]);
    }

    #[test]
    fn promote_moves_to_front() {
        let q = DeployQueue::new();
        for id in ["a", "b", "c"] {
            q.enqueue(id.into());
        }
        q.promote("c");
        assert_eq!(queued(&q), vec!["c", "a", "b"]);
        // promover um id inexistente é no-op.
        q.promote("zzz");
        assert_eq!(queued(&q), vec!["c", "a", "b"]);
    }

    #[test]
    fn reorder_applies_and_appends_omitted() {
        let q = DeployQueue::new();
        for id in ["a", "b", "c"] {
            q.enqueue(id.into());
        }
        // "b" omitido e "zzz" desconhecido: b vai ao fim, zzz ignorado.
        q.reorder(&["c".into(), "zzz".into(), "a".into()]);
        assert_eq!(queued(&q), vec!["c", "a", "b"]);
    }

    #[test]
    fn remove_queued_reports_and_removes() {
        let q = DeployQueue::new();
        q.enqueue("a".into());
        q.enqueue("b".into());
        assert!(q.remove_queued("a"));
        assert!(!q.remove_queued("nope"));
        assert_eq!(queued(&q), vec!["b"]);
    }

    #[test]
    fn take_next_respects_pause_and_marks_running() {
        let q = DeployQueue::new();
        q.enqueue("a".into());
        q.set_paused(true);
        assert!(q.take_next().is_none(), "pausada não entrega");
        q.set_paused(false);
        assert_eq!(q.take_next().as_deref(), Some("a"));
        let (running, queued_now, paused) = q.snapshot();
        assert_eq!(running.as_deref(), Some("a"));
        assert!(queued_now.is_empty());
        assert!(!paused);
        q.clear_running();
        assert!(q.snapshot().0.is_none());
    }
}
