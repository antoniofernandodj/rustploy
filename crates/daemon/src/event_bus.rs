use crate::db::Db;
use shared::Event;
use std::sync::Arc;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    db: Arc<Db>,
}

impl EventBus {
    pub fn new(db: Arc<Db>) -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { sender, db }
    }

    /// Publica um evento no bus. Tipos persistíveis (DeployStateChanged,
    /// DeployProgress, ServiceStatusChanged) são gravados no SQLite em
    /// background para replay após restart do daemon.
    pub fn publish(&self, event: Event) {
        let is_persistent = matches!(
            &event,
            Event::DeployStateChanged { .. }
                | Event::DeployProgress { .. }
                | Event::ServiceStatusChanged { .. }
        );

        if is_persistent {
            let db = self.db.clone();
            let ev = event.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::db::event_log::append(&db, &ev).await {
                    tracing::warn!(error = %e, "event_log: falha ao persistir evento");
                }
            });
        }

        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}
