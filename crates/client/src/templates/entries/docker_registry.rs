use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "docker-registry",
    name: "Docker Registry",
    description: "Servidor de distribuição oficial para imagens Docker",
    category: TemplateCategory::DevTools,
    default_port: 5000,
    compose: r#"
services:
  docker-registry:
    image: registry:2
    restart: unless-stopped
    expose:
      - "5000"
    volumes:
      - data:/var/lib/registry

volumes:
  data:
"#,
    variables: &[],
};
