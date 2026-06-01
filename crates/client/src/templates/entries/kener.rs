use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kener",
    name: "Kener",
    description: "Página de status open-source moderna para monitoramento",
    category: TemplateCategory::Monitoring,
    default_port: 3000,
    compose: r#"
services:
  kener:
    image: rajnandan1/kener:latest
    restart: unless-stopped
    expose:
      - "3000"
    volumes:
      - data:/app/db

volumes:
  data:
"#,
    variables: &[],
};
