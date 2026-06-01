use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "meilisearch",
    name: "Meilisearch",
    description: "Motor de busca textual open-source extremamente rápido",
    category: TemplateCategory::DevTools,
    default_port: 7700,
    compose: r#"
services:
  meilisearch:
    image: getmeili/meilisearch:latest
    restart: unless-stopped
    expose:
      - "7700"
    environment:
      MEILI_MASTER_KEY: {{MASTER_KEY}}
    volumes:
      - meili_data:/meili_data

volumes:
  meili_data:
"#,
    variables: &[TemplateVar {
        key: "MASTER_KEY",
        label: "Master Key",
        default: None,
        required: true,
        secret: true,
    }],
};
