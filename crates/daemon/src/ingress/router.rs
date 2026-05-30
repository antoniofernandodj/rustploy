use arc_swap::ArcSwap;
use std::{collections::HashMap, sync::{Arc, Mutex}};

#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub domain: String,
    pub backend_addr: String,
    pub service_id: String,
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

/// Backend atual para um listener de porta específica. None = sem serviço ativo.
pub type PortBackend = Arc<ArcSwap<Option<String>>>;

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

    pub fn upsert_route(&self, domain: &str, backend_addr: &str, service_id: &str) {
        let old = self.table.load();
        let mut new_table = (**old).clone();
        new_table.routes.insert(
            domain.to_string(),
            RouteEntry {
                domain: domain.to_string(),
                backend_addr: backend_addr.to_string(),
                service_id: service_id.to_string(),
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

    pub fn lookup(&self, domain: &str) -> Option<RouteEntry> {
        self.table.load().get(domain).cloned()
    }

    pub fn table_handle(&self) -> RouteHandle {
        self.table.clone()
    }

    /// Aponta `host_port` para `backend_addr` (ex: "172.23.0.2:3000").
    /// Na primeira chamada para essa porta, sobe um listener TCP dedicado.
    pub fn upsert_port_route(&self, host_port: u16, backend_addr: &str) {
        let mut ports = self.port_backends.lock().unwrap();
        if let Some(backend) = ports.get(&host_port) {
            backend.store(Arc::new(Some(backend_addr.to_string())));
        } else {
            let backend: PortBackend =
                Arc::new(ArcSwap::from_pointee(Some(backend_addr.to_string())));
            ports.insert(host_port, backend.clone());
            tokio::spawn(crate::ingress::proxy::serve_port_proxy(host_port, backend));
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
