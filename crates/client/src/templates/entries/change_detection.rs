use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "change-detection",
    name: "changedetection.io",
    description: "Monitor inteligente para alterações em páginas web",
    category: TemplateCategory::Monitoring,
    default_port: 5000,
    compose: r#"
services:
  change-detection:
    image: ghcr.io/dgtlmoon/changedetection.io:latest
    restart: unless-stopped
    expose:
      - "5000"
    volumes:
      - datastore:/datastore

volumes:
  datastore:
"#,
    variables: &[],
};
