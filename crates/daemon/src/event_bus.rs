use shared::Event;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 1024;

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);
        Self { sender }
    }

    /// Publica um evento no bus, para os assinantes conectados no momento.
    ///
    /// Sem persistência, de propósito: os eventos eram gravados no SQLite
    /// (`event_log`) para replay ao reconectar, mas isso existia só para o
    /// cliente TUI, removido. O que precisa sobreviver a um restart já tem
    /// tabela própria (deployments, build logs, job runs), e a GUI reconstrói o
    /// estado corrente pelo snapshot do SSE.
    pub fn publish(&self, event: Event) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
