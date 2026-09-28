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
    pub domain: String,
    pub backends: Vec<String>,
    pub cursor: Arc<AtomicUsize>,
    pub service_id: String,
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

/// Entrada nova para `domain`, reaproveitando o cursor de round-robin da
/// tabela atual para que a posição sobreviva a uma atualização de rota.
fn new_entry(
    table: &RouteTable,
    domain: &str,
    backends: Vec<String>,
    service_id: &str,
) -> RouteEntry {
    let cursor = table
        .get(domain)
        .map(|e| e.cursor.clone())
        .unwrap_or_else(|| Arc::new(AtomicUsize::new(0)));
    RouteEntry {
        domain: domain.to_string(),
        backends,
        cursor,
        service_id: service_id.to_string(),
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
        let entry = new_entry(&new_table, domain, backends, service_id);
        new_table.routes.insert(domain.to_string(), entry);
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
    ///
    /// As rotas deste `service_id` que **não** estão mais no spec são removidas
    /// no mesmo passo: um domínio tirado do serviço sobrevivia na tabela
    /// apontando para o IP de um container que já não existe, e respondia 502
    /// em vez de 404 — indistinguível, para quem olha, de um app quebrado.
    /// Como todo caminho que registra rota passa por aqui (deploy, rolling
    /// update, compose, reconcile de boot e sob demanda), a poda vale para
    /// todos eles.
    pub fn register_domains(&self, spec: &shared::ServiceSpec, ips: &[String], service_id: &str) {
        let routes = spec.domain_routes();
        let old = self.table.load();
        let mut new_table = (**old).clone();

        new_table.routes.retain(|domain, entry| {
            entry.service_id != service_id || routes.iter().any(|r| &r.domain == domain)
        });

        for route in routes {
            let port = route.container_port(spec.port);
            let backends: Vec<String> = ips.iter().map(|ip| format!("{ip}:{port}")).collect();
            let entry = new_entry(&new_table, &route.domain, backends, service_id);
            new_table.routes.insert(route.domain, entry);
        }

        self.table.store(Arc::new(new_table));
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

    /// Foto da tabela de rotas viva, para `Command::IngressRoutes`.
    ///
    /// Ordenada (domínio, depois porta) porque isto é saída de diagnóstico
    /// lida por gente: a ordem de um `HashMap` mudaria a cada chamada.
    pub fn snapshot(&self) -> shared::IngressSnapshot {
        let table = self.table.load();
        let mut domains: Vec<shared::IngressDomainRoute> = table
            .routes
            .values()
            .map(|e| shared::IngressDomainRoute {
                domain: e.domain.clone(),
                backends: e.backends.clone(),
                service_id: e.service_id.clone(),
            })
            .collect();
        domains.sort_by(|a, b| a.domain.cmp(&b.domain));

        let mut ports: Vec<shared::IngressPortRoute> = self
            .port_backends
            .lock()
            .unwrap()
            .iter()
            .map(|(port, backend)| shared::IngressPortRoute {
                host_port: *port,
                // `None` = listener no ar sem destino; vira lista vazia.
                backends: (**backend.load())
                    .as_ref()
                    .map(|b| b.addrs.clone())
                    .unwrap_or_default(),
            })
            .collect();
        ports.sort_by_key(|p| p.host_port);

        shared::IngressSnapshot { domains, ports }
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
            existing.store(Arc::new(Some(PortBackends {
                addrs: backends,
                cursor,
            })));
        } else {
            let port_backend: PortBackend =
                Arc::new(ArcSwap::from_pointee(Some(PortBackends::new(backends))));
            ports.insert(host_port, port_backend.clone());
            tokio::spawn(crate::ingress::proxy::serve_port_proxy(
                host_port,
                port_backend,
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{DomainRoute, Healthcheck, ServiceSource, ServiceSpec};

    fn spec(domains: &[&str]) -> ServiceSpec {
        ServiceSpec {
            name: "app".into(),
            project_id: "prj_1".into(),
            source: ServiceSource::Registry {
                image: "nginx".into(),
            },
            port: 80,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: Default::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: domains
                .iter()
                .map(|d| DomainRoute {
                    domain: (*d).into(),
                    port: None,
                    tls: false,
                })
                .collect(),
            pre_deploy_job_id: None,
            pre_deploy_job_ids: vec![],
        }
    }

    fn domains_of(ing: &IngressController) -> Vec<String> {
        ing.snapshot()
            .domains
            .into_iter()
            .map(|d| d.domain)
            .collect()
    }

    /// O caso que motivou a poda: o domínio sai do spec e a rota antiga fica
    /// apontando para o IP de um container morto, respondendo 502 em vez de 404.
    #[test]
    fn domain_removido_do_spec_perde_a_rota() {
        let ing = IngressController::new();
        ing.register_domains(
            &spec(&["itemize.com.br", "www.itemize.com.br"]),
            &["1.2.3.4".into()],
            "svc_1",
        );
        assert_eq!(
            domains_of(&ing),
            vec!["itemize.com.br", "www.itemize.com.br"]
        );

        ing.register_domains(&spec(&["www.itemize.com.br"]), &["1.2.3.5".into()], "svc_1");
        assert_eq!(domains_of(&ing), vec!["www.itemize.com.br"]);
        assert_eq!(
            ing.snapshot().domains[0].backends,
            vec!["1.2.3.5:80".to_string()]
        );
    }

    /// A poda é por serviço: o domínio de outro serviço não é tocado.
    #[test]
    fn poda_nao_alcanca_outro_servico() {
        let ing = IngressController::new();
        ing.register_domains(&spec(&["a.com"]), &["1.2.3.4".into()], "svc_1");
        ing.register_domains(&spec(&["b.com"]), &["1.2.3.5".into()], "svc_2");

        ing.register_domains(&spec(&["a.com"]), &["1.2.3.6".into()], "svc_1");
        assert_eq!(domains_of(&ing), vec!["a.com", "b.com"]);
    }

    /// Re-registrar não pode reiniciar o round-robin: senão todo deploy manda
    /// as primeiras requisições sempre para a mesma réplica.
    #[test]
    fn cursor_sobrevive_ao_re_registro() {
        let ing = IngressController::new();
        let s = spec(&["a.com"]);
        ing.register_domains(&s, &["1.1.1.1".into(), "2.2.2.2".into()], "svc_1");

        let table = ing.table.load();
        let entry = table.get("a.com").unwrap();
        assert_eq!(entry.next_backend().unwrap(), "1.1.1.1:80");
        drop(table);

        ing.register_domains(&s, &["1.1.1.1".into(), "2.2.2.2".into()], "svc_1");
        let table = ing.table.load();
        assert_eq!(
            table.get("a.com").unwrap().next_backend().unwrap(),
            "2.2.2.2:80"
        );
    }
}
