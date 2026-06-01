use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "grafana",
    name: "Grafana",
    description: "Dashboards de observabilidade e métricas",
    category: TemplateCategory::Monitoring,
    default_port: 3000,
    compose: r#"
services:
  grafana:
    image: grafana/grafana:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  grafana_data:
"#,
    variables: &[TemplateVar {
        key: "ADMIN_PASSWORD",
        label: "Senha admin",
        default: None,
        required: true,
        secret: true,
    }],
};
