use arc_swap::ArcSwap;
use std::{collections::HashMap, sync::Arc};

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

#[derive(Clone)]
pub struct IngressController {
    table: RouteHandle,
}

impl IngressController {
    pub fn new() -> Self {
        Self {
            table: Arc::new(ArcSwap::from_pointee(RouteTable::default())),
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
}

impl Default for IngressController {
    fn default() -> Self {
        Self::new()
    }
}
