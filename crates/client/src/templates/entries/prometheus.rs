use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "prometheus",
    name: "Prometheus",
    description: "Central de monitoramento de séries temporais via scraping",
    category: TemplateCategory::Monitoring,
    default_port: 9090,
    compose: r#"
services:
  prometheus:
    image: prom/prometheus:latest
    restart: unless-stopped
    ports:
      - "9090"
    volumes:
      - data:/prometheus
      - config:/etc/prometheus

volumes:
  data:
  config:
"#,
    variables: &[],
};
