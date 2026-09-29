//! Ingress: proxy reverso HTTP/HTTPS embutido, tabela de rotas e TLS/ACME.

pub mod proxy;
pub mod router;
pub mod tls;

pub use router::IngressController;
pub use tls::TlsManager;
