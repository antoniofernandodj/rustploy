use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "dragonfly",
    name: "Dragonfly",
    description: "Substituto drop-in de alta performance para o Redis",
    category: TemplateCategory::Database,
    default_port: 6379,
    compose: r#"
services:
  dragonfly:
    image: docker.dragonflydb.io/dragonflydb/dragonfly:latest
    restart: unless-stopped
    expose:
      - "6379"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
