pub mod dokploy;

use shared::models::{Project, Service};

pub struct TransformedData {
    pub projects: Vec<Project>,
    pub services: Vec<Service>,
}
