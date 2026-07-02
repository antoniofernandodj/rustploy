use arc_swap::ArcSwap;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub _domain: String,
    pub backends: Vec<String>,
    pub cursor: Arc<AtomicUsize>,
    pub _service_id: String,
}

impl RouteEntry {
    pub fn next_backend(&self) -> Option<String> {
        if self.backends.is_empty() {
            return None;
        }
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.backends.len();
        Some(self.backends[idx].clone())
    }
}

#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    pub routes: HashMap<String, RouteEntry>,
}

impl RouteTable {
    pub fn get(&self, domain: &str) -> Option<&RouteEntry> {
        self.routes.get(domain)
    }
}

/// Shared handle to the live route table, readable lock-free from the proxy thread.
pub type RouteHandle = Arc<ArcSwap<RouteTable>>;

#[derive(Debug, Clone)]
pub struct PortBackends {
    pub addrs: Vec<String>,
    pub cursor: Arc<AtomicUsize>,
}

impl PortBackends {
    pub fn new(addrs: Vec<String>) -> Self {
        Self {
            addrs,
            cursor: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn next(&self) -> Option<String> {
        if self.addrs.is_empty() {
            return None;
        }
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed) % self.addrs.len();
        Some(self.addrs[idx].clone())
    }
}

/// Backend(s) atual para um listener de porta específica. None = sem serviço ativo.
pub type PortBackend = Arc<ArcSwap<Option<PortBackends>>>;

#[derive(Clone)]
pub struct IngressController {
    table: RouteHandle,
    /// port → backend atual. Um listener tokio por porta já iniciada.
    port_backends: Arc<Mutex<HashMap<u16, PortBackend>>>,
}

impl IngressController {
    pub fn new() -> Self {
        Self {
            table: Arc::new(ArcSwap::from_pointee(RouteTable::default())),
            port_backends: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn upsert_route(&self, domain: &str, backends: Vec<String>, service_id: &str) {
        let old = self.table.load();
        let mut new_table = (**old).clone();
        // Reuse cursor so round-robin position survives route updates
        let cursor = old
            .get(domain)
            .map(|e| e.cursor.clone())
            .unwrap_or_else(|| Arc::new(AtomicUsize::new(0)));
        new_table.routes.insert(
            domain.to_string(),
            RouteEntry {
                _domain: domain.to_string(),
                backends,
                cursor,
                _service_id: service_id.to_string(),
            },
        );
        self.table.store(Arc::new(new_table));
    }

    pub fn remove_route(&self, domain: &str) {
        let old = self.table.load();
        let mut new_table = (**old).clone();
        new_table.routes.remove(domain);
        self.table.store(Arc::new(new_table));
    }

    /// Registra todas as rotas HTTP de domínio de um serviço a partir dos IPs
    /// dos containers live. Cada domínio é roteado para a sua porta de container
    /// (own `port`, ou a `port` padrão do serviço) — é isto que permite um
    /// serviço em várias portas expor um subdomínio por porta.
    pub fn register_domains(&self, spec: &shared::ServiceSpec, ips: &[String], service_id: &str) {
        for route in spec.domain_routes() {
            let port = route.container_port(spec.port);
            let backends: Vec<String> = ips.iter().map(|ip| format!("{ip}:{port}")).collect();
            self.upsert_route(&route.domain, backends, service_id);
        }
    }

    /// Remove todas as rotas de domínio do serviço (parada/remoção/reconcile).
    pub fn remove_domains(&self, spec: &shared::ServiceSpec) {
        for route in spec.domain_routes() {
            self.remove_route(&route.domain);
        }
    }

    pub fn _lookup(&self, domain: &str) -> Option<RouteEntry> {
        self.table.load().get(domain).cloned()
    }

    pub fn table_handle(&self) -> RouteHandle {
        self.table.clone()
    }

    /// Aponta `host_port` para os `backends` fornecidos.
    /// Na primeira chamada para essa porta, sobe um listener TCP dedicado.
    pub fn upsert_port_route(&self, host_port: u16, backends: Vec<String>) {
        let mut ports = self.port_backends.lock().unwrap();
        if let Some(existing) = ports.get(&host_port) {
            // Reuse cursor to maintain round-robin continuity across redeploys
            let cursor = (**existing.load())
                .as_ref()
                .map(|b| b.cursor.clone())
                .unwrap_or_else(|| Arc::new(AtomicUsize::new(0)));
            existing.store(Arc::new(Some(PortBackends { addrs: backends, cursor })));
        } else {
            let port_backend: PortBackend =
                Arc::new(ArcSwap::from_pointee(Some(PortBackends::new(backends))));
            ports.insert(host_port, port_backend.clone());
            tokio::spawn(crate::ingress::proxy::serve_port_proxy(host_port, port_backend));
        }
    }

    /// Remove o roteamento de `host_port` (conexões novas são recusadas com reset).
    pub fn remove_port_route(&self, host_port: u16) {
        if let Some(backend) = self.port_backends.lock().unwrap().get(&host_port) {
            backend.store(Arc::new(None));
        }
    }
}

impl Default for IngressController {
    fn default() -> Self {
        Self::new()
    }
}
