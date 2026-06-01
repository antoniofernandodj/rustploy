use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "flaresolver",
    name: "FlareSolverr",
    description: "Proxy para contornar proteções do Cloudflare",
    category: TemplateCategory::Networking,
    default_port: 8191,
    compose: r#"
services:
  flaresolver:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    restart: unless-stopped
    ports:
      - "8191"
"#,
    variables: &[],
};
