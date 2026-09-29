//! Conversão dos dados de origem para os modelos do rustploy
//! (`TransformedData`).

pub mod dokploy;

use shared::models::{Project, Service};

pub struct TransformedData {
    pub projects: Vec<Project>,
    pub services: Vec<Service>,
}
