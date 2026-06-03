use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "typesense",
    name: "Typesense",
    description: "Motor de busca rápida e tolerante a falhas para tempo real",
    category: TemplateCategory::DevTools,
    default_port: 8108,
    compose: r#"
services:
  typesense:
    image: typesense/typesense:latest
    restart: unless-stopped
    expose:
      - "8108"
    environment:
      TYPESENSE_API_KEY: {{TYPESENSE_API_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "TYPESENSE_API_KEY",
        label: "API Key",
        default: None,
        required: true,
        secret: true,
    }],
};
