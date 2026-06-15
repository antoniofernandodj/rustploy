use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "beszel",
    name: "Beszel",
    description: "Monitor leve de servidores com estatísticas de containers",
    category: TemplateCategory::Monitoring,
    default_port: 8090,
    compose: r#"
services:
  beszel:
    image: henrygd/beszel:latest
    restart: unless-stopped
    expose:
      - "8090"
    volumes:
      - data:/beszel_data

volumes:
  data:
"#,
    variables: &[],
};
